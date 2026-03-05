import { invoke } from "@tauri-apps/api/core";

export interface TokenStatus {
  has_token: boolean;
  token_source: string | null;
  expires_at: string | null;
  is_valid: boolean;
}

export interface ClaudeTakeoverStatus {
  settings_path: string;
  exists: boolean;
  anthropic_base_url: string | null;
  anthropic_api_key: string | null;
  anthropic_auth_token: string | null;
  using_local_proxy: boolean;
}

export interface ProxyStatus {
  running: boolean;
  port: number;
  requests_today: number;
  total_requests: number;
}

export interface UsageStats {
  date: string;
  request_count: number;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  model: string;
}

export interface CopilotUsageOverview {
  today_requests: number;
  total_requests: number;
  requests_7d: number;
  tokens_7d: number;
  success_rate_7d: number;
  avg_latency_ms_7d: number;
  premium_usage_percent: number | null;
  allowance_reset_at: string | null;
  remote_source: string | null;
  remote_error: string | null;
  remote_raw: unknown | null;
}

export interface LogEntry {
  id: number;
  timestamp: string;
  method: string;
  path: string;
  requested_model: string;
  mapped_model: string;
  model: string;
  status_code: number;
  prompt_tokens: number;
  completion_tokens: number;
  latency_ms: number;
  error: string | null;
}

export interface AppConfig {
  proxy_port: number;
  proxy_enabled: boolean;
  auth_mode: "auto" | "manual";
  github_token: string | null;
  client_model_targets: ClientModelTargets;
  client_model_profiles: ClientModelProfiles;
  model_mappings: ModelMapping[];
  start_on_login: boolean;
  start_minimized: boolean;
}

export interface ClientModelTargets {
  claude_code: string;
  codex: string;
  generic: string;
}

export interface ClaudeModelProfile {
  default: string;
  haiku: string;
  sonnet: string;
  opus: string;
  reasoning: string;
  small_fast: string;
}

export interface CodexModelProfile {
  default: string;
  reasoning: string;
  small_fast: string;
}

export interface GenericModelProfile {
  default: string;
}

export interface ClientModelProfiles {
  claude_code: ClaudeModelProfile;
  codex: CodexModelProfile;
  generic: GenericModelProfile;
}

export interface ModelMapping {
  from_model: string;
  to_model: string;
}

export interface DeviceAuthInfo {
  device_code: string;
  user_code: string;
  verification_uri: string;
}

export interface ModelMappingTestResult {
  requested_model: string;
  resolved_model: string;
  is_mapped: boolean;
  upstream_checked: boolean;
  upstream_ok: boolean | null;
  upstream_status: number | null;
  upstream_error: string | null;
}

export const tauriApi = {
  getProxyStatus: () => invoke<ProxyStatus>("get_proxy_status"),
  getTokenStatus: () => invoke<TokenStatus>("get_token_status"),
  getClaudeTakeoverStatus: () => invoke<ClaudeTakeoverStatus>("get_claude_takeover_status"),
  repairClaudeTakeover: () => invoke<void>("repair_claude_takeover"),
  getUsageStats: (days: number) => invoke<UsageStats[]>("get_usage_stats", { days }),
  getCopilotUsageOverview: () => invoke<CopilotUsageOverview>("get_copilot_usage_overview"),
  getRecentLogs: (limit: number) => invoke<LogEntry[]>("get_recent_logs", { limit }),
  getConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<void>("save_config", { config }),
  testModelMapping: (requestedModel: string) =>
    invoke<ModelMappingTestResult>("test_model_mapping", { requestedModel }),
  startProxy: () => invoke<void>("start_proxy"),
  stopProxy: () => invoke<void>("stop_proxy"),
  refreshToken: () => invoke<void>("refresh_token"),
  autoDetectToken: () => invoke<string | null>("auto_detect_token"),
  requestGithubDeviceCode: () => invoke<DeviceAuthInfo>("request_github_device_code"),
  waitGithubDeviceToken: (deviceCode: string) => invoke<string>("wait_github_device_token", { deviceCode }),
  copyToClipboard: (text: string) => invoke<void>("copy_to_clipboard", { text }),
  openLogsDir: () => invoke<void>("open_logs_dir"),
  clearLogs: () => invoke<void>("clear_logs"),
};
