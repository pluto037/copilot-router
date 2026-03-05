use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use crate::state::AppState;
use super::exchange_github_token;

/// Background loop that refreshes the Copilot token before it expires.
pub async fn start_refresh_loop(state: Arc<Mutex<AppState>>) {
    loop {
        // Check if token needs refresh (refresh when < 5 minutes remaining)
        let needs_refresh = {
            let s = state.lock().await;
            !s.is_token_valid()
        };

        if needs_refresh {
            let github_token = {
                let s = state.lock().await;
                s.config.github_token.clone().or_else(|| {
                    // Try auto-detection
                    crate::auth::vscode::detect_token()
                        .or_else(|| crate::auth::jetbrains::detect_token())
                })
            };

            if let Some(token) = github_token {
                match exchange_github_token(&token).await {
                    Ok(copilot_token) => {
                        let mut s = state.lock().await;
                        s.copilot_token = Some(copilot_token);
                        if s.config.github_token.is_none() {
                            s.config.github_token = Some(token);
                        }
                        info!("Copilot token refreshed successfully");
                    }
                    Err(e) => {
                        warn!("Failed to refresh Copilot token: {}", e);
                    }
                }
            } else {
                warn!("No GitHub token available for Copilot authentication");
            }
        }

        // Check every 5 minutes
        sleep(Duration::from_secs(300)).await;
    }
}
