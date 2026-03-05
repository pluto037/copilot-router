use anyhow::Result;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use bytes::Bytes;
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

use crate::state::AppState;
use crate::usage::tracker::{insert_log, UsageRecord};

pub type SharedState = Arc<Mutex<AppState>>;

const COPILOT_API_URL: &str = "https://api.githubcopilot.com/chat/completions";
const COPILOT_MODELS_URL_CANDIDATES: [&str; 2] = [
    "https://api.githubcopilot.com/models",
    "https://api.githubcopilot.com/v1/models",
];
const FALLBACK_MODEL_IDS: [&str; 24] = [
    "claude-haiku-4-5",
    "claude-opus-4-5",
    "claude-opus-4-6",
    "claude-sonnet-4",
    "claude-sonnet-4-5",
    "claude-sonnet-4-6",
    "gemini-2.5-pro",
    "gemini-3-flash-preview",
    "gemini-3-pro-preview",
    "gemini-3.1-pro-preview",
    "gpt-4.1",
    "gpt-4o",
    "gpt-5-mini",
    "gpt-5.1",
    "gpt-5.1-codex",
    "gpt-5.1-codex-max",
    "gpt-5.1-codex-mini-preview",
    "gpt-5.2",
    "gpt-5.2-codex",
    "gpt-5.3-codex",
    "grok-code-fast-1",
    "gpt-4o",
    "gpt-4o-mini",
    "o3-mini",
];

/// Start the local proxy server.
pub async fn start(state: SharedState, port: u16) -> Result<()> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // OpenAI-compatible endpoint
        .route("/v1/chat/completions", post(handle_openai))
        // Anthropic-compatible endpoint
        .route("/v1/messages", post(handle_anthropic))
        // Model list endpoint
        .route("/v1/models", axum::routing::get(handle_models))
        .layer(cors)
        .with_state(state.clone());

    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Proxy server listening on http://{}", addr);

    // Mark proxy as running
    {
        let mut s = state.lock().await;
        s.proxy_running = true;
        s.proxy_port = port;
    }

    axum::serve(listener, app).await?;
    Ok(())
}

/// Handle OpenAI-format requests: POST /v1/chat/completions
async fn handle_openai(
    State(state): State<SharedState>,
    _headers: HeaderMap,
    body: Bytes,
) -> Response {
    let start = std::time::Instant::now();
    let path = "/v1/chat/completions".to_string();

    let proxy_enabled = {
        let s = state.lock().await;
        s.config.proxy_enabled
    };
    if !proxy_enabled {
        return error_response(503, "Proxy is disabled in settings. Please enable proxy routing first.");
    }

    // Parse request body
    let mut req_body: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return error_response(400, &format!("Invalid JSON: {}", e));
        }
    };

    // Extract model
    let requested_model = req_body
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-4o")
        .to_string();

    // Get Copilot token and resolve model
    let (copilot_token, resolved_model) = {
        let s = state.lock().await;
        let token = match &s.copilot_token {
            Some(t) if s.is_token_valid() => t.token.clone(),
            _ => {
                return error_response(401, "Copilot token not available. Please configure authentication.");
            }
        };
        let model = s.resolve_model(&requested_model);
        (token, model)
    };

    // Rewrite model in request
    if let Some(obj) = req_body.as_object_mut() {
        obj.insert("model".to_string(), Value::String(resolved_model.clone()));
    }

    // Forward to Copilot API
    let client = match build_copilot_client() {
        Ok(c) => c,
        Err(e) => return error_response(500, &e.to_string()),
    };

    let is_stream = req_body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let upstream = match send_copilot_request(&client, &copilot_token, &req_body).await {
        Ok(r) => r,
        Err(e) => {
            log_request(
                &state,
                UsageRecord {
                    timestamp: Utc::now(),
                    requested_model: requested_model.clone(),
                    mapped_model: resolved_model.clone(),
                    model: resolved_model,
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                    status_code: 502,
                    latency_ms: start.elapsed().as_millis() as i64,
                    path: path.clone(),
                    error: Some(e.to_string()),
                },
            )
            .await;
            return error_response(502, &format!("Upstream error: {}", e));
        }
    };

    let mut used_model = resolved_model.clone();
    let mut status = upstream.status();
    let mut response_body = match upstream.bytes().await {
        Ok(b) => b,
        Err(e) => return error_response(502, &e.to_string()),
    };

    if status == StatusCode::BAD_REQUEST
        && used_model != "gpt-4o"
        && is_model_not_supported(&response_body)
    {
        tracing::warn!(
            "Model '{}' not supported upstream, retrying once with fallback model 'gpt-4o'",
            used_model
        );

        let mut retry_body = req_body.clone();
        if let Some(obj) = retry_body.as_object_mut() {
            obj.insert("model".to_string(), Value::String("gpt-4o".to_string()));
        }

        let retry_upstream = match send_copilot_request(&client, &copilot_token, &retry_body).await {
            Ok(r) => r,
            Err(e) => {
                log_request(
                    &state,
                    UsageRecord {
                        timestamp: Utc::now(),
                        requested_model: requested_model.clone(),
                        mapped_model: "gpt-4o".to_string(),
                        model: "gpt-4o".to_string(),
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                        status_code: 502,
                        latency_ms: start.elapsed().as_millis() as i64,
                        path: path.clone(),
                        error: Some(e.to_string()),
                    },
                )
                .await;
                return error_response(502, &format!("Upstream retry error: {}", e));
            }
        };

        used_model = "gpt-4o".to_string();
        status = retry_upstream.status();
        response_body = match retry_upstream.bytes().await {
            Ok(b) => b,
            Err(e) => return error_response(502, &e.to_string()),
        };
    }

    let latency_ms = start.elapsed().as_millis() as i64;

    if is_stream {
        // Stream the SSE response back directly
        log_request(
            &state,
            UsageRecord {
                timestamp: Utc::now(),
                requested_model: requested_model.clone(),
                mapped_model: used_model.clone(),
                model: used_model,
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                status_code: status.as_u16() as i64,
                latency_ms,
                path: path.clone(),
                error: None,
            },
        )
        .await;

        axum::response::Response::builder()
            .status(status)
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(axum::body::Body::from(response_body))
            .unwrap_or_else(|_| error_response(500, "Response build failed"))
    } else {
        // Extract token usage for logging
        let (prompt_tokens, completion_tokens) = extract_usage(&response_body);

        log_request(
            &state,
            UsageRecord {
                timestamp: Utc::now(),
                requested_model: requested_model,
                mapped_model: used_model.clone(),
                model: used_model,
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
                status_code: status.as_u16() as i64,
                latency_ms,
                path,
                error: if status.is_success() { None } else { Some("Upstream error".to_string()) },
            },
        )
        .await;

        axum::response::Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(response_body))
            .unwrap_or_else(|_| error_response(500, "Response build failed"))
    }
}

/// Handle Anthropic-format requests: POST /v1/messages
async fn handle_anthropic(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let openai_body = match super::anthropic::to_openai_request(&body) {
        Ok(b) => b,
        Err(e) => return error_response(400, &format!("Format conversion error: {}", e)),
    };

    // Reuse OpenAI handler
    let openai_bytes = match serde_json::to_vec(&openai_body) {
        Ok(b) => Bytes::from(b),
        Err(e) => return error_response(500, &e.to_string()),
    };

    let openai_response = handle_openai(State(state), headers, openai_bytes).await;

    // If it's a non-stream response, convert back to Anthropic format
    let (parts, body_bytes) = match axum::response::Response::into_parts(openai_response) {
        (p, b) => {
            use http_body_util::BodyExt;
            let bytes = match b.collect().await {
                Ok(c) => c.to_bytes(),
                Err(_) => return error_response(500, "Failed to read response body"),
            };
            (p, bytes)
        }
    };

    if !parts.status.is_success() {
        return axum::response::Response::from_parts(
            parts,
            axum::body::Body::from(body_bytes),
        );
    }

    let content_type = parts
        .headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.contains("text/event-stream") {
        // SSE stream: convert each chunk
        let converted = super::anthropic::convert_stream_to_anthropic(&body_bytes);
        axum::response::Response::builder()
            .status(200)
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .body(axum::body::Body::from(converted))
            .unwrap_or_else(|_| error_response(500, "Response build failed"))
    } else {
        // Regular JSON: convert to Anthropic format
        match super::anthropic::to_anthropic_response(&body_bytes) {
            Ok(anthropic_body) => Json(anthropic_body).into_response(),
            Err(e) => error_response(500, &format!("Response conversion error: {}", e)),
        }
    }
}

/// List available models
async fn handle_models(State(state): State<SharedState>) -> Response {
    let (copilot_token, mappings) = {
        let s = state.lock().await;
        (
            s.copilot_token
                .as_ref()
                .filter(|_| s.is_token_valid())
                .map(|t| t.token.clone()),
            s.config.model_mappings.clone(),
        )
    };

    let mut ids: Vec<String> = Vec::new();

    for id in FALLBACK_MODEL_IDS {
        push_unique_model_id(&mut ids, id.to_string());
    }

    for mapping in mappings {
        if !mapping.from_model.trim().is_empty() {
            push_unique_model_id(&mut ids, mapping.from_model.trim().to_string());
        }
        if !mapping.to_model.trim().is_empty() {
            push_unique_model_id(&mut ids, mapping.to_model.trim().to_string());
        }
    }

    if let Some(token) = copilot_token {
        match fetch_remote_models(&token).await {
            Ok(remote_ids) => {
                for id in remote_ids {
                    push_unique_model_id(&mut ids, id);
                }
            }
            Err(e) => {
                tracing::debug!("Failed to fetch remote model list: {}", e);
            }
        }
    }

    let models = serde_json::json!({
        "object": "list",
        "data": ids
            .into_iter()
            .map(|id| {
                serde_json::json!({
                    "id": id,
                    "object": "model",
                    "created": 1706745938,
                    "owned_by": "copilot-router"
                })
            })
            .collect::<Vec<Value>>()
    });

    Json(models).into_response()
}

fn push_unique_model_id(ids: &mut Vec<String>, id: String) {
    if ids.iter().any(|existing| existing == &id) {
        return;
    }
    ids.push(id);
}

async fn fetch_remote_models(copilot_token: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .user_agent("GitHubCopilotChat/0.20.3")
        .timeout(std::time::Duration::from_secs(20))
        .build()?;

    for url in COPILOT_MODELS_URL_CANDIDATES {
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", copilot_token))
            .header("Accept", "application/json")
            .send()
            .await;

        let response = match response {
            Ok(r) => r,
            Err(_) => continue,
        };

        if !response.status().is_success() {
            continue;
        }

        let payload = match response.json::<Value>().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(ids) = extract_model_ids(&payload) {
            return Ok(ids);
        }
    }

    anyhow::bail!("No usable remote model endpoint response")
}

fn extract_model_ids(payload: &Value) -> Option<Vec<String>> {
    let data_array = payload
        .get("data")
        .and_then(|v| v.as_array())
        .or_else(|| payload.get("models").and_then(|v| v.as_array()))
        .or_else(|| payload.as_array());

    let entries = data_array?;
    let mut ids = Vec::new();
    for item in entries {
        if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
            if !id.trim().is_empty() {
                ids.push(id.trim().to_string());
            }
        }
    }

    if ids.is_empty() {
        None
    } else {
        Some(ids)
    }
}

fn build_copilot_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent("GitHubCopilotChat/0.20.3")
        .timeout(std::time::Duration::from_secs(120))
        .build()?)
}

fn extract_usage(body: &[u8]) -> (i64, i64) {
    if let Ok(json) = serde_json::from_slice::<Value>(body) {
        let prompt = json
            .get("usage")
            .and_then(|u| u.get("prompt_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let completion = json
            .get("usage")
            .and_then(|u| u.get("completion_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        (prompt, completion)
    } else {
        (0, 0)
    }
}

fn is_model_not_supported(body: &[u8]) -> bool {
    if let Ok(json) = serde_json::from_slice::<Value>(body) {
        let error_code = json
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_str())
            .unwrap_or_default();

        if error_code == "model_not_supported" {
            return true;
        }

        let message = json
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or_default();

        return message.contains("model is not supported")
            || message.contains("requested model is not supported");
    }

    false
}

async fn log_request(state: &SharedState, record: UsageRecord) {
    let db = {
        let s = state.lock().await;
        s.db.clone()
    };
    if let Err(e) = insert_log(&db, &record).await {
        tracing::warn!("Failed to log request: {}", e);
    }
}

async fn send_copilot_request(
    client: &reqwest::Client,
    copilot_token: &str,
    body: &Value,
) -> Result<reqwest::Response, reqwest::Error> {
    client
        .post(COPILOT_API_URL)
        .header("Authorization", format!("Bearer {}", copilot_token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Copilot-Integration-Id", "vscode-chat")
        .json(body)
        .send()
        .await
}

fn error_response(status: u16, message: &str) -> Response {
    let body = serde_json::json!({
        "error": {
            "message": message,
            "type": "proxy_error",
            "code": status
        }
    });
    axum::response::Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&body).unwrap_or_default(),
        ))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
