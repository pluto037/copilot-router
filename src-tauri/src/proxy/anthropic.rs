/// Anthropic Messages API ↔ OpenAI Chat Completions format converters.

use anyhow::Result;
use bytes::Bytes;
use serde_json::{json, Value};

/// Convert an Anthropic `/v1/messages` request body to OpenAI `/v1/chat/completions` format.
pub fn to_openai_request(body: &Bytes) -> Result<Value> {
    let anthropic: Value = serde_json::from_slice(body)?;

    let model = anthropic
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("claude-sonnet-4-5")
        .to_string();

    let max_tokens = anthropic.get("max_tokens").and_then(|v| v.as_i64());

    let stream = anthropic
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Convert messages: Anthropic → OpenAI
    let mut messages: Vec<Value> = Vec::new();

    // Handle system prompt
    if let Some(system) = anthropic.get("system").and_then(|v| v.as_str()) {
        messages.push(json!({ "role": "system", "content": system }));
    }

    // Convert message array
    if let Some(input_messages) = anthropic.get("messages").and_then(|v| v.as_array()) {
        for msg in input_messages {
            let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
            let content = convert_anthropic_content(msg.get("content"));
            messages.push(json!({ "role": role, "content": content }));
        }
    }

    let mut openai_req = json!({
        "model": model,
        "messages": messages,
        "stream": stream
    });

    if let Some(max_tok) = max_tokens {
        openai_req["max_tokens"] = json!(max_tok);
    }

    if let Some(temp) = anthropic.get("temperature") {
        openai_req["temperature"] = temp.clone();
    }

    if let Some(top_p) = anthropic.get("top_p") {
        openai_req["top_p"] = top_p.clone();
    }

    Ok(openai_req)
}

/// Convert Anthropic content block to OpenAI string content.
fn convert_anthropic_content(content: Option<&Value>) -> Value {
    match content {
        None => json!(""),
        Some(Value::String(s)) => json!(s),
        Some(Value::Array(parts)) => {
            // Convert content parts array to string
            let text_parts: Vec<String> = parts
                .iter()
                .filter_map(|part| {
                    if part.get("type").and_then(|v| v.as_str()) == Some("text") {
                        part.get("text").and_then(|v| v.as_str()).map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect();
            json!(text_parts.join(""))
        }
        Some(other) => other.clone(),
    }
}

/// Convert an OpenAI response back to Anthropic Messages API format.
pub fn to_anthropic_response(body: &[u8]) -> Result<Value> {
    let openai: Value = serde_json::from_slice(body)?;

    let id = openai
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("msg_unknown")
        .to_string();

    let model = openai
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let content_text = openai
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();

    let finish_reason = openai
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("finish_reason"))
        .and_then(|f| f.as_str())
        .map(map_finish_reason)
        .unwrap_or("end_turn");

    let usage = if let Some(u) = openai.get("usage") {
        json!({
            "input_tokens": u.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
            "output_tokens": u.get("completion_tokens").and_then(|v| v.as_i64()).unwrap_or(0)
        })
    } else {
        json!({ "input_tokens": 0, "output_tokens": 0 })
    };

    Ok(json!({
        "id": id,
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": [
            {
                "type": "text",
                "text": content_text
            }
        ],
        "stop_reason": finish_reason,
        "stop_sequence": null,
        "usage": usage
    }))
}

/// Convert SSE stream from OpenAI format to Anthropic format.
pub fn convert_stream_to_anthropic(body: &[u8]) -> bytes::Bytes {
    // For streaming, we pass through and let the client handle it
    // A full implementation would convert each SSE chunk
    // For now, return as-is with a content-type header change
    let text = String::from_utf8_lossy(body);
    let mut output = String::new();

    // Signal stream start
    output.push_str("event: message_start\n");
    output.push_str("data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_stream\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"copilot\",\"stop_reason\":null,\"usage\":{\"input_tokens\":0,\"output_tokens\":0}}}\n\n");

    output.push_str("event: content_block_start\n");
    output.push_str("data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n");

    // Parse and forward SSE chunks
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                continue;
            }
            if let Ok(chunk) = serde_json::from_str::<Value>(data) {
                let delta_text = chunk
                    .get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|c| c.first())
                    .and_then(|c| c.get("delta"))
                    .and_then(|d| d.get("content"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");

                if !delta_text.is_empty() {
                    let anthropic_chunk = json!({
                        "type": "content_block_delta",
                        "index": 0,
                        "delta": { "type": "text_delta", "text": delta_text }
                    });
                    output.push_str("event: content_block_delta\n");
                    output.push_str(&format!("data: {}\n\n", anthropic_chunk));
                }
            }
        }
    }

    output.push_str("event: content_block_stop\n");
    output.push_str("data: {\"type\":\"content_block_stop\",\"index\":0}\n\n");

    output.push_str("event: message_stop\n");
    output.push_str("data: {\"type\":\"message_stop\"}\n\n");

    bytes::Bytes::from(output)
}

fn map_finish_reason(reason: &str) -> &'static str {
    match reason {
        "stop" => "end_turn",
        "length" => "max_tokens",
        "content_filter" => "stop_sequence",
        _ => "end_turn",
    }
}
