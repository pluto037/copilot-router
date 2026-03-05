use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessTokenResponse {
    pub access_token: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

const CLIENT_ID: &str = "01ab8ac9400c4e429b23";

/// 请求设备授权码
pub async fn request_device_code() -> anyhow::Result<DeviceCodeResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&[("client_id", CLIENT_ID), ("scope", "read:user")])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("HTTP {}: {}", status, text));
    }

    let data = resp.json::<DeviceCodeResponse>().await?;
    Ok(data)
}

/// 轮询直到获取到最终的 Token
pub async fn poll_token(device_code: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let resp = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", CLIENT_ID),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            continue;
        }

        let data = resp.json::<AccessTokenResponse>().await?;

        if let Some(token) = data.access_token {
            return Ok(token);
        }
        if let Some(err) = data.error {
            match err.as_str() {
                "authorization_pending" | "slow_down" => continue,
                "expired_token" => return Err(anyhow::anyhow!("设备校验码已过期，请重新发起登录")),
                "access_denied" => return Err(anyhow::anyhow!("用户拒绝了授权")),
                _ => {
                    return Err(anyhow::anyhow!(
                        "认证失败: {}",
                        data.error_description.unwrap_or(err)
                    ))
                }
            }
        }
    }
}

/// Validate that a GitHub token has Copilot access.
pub async fn validate_github_token(token: &str) -> anyhow::Result<bool> {
    let client = reqwest::Client::builder()
        .user_agent("GitHubCopilotChat/0.20.3")
        .build()?;

    let resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    Ok(resp.status().is_success())
}
