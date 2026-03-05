pub mod jetbrains;
pub mod oauth;
pub mod refresher;
pub mod vscode;

use anyhow::Result;
use serde::Deserialize;
use crate::state::CopilotToken;

const COPILOT_TOKEN_URL: &str =
    "https://api.github.com/copilot_internal/v2/token";

#[derive(Debug, Deserialize)]
struct CopilotTokenResponse {
    token: String,
    expires_at: i64,
}

/// Exchange a GitHub OAuth token for a short-lived Copilot API token.
pub async fn exchange_github_token(github_token: &str) -> Result<CopilotToken> {
    let client = reqwest::Client::builder()
        .user_agent("GitHubCopilotChat/0.20.3")
        .build()?;

    let resp = client
        .get(COPILOT_TOKEN_URL)
        .header("Authorization", format!("token {}", github_token))
        .header("Accept", "application/json")
        .header("Editor-Version", "vscode/1.94.0")
        .header("Editor-Plugin-Version", "copilot/1.233.0")
        .header("Openai-Organization", "github-copilot")
        .send()
        .await?
        .error_for_status()?;

    let body: CopilotTokenResponse = resp.json().await?;

    let expires_at = chrono::DateTime::from_timestamp(body.expires_at, 0)
        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::minutes(30));

    Ok(CopilotToken {
        token: body.token,
        expires_at,
    })
}
