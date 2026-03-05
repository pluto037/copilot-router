use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMapping {
    pub from_model: String,
    pub to_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientModelTargets {
    pub claude_code: String,
    pub codex: String,
    pub generic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeModelProfile {
    pub default: String,
    pub haiku: String,
    pub sonnet: String,
    pub opus: String,
    pub reasoning: String,
    pub small_fast: String,
}

impl Default for ClaudeModelProfile {
    fn default() -> Self {
        Self {
            default: "claude-sonnet-4-6".to_string(),
            haiku: "claude-haiku-4-5".to_string(),
            sonnet: "claude-sonnet-4-6".to_string(),
            opus: "claude-opus-4-6".to_string(),
            reasoning: "claude-sonnet-4-6".to_string(),
            small_fast: "claude-haiku-4-5".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexModelProfile {
    pub default: String,
    pub reasoning: String,
    pub small_fast: String,
}

impl Default for CodexModelProfile {
    fn default() -> Self {
        Self {
            default: "gpt-5.2-codex".to_string(),
            reasoning: "gpt-5.2-codex".to_string(),
            small_fast: "gpt-5.1-codex-mini-preview".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericModelProfile {
    pub default: String,
}

impl Default for GenericModelProfile {
    fn default() -> Self {
        Self {
            default: "gpt-4o".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientModelProfiles {
    #[serde(default)]
    pub claude_code: ClaudeModelProfile,
    #[serde(default)]
    pub codex: CodexModelProfile,
    #[serde(default)]
    pub generic: GenericModelProfile,
}

impl Default for ClientModelTargets {
    fn default() -> Self {
        Self {
            claude_code: "claude-sonnet-4-6".to_string(),
            codex: "gpt-5.2-codex".to_string(),
            generic: "gpt-4o".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_port")]
    pub proxy_port: u16,
    #[serde(default = "default_proxy_enabled")]
    pub proxy_enabled: bool,
    pub auth_mode: AuthMode,
    pub github_token: Option<String>,
    #[serde(default)]
    pub client_model_targets: ClientModelTargets,
    #[serde(default)]
    pub client_model_profiles: ClientModelProfiles,
    #[serde(default)]
    pub model_mappings: Vec<ModelMapping>,
    #[serde(default)]
    pub start_on_login: bool,
    #[serde(default)]
    pub start_minimized: bool,
}

fn default_port() -> u16 {
    3100
}

fn default_proxy_enabled() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            proxy_port: 3100,
            proxy_enabled: true,
            auth_mode: AuthMode::Auto,
            github_token: None,
            client_model_targets: ClientModelTargets::default(),
            client_model_profiles: ClientModelProfiles::default(),
            model_mappings: default_model_mappings(),
            start_on_login: false,
            start_minimized: false,
        }
    }
}

fn default_model_mappings() -> Vec<ModelMapping> {
    vec![
        ModelMapping {
            from_model: "*".to_string(),
            to_model: "gpt-4o".to_string(),
        },
    ]
}

#[derive(Debug, Clone)]
pub struct CopilotToken {
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub struct AppState {
    pub config: AppConfig,
    pub db: SqlitePool,
    pub copilot_token: Option<CopilotToken>,
    pub token_source: Option<String>,
    pub proxy_running: bool,
    pub proxy_port: u16,
}

impl AppState {
    pub async fn new(db_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
        let pool = SqlitePool::connect(&db_url).await?;

        // Run migrations
        crate::usage::tracker::run_migrations(&pool).await?;

        // Load config from DB
        let config = load_config(&pool).await.unwrap_or_default();
        let proxy_port = config.proxy_port;

        Ok(Self {
            config,
            db: pool,
            copilot_token: None,
            token_source: None,
            proxy_running: false,
            proxy_port,
        })
    }

    pub fn is_token_valid(&self) -> bool {
        if let Some(token) = &self.copilot_token {
            token.expires_at > chrono::Utc::now() + chrono::Duration::minutes(2)
        } else {
            false
        }
    }

    pub fn resolve_model(&self, requested_model: &str) -> String {
        let targeted = self.resolve_model_by_client_target(requested_model);
        if !targeted.trim().is_empty() {
            return targeted;
        }

        let mut wildcard_model: Option<String> = None;

        for mapping in &self.config.model_mappings {
            let from = mapping.from_model.trim();
            let to = mapping.to_model.trim();
            if to.is_empty() {
                continue;
            }

            if from == "*" || from.eq_ignore_ascii_case("all") || from.is_empty() {
                if wildcard_model.is_none() {
                    wildcard_model = Some(to.to_string());
                }
                continue;
            }

            if requested_model.starts_with(from) {
                return to.to_string();
            }
        }

        if let Some(model) = wildcard_model {
            return model;
        }

        requested_model.to_string()
    }

    fn resolve_model_by_client_target(&self, requested_model: &str) -> String {
        let model_lower = requested_model.to_lowercase();

        if model_lower.contains("codex") {
            if model_lower.contains("mini") || model_lower.contains("fast") {
                return self.configured_target_with_fallback(
                    &self.config.client_model_profiles.codex.small_fast,
                    &self.config.client_model_targets.codex,
                );
            }

            if model_lower.contains("reason") {
                return self.configured_target_with_fallback(
                    &self.config.client_model_profiles.codex.reasoning,
                    &self.config.client_model_targets.codex,
                );
            }

            return self.configured_target_with_fallback(
                &self.config.client_model_profiles.codex.default,
                &self.config.client_model_targets.codex,
            );
        }

        if model_lower.contains("claude")
            || model_lower.contains("sonnet")
            || model_lower.contains("haiku")
            || model_lower.contains("opus")
        {
            if model_lower.contains("haiku") {
                return self.configured_target_with_fallback(
                    &self.config.client_model_profiles.claude_code.haiku,
                    &self.config.client_model_targets.claude_code,
                );
            }

            if model_lower.contains("opus") {
                return self.configured_target_with_fallback(
                    &self.config.client_model_profiles.claude_code.opus,
                    &self.config.client_model_targets.claude_code,
                );
            }

            if model_lower.contains("reason") {
                return self.configured_target_with_fallback(
                    &self.config.client_model_profiles.claude_code.reasoning,
                    &self.config.client_model_targets.claude_code,
                );
            }

            if model_lower.contains("fast") {
                return self.configured_target_with_fallback(
                    &self.config.client_model_profiles.claude_code.small_fast,
                    &self.config.client_model_targets.claude_code,
                );
            }

            if model_lower.contains("sonnet") {
                return self.configured_target_with_fallback(
                    &self.config.client_model_profiles.claude_code.sonnet,
                    &self.config.client_model_targets.claude_code,
                );
            }

            return self.configured_target_with_fallback(
                &self.config.client_model_profiles.claude_code.default,
                &self.config.client_model_targets.claude_code,
            );
        }

        self.configured_target_with_fallback(
            &self.config.client_model_profiles.generic.default,
            &self.config.client_model_targets.generic,
        )
    }

    fn configured_target_with_fallback(&self, primary: &str, fallback: &str) -> String {
        let primary_trimmed = primary.trim();
        if !primary_trimmed.is_empty() {
            return normalize_upstream_model_id(primary_trimmed);
        }

        self.configured_target_or_empty(fallback)
    }

    fn configured_target_or_empty(&self, value: &str) -> String {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            String::new()
        } else {
            normalize_upstream_model_id(trimmed)
        }
    }
}

fn normalize_upstream_model_id(model: &str) -> String {
    let trimmed = model.trim();
    let lower = trimmed.to_lowercase();

    match lower.as_str() {
        "claude haiku 4.5" => "claude-haiku-4-5".to_string(),
        "claude opus 4.5" => "claude-opus-4-5".to_string(),
        "claude opus 4.6" => "claude-opus-4-6".to_string(),
        "claude sonnet 4" => "claude-sonnet-4".to_string(),
        "claude sonnet 4.5" => "claude-sonnet-4-5".to_string(),
        "claude sonnet 4.6" => "claude-sonnet-4-6".to_string(),
        "gemini 2.5 pro" => "gemini-2.5-pro".to_string(),
        "gemini 3 flash (preview)" => "gemini-3-flash-preview".to_string(),
        "gemini 3 pro (preview)" => "gemini-3-pro-preview".to_string(),
        "gemini 3.1 pro (preview)" => "gemini-3.1-pro-preview".to_string(),
        "gpt-4.1" => "gpt-4.1".to_string(),
        "gpt-4o" => "gpt-4o".to_string(),
        "gpt-5 mini" => "gpt-5-mini".to_string(),
        "gpt-5.1" => "gpt-5.1".to_string(),
        "gpt-5.1-codex" => "gpt-5.1-codex".to_string(),
        "gpt-5.1-codex-max" => "gpt-5.1-codex-max".to_string(),
        "gpt-5.1-codex-mini (preview)" => "gpt-5.1-codex-mini-preview".to_string(),
        "gpt-5.2" => "gpt-5.2".to_string(),
        "gpt-5.2-codex" => "gpt-5.2-codex".to_string(),
        "gpt-5.3-codex" => "gpt-5.3-codex".to_string(),
        "grok code fast 1" => "grok-code-fast-1".to_string(),
        _ => trimmed.to_string(),
    }
}

async fn load_config(pool: &SqlitePool) -> Result<AppConfig> {
    let value = crate::usage::tracker::load_config_from_db(pool).await?;

    if let Some(json_str) = value {
        Ok(serde_json::from_str(&json_str)?)
    } else {
        Ok(AppConfig::default())
    }
}

pub async fn save_config_to_db(pool: &SqlitePool, config: &AppConfig) -> Result<()> {
    let value = serde_json::to_string(config)?;
    crate::usage::tracker::save_config_to_db_raw(pool, &value).await
}
