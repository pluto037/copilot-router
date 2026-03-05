/// Auto-detect GitHub OAuth token from VS Code's storage.
///
/// VS Code stores the token in hosts.json or globalStorage files.
pub fn detect_token() -> Option<String> {
    try_hosts_json()
}

fn try_hosts_json() -> Option<String> {
    let home = dirs::home_dir()?;

    let candidates = [
        // VS Code (macOS)
        home.join("Library/Application Support/Code/User/globalStorage/github.copilot/hosts.json"),
        // VS Code (Linux)
        home.join(".config/Code/User/globalStorage/github.copilot/hosts.json"),
        // VS Code (Windows via Wine / cross-platform)
        home.join(".vscode/globalStorage/github.copilot/hosts.json"),
        // JetBrains shared path
        home.join(".config/github-copilot/hosts.json"),
    ];

    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                // hosts.json format: { "github.com": { "oauth_token": "ghp_..." } }
                if let Some(token) = json
                    .get("github.com")
                    .and_then(|v| v.get("oauth_token"))
                    .and_then(|v| v.as_str())
                {
                    return Some(token.to_string());
                }
            }
        }
    }

    None
}
