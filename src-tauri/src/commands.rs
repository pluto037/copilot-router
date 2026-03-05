use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

use crate::auth;
use crate::state::{save_config_to_db, AppConfig, AppState};
use crate::usage::tracker;

pub type SharedState = Arc<Mutex<AppState>>;
const COPILOT_API_URL: &str = "https://api.githubcopilot.com/chat/completions";
const PROXY_TOKEN_PLACEHOLDER: &str = "PROXY_MANAGED";

#[derive(Serialize)]
pub struct ProxyStatus {
    pub running: bool,
    pub port: u16,
    pub requests_today: i64,
    pub total_requests: i64,
}

#[derive(Serialize)]
pub struct TokenStatus {
    pub has_token: bool,
    pub token_source: Option<String>,
    pub expires_at: Option<String>,
    pub is_valid: bool,
}

#[derive(Serialize)]
pub struct ClaudeTakeoverStatus {
    pub settings_path: String,
    pub exists: bool,
    pub anthropic_base_url: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub anthropic_auth_token: Option<String>,
    pub using_local_proxy: bool,
}

#[tauri::command]
pub async fn get_proxy_status(state: State<'_, SharedState>) -> Result<ProxyStatus, String> {
    let s = state.lock().await;
    let requests_today = tracker::get_today_request_count(&s.db)
        .await
        .unwrap_or(0);
    let total_requests = tracker::get_total_request_count(&s.db)
        .await
        .unwrap_or(0);

    Ok(ProxyStatus {
        running: s.proxy_running,
        port: s.proxy_port,
        requests_today,
        total_requests,
    })
}

#[tauri::command]
pub async fn get_token_status(state: State<'_, SharedState>) -> Result<TokenStatus, String> {
    let s = state.lock().await;

    let (has_token, expires_at, is_valid) = if let Some(token) = &s.copilot_token {
        let expires = token.expires_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let valid = s.is_token_valid();
        (true, Some(expires), valid)
    } else {
        (false, None, false)
    };

    Ok(TokenStatus {
        has_token,
        token_source: s.token_source.clone(),
        expires_at,
        is_valid,
    })
}

#[tauri::command]
pub async fn get_claude_takeover_status() -> Result<ClaudeTakeoverStatus, String> {
    let path = claude_settings_path()?;
    let settings_path = path.to_string_lossy().to_string();

    if !path.exists() {
        return Ok(ClaudeTakeoverStatus {
            settings_path,
            exists: false,
            anthropic_base_url: None,
            anthropic_api_key: None,
            anthropic_auth_token: None,
            using_local_proxy: false,
        });
    }

    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read Claude settings: {}", e))?;

    let root: Value = serde_json::from_str(&content).unwrap_or_else(|_| json!({}));
    let env = root
        .get("env")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let anthropic_base_url = env
        .get("ANTHROPIC_BASE_URL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let anthropic_api_key = env
        .get("ANTHROPIC_API_KEY")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let anthropic_auth_token = env
        .get("ANTHROPIC_AUTH_TOKEN")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let using_local_proxy = anthropic_base_url
        .as_ref()
        .map(|url| url.contains("127.0.0.1") || url.contains("localhost"))
        .unwrap_or(false);

    Ok(ClaudeTakeoverStatus {
        settings_path,
        exists: true,
        anthropic_base_url,
        anthropic_api_key,
        anthropic_auth_token,
        using_local_proxy,
    })
}

#[tauri::command]
pub async fn repair_claude_takeover(state: State<'_, SharedState>) -> Result<(), String> {
    let config = {
        let s = state.lock().await;
        s.config.clone()
    };

    sync_claude_code_proxy_settings(&config).await
}

#[tauri::command]
pub async fn get_usage_stats(
    state: State<'_, SharedState>,
    days: i64,
) -> Result<Vec<tracker::UsageStats>, String> {
    let s = state.lock().await;
    tracker::get_usage_stats(&s.db, days)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_recent_logs(
    state: State<'_, SharedState>,
    limit: i64,
) -> Result<Vec<tracker::LogEntry>, String> {
    let s = state.lock().await;
    tracker::get_recent_logs(&s.db, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_config(state: State<'_, SharedState>) -> Result<AppConfig, String> {
    let s = state.lock().await;
    Ok(s.config.clone())
}

#[tauri::command]
pub async fn save_config(
    state: State<'_, SharedState>,
    config: AppConfig,
) -> Result<(), String> {
    let db = {
        let s = state.lock().await;
        s.db.clone()
    };

    save_config_to_db(&db, &config)
        .await
        .map_err(|e| e.to_string())?;

    {
        let mut s = state.lock().await;
        s.config = config.clone();
    }

    sync_claude_code_proxy_settings(&config).await?;
    Ok(())
}

pub async fn sync_claude_code_settings_from_state(state: SharedState) -> Result<(), String> {
    let config = {
        let s = state.lock().await;
        s.config.clone()
    };
    sync_claude_code_proxy_settings(&config).await
}

pub async fn sync_claude_code_proxy_settings(config: &AppConfig) -> Result<(), String> {
    let path = claude_settings_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create Claude config dir: {}", e))?;
    }

    let mut root = if path.exists() {
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => serde_json::from_str::<Value>(&content).unwrap_or_else(|_| json!({})),
            Err(_) => json!({}),
        }
    } else {
        json!({})
    };

    if !root.is_object() {
        root = json!({});
    }

    let root_obj = root.as_object_mut().ok_or("Invalid Claude settings root")?;
    if !root_obj.get("env").map(|v| v.is_object()).unwrap_or(false) {
        root_obj.insert("env".to_string(), json!({}));
    }

    let env_obj = root_obj
        .get_mut("env")
        .and_then(|v| v.as_object_mut())
        .ok_or("Invalid Claude env object")?;

    if config.proxy_enabled {
        let proxy_base_url = format!("http://127.0.0.1:{}", config.proxy_port);
        env_obj.insert("ANTHROPIC_BASE_URL".to_string(), Value::String(proxy_base_url));
        env_obj.insert(
            "ANTHROPIC_API_KEY".to_string(),
            Value::String(PROXY_TOKEN_PLACEHOLDER.to_string()),
        );
        env_obj.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            Value::String(PROXY_TOKEN_PLACEHOLDER.to_string()),
        );

        let profile = &config.client_model_profiles.claude_code;

        upsert_or_remove_env_value(env_obj, "ANTHROPIC_MODEL", &profile.default);
        upsert_or_remove_env_value(env_obj, "ANTHROPIC_REASONING_MODEL", &profile.reasoning);
        upsert_or_remove_env_value(env_obj, "ANTHROPIC_DEFAULT_HAIKU_MODEL", &profile.haiku);
        upsert_or_remove_env_value(env_obj, "ANTHROPIC_DEFAULT_SONNET_MODEL", &profile.sonnet);
        upsert_or_remove_env_value(env_obj, "ANTHROPIC_DEFAULT_OPUS_MODEL", &profile.opus);
        upsert_or_remove_env_value(env_obj, "ANTHROPIC_SMALL_FAST_MODEL", &profile.small_fast);
    } else {
        if env_obj
            .get("ANTHROPIC_BASE_URL")
            .and_then(|v| v.as_str())
            .map(|v| v.contains("127.0.0.1") || v.contains("localhost"))
            .unwrap_or(false)
        {
            env_obj.remove("ANTHROPIC_BASE_URL");
        }

        if env_obj
            .get("ANTHROPIC_API_KEY")
            .and_then(|v| v.as_str())
            .map(|v| v == PROXY_TOKEN_PLACEHOLDER)
            .unwrap_or(false)
        {
            env_obj.remove("ANTHROPIC_API_KEY");
        }

        if env_obj
            .get("ANTHROPIC_AUTH_TOKEN")
            .and_then(|v| v.as_str())
            .map(|v| v == PROXY_TOKEN_PLACEHOLDER)
            .unwrap_or(false)
        {
            env_obj.remove("ANTHROPIC_AUTH_TOKEN");
        }
    }

    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Failed to serialize Claude settings: {}", e))?;
    tokio::fs::write(&path, format!("{}\n", content))
        .await
        .map_err(|e| format!("Failed to write Claude settings: {}", e))?;

    Ok(())
}

fn upsert_or_remove_env_value(
    env_obj: &mut serde_json::Map<String, Value>,
    key: &str,
    value: &str,
) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        env_obj.remove(key);
    } else {
        env_obj.insert(key.to_string(), Value::String(trimmed.to_string()));
    }
}

fn claude_settings_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot resolve home directory")?;
    Ok(home.join(".claude").join("settings.json"))
}

#[derive(Serialize)]
pub struct ModelMappingTestResult {
    pub requested_model: String,
    pub resolved_model: String,
    pub is_mapped: bool,
    pub upstream_checked: bool,
    pub upstream_ok: Option<bool>,
    pub upstream_status: Option<u16>,
    pub upstream_error: Option<String>,
}

#[tauri::command]
pub async fn test_model_mapping(
    state: State<'_, SharedState>,
    requested_model: String,
) -> Result<ModelMappingTestResult, String> {
    let requested_model = requested_model.trim().to_string();
    if requested_model.is_empty() {
        return Err("requested_model cannot be empty".to_string());
    }

    let (resolved_model, copilot_token) = {
        let s = state.lock().await;
        (
            s.resolve_model(&requested_model),
            s.copilot_token
                .as_ref()
                .filter(|_| s.is_token_valid())
                .map(|t| t.token.clone()),
        )
    };

    let mut result = ModelMappingTestResult {
        requested_model: requested_model.clone(),
        resolved_model: resolved_model.clone(),
        is_mapped: requested_model != resolved_model,
        upstream_checked: false,
        upstream_ok: None,
        upstream_status: None,
        upstream_error: None,
    };

    let Some(copilot_token) = copilot_token else {
        result.upstream_error = Some("Copilot token not available or expired".to_string());
        return Ok(result);
    };

    result.upstream_checked = true;

    let client = reqwest::Client::builder()
        .user_agent("GitHubCopilotChat/0.20.3")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    let body = json!({
        "model": resolved_model,
        "messages": [{"role": "user", "content": "ping"}],
        "stream": false,
        "max_tokens": 1,
        "temperature": 0
    });

    let response = client
        .post(COPILOT_API_URL)
        .header("Authorization", format!("Bearer {}", copilot_token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Copilot-Integration-Id", "vscode-chat")
        .json(&body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let ok = resp.status().is_success();
            result.upstream_status = Some(status);
            result.upstream_ok = Some(ok);
            if !ok {
                let text = resp.text().await.unwrap_or_default();
                let preview: String = text.chars().take(300).collect();
                result.upstream_error = Some(if preview.is_empty() {
                    format!("Upstream returned status {}", status)
                } else {
                    format!("Upstream returned status {}: {}", status, preview)
                });
            }
        }
        Err(e) => {
            result.upstream_ok = Some(false);
            result.upstream_error = Some(e.to_string());
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn start_proxy(_state: State<'_, SharedState>) -> Result<(), String> {
    // Proxy is started automatically at launch; this is a no-op placeholder
    Ok(())
}

#[tauri::command]
pub async fn stop_proxy(state: State<'_, SharedState>) -> Result<(), String> {
    let mut s = state.lock().await;
    s.proxy_running = false;
    Ok(())
}

#[tauri::command]
pub async fn refresh_token(state: State<'_, SharedState>) -> Result<(), String> {
    let github_token = {
        let s = state.lock().await;
        s.config.github_token.clone()
    };

    if let Some(token) = github_token {
        match auth::exchange_github_token(&token).await {
            Ok(copilot_token) => {
                let mut s = state.lock().await;
                s.token_source = Some("manual".to_string());
                s.copilot_token = Some(copilot_token);
                Ok(())
            }
            Err(e) => Err(format!("Failed to refresh token: {}", e)),
        }
    } else {
        Err("No GitHub token configured".to_string())
    }
}

#[tauri::command]
pub async fn auto_detect_token(state: State<'_, SharedState>) -> Result<Option<String>, String> {
    // Try VS Code first, then JetBrains
    let token = auth::vscode::detect_token()
        .or_else(|| auth::jetbrains::detect_token());

    if let Some(ref t) = token {
        // Store detected token and try to get Copilot token
        match auth::exchange_github_token(t).await {
            Ok(copilot_token) => {
                let mut s = state.lock().await;
                s.config.github_token = Some(t.clone());
                s.copilot_token = Some(copilot_token);
                s.token_source = Some(
                    if auth::vscode::detect_token().is_some() {
                        "VS Code"
                    } else {
                        "JetBrains"
                    }
                    .to_string(),
                );
            }
            Err(e) => {
                tracing::warn!("Detected token but failed to exchange: {}", e);
            }
        }
    }

    Ok(token)
}

#[tauri::command]
pub async fn clear_logs(state: State<'_, SharedState>) -> Result<(), String> {
    let s = state.lock().await;
    tracker::clear_logs(&s.db)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct DeviceAuthInfo {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
}

#[tauri::command]
pub async fn request_github_device_code() -> Result<DeviceAuthInfo, String> {
    let res = crate::auth::oauth::request_device_code()
        .await
        .map_err(|e| e.to_string())?;
    Ok(DeviceAuthInfo {
        device_code: res.device_code,
        user_code: res.user_code,
        verification_uri: res.verification_uri,
    })
}

#[tauri::command]
pub async fn wait_github_device_token(
    state: State<'_, SharedState>,
    device_code: String,
) -> Result<String, String> {
    let token = crate::auth::oauth::poll_token(&device_code)
        .await
        .map_err(|e| e.to_string())?;

    match crate::auth::exchange_github_token(&token).await {
        Ok(copilot_token) => {
            let mut s = state.lock().await;
            s.config.github_token = Some(token.clone());
            s.copilot_token = Some(copilot_token);
            s.token_source = Some("GitHub Auth".to_string());
            let config = s.config.clone();
            let db = s.db.clone();
            let _ = crate::state::save_config_to_db(&db, &config).await;
            Ok(token)
        }
        Err(e) => Err(format!("换取 Copilot 权限失败: {}", e)),
    }
}

#[tauri::command]
pub fn copy_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct CopilotUsageOverview {
    pub today_requests: i64,
    pub total_requests: i64,
    pub requests_7d: i64,
    pub tokens_7d: i64,
    pub success_rate_7d: f64,
    pub avg_latency_ms_7d: i64,
    pub premium_usage_percent: Option<f64>,
    pub allowance_reset_at: Option<String>,
    pub remote_source: Option<String>,
    pub remote_error: Option<String>,
    pub remote_raw: Option<Value>,
}

#[tauri::command]
pub async fn get_copilot_usage_overview(
    state: State<'_, SharedState>,
) -> Result<CopilotUsageOverview, String> {
    let (db, github_token, copilot_token) = {
        let s = state.lock().await;
        (
            s.db.clone(),
            s.config.github_token.clone(),
            s.copilot_token.as_ref().map(|t| t.token.clone()),
        )
    };

    let today_requests = tracker::get_today_request_count(&db)
        .await
        .unwrap_or(0);
    let total_requests = tracker::get_total_request_count(&db)
        .await
        .unwrap_or(0);

    let stats_7d = tracker::get_usage_stats(&db, 7)
        .await
        .unwrap_or_default();
    let requests_7d = stats_7d.iter().map(|r| r.request_count).sum::<i64>();
    let tokens_7d = stats_7d.iter().map(|r| r.total_tokens).sum::<i64>();

    let recent_logs = tracker::get_recent_logs(&db, 500)
        .await
        .unwrap_or_default();
    let mut ok_count = 0_i64;
    let mut total_count = 0_i64;
    let mut latency_sum = 0_i64;

    for log in &recent_logs {
        total_count += 1;
        latency_sum += log.latency_ms.max(0);
        if (200..300).contains(&log.status_code) {
            ok_count += 1;
        }
    }

    let success_rate_7d = if total_count > 0 {
        (ok_count as f64 / total_count as f64) * 100.0
    } else {
        0.0
    };

    let avg_latency_ms_7d = if total_count > 0 {
        latency_sum / total_count
    } else {
        0
    };

    let mut premium_usage_percent = None;
    let mut allowance_reset_at = None;
    let mut remote_source = None;
    let mut remote_error = None;
    let mut remote_raw = None;

    if let Some(github_token) = github_token {
        match fetch_remote_copilot_usage(&github_token, copilot_token.as_deref()).await {
            Ok((source, data)) => {
                let (percent, reset_at) = extract_usage_fields(&data);
                premium_usage_percent = percent;
                allowance_reset_at = reset_at;
                remote_source = Some(source);
                remote_raw = Some(data);
            }
            Err(e) => {
                remote_error = Some(e);
            }
        }
    } else {
        remote_error = Some("No GitHub token configured".to_string());
    }

    Ok(CopilotUsageOverview {
        today_requests,
        total_requests,
        requests_7d,
        tokens_7d,
        success_rate_7d,
        avg_latency_ms_7d,
        premium_usage_percent,
        allowance_reset_at,
        remote_source,
        remote_error,
        remote_raw,
    })
}

async fn fetch_remote_copilot_usage(
    github_token: &str,
    copilot_token: Option<&str>,
) -> Result<(String, Value), String> {
    let client = reqwest::Client::builder()
        .user_agent("GitHubCopilotChat/0.20.3")
        .build()
        .map_err(|e| e.to_string())?;

    let mut errors = Vec::new();

    let mut candidates: Vec<(&str, &str, String)> = vec![
        (
            "github-copilot-internal-user",
            "https://api.github.com/copilot_internal/user",
            format!("token {}", github_token),
        ),
        (
            "github-copilot-internal-usage",
            "https://api.github.com/copilot_internal/usage",
            format!("token {}", github_token),
        ),
    ];

    if let Some(copilot_token) = copilot_token {
        candidates.push((
            "githubcopilot-user",
            "https://api.githubcopilot.com/user",
            format!("Bearer {}", copilot_token),
        ));
        candidates.push((
            "githubcopilot-usage",
            "https://api.githubcopilot.com/usage",
            format!("Bearer {}", copilot_token),
        ));
    }

    for (source, url, auth_header) in candidates {
        let resp = match client
            .get(url)
            .header("Authorization", auth_header)
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                errors.push(format!("{} request failed: {}", source, e));
                continue;
            }
        };

        if !resp.status().is_success() {
            errors.push(format!("{} status: {}", source, resp.status()));
            continue;
        }

        match resp.json::<Value>().await {
            Ok(json) => return Ok((source.to_string(), json)),
            Err(e) => {
                errors.push(format!("{} parse error: {}", source, e));
            }
        }
    }

    Err(errors.join(" | "))
}

fn extract_usage_fields(data: &Value) -> (Option<f64>, Option<String>) {
    let premium_usage_percent = find_first_number_by_key(data, &[
        "premium_requests_percent",
        "premium_percentage",
        "usage_percentage",
        "percent",
        "percentage",
    ]);

    let allowance_reset_at = find_first_string_by_key_contains(data, "reset");

    (premium_usage_percent, allowance_reset_at)
}

fn find_first_number_by_key(value: &Value, keys: &[&str]) -> Option<f64> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(v) = map.get(*key) {
                    if let Some(n) = v.as_f64() {
                        return Some(n);
                    }
                    if let Some(s) = v.as_str() {
                        if let Ok(n) = s.parse::<f64>() {
                            return Some(n);
                        }
                    }
                }
            }

            for nested in map.values() {
                if let Some(found) = find_first_number_by_key(nested, keys) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(arr) => {
            for item in arr {
                if let Some(found) = find_first_number_by_key(item, keys) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn find_first_string_by_key_contains(value: &Value, needle: &str) -> Option<String> {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                if k.to_lowercase().contains(needle) {
                    if let Some(s) = v.as_str() {
                        return Some(s.to_string());
                    }
                }
                if let Some(found) = find_first_string_by_key_contains(v, needle) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(arr) => {
            for item in arr {
                if let Some(found) = find_first_string_by_key_contains(item, needle) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}
