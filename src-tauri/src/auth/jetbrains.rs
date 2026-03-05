/// Detect GitHub token from JetBrains IDE storage.
pub fn detect_token() -> Option<String> {
    let home = dirs::home_dir()?;

    let candidates = get_jetbrains_config_dirs(&home);

    for path in candidates {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
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

fn get_jetbrains_config_dirs(home: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut paths = vec![
        // Cross-platform GitHub Copilot hosts file
        home.join(".config/github-copilot/hosts.json"),
    ];

    // macOS: ~/Library/Application Support/JetBrains/<IDE>*/options/
    #[cfg(target_os = "macos")]
    {
        let app_support = home.join("Library/Application Support/JetBrains");
        if let Ok(entries) = std::fs::read_dir(&app_support) {
            for entry in entries.flatten() {
                let options = entry.path().join("options/github.xml");
                paths.push(options);
            }
        }
    }

    // Linux: ~/.config/JetBrains/<IDE>*/options/
    #[cfg(target_os = "linux")]
    {
        let config_dir = home.join(".config/JetBrains");
        if let Ok(entries) = std::fs::read_dir(&config_dir) {
            for entry in entries.flatten() {
                let options = entry.path().join("options/github.xml");
                paths.push(options);
            }
        }
    }

    // Windows: %APPDATA%\JetBrains\<IDE>*\options\
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = dirs::config_dir() {
            let jetbrains = appdata.join("JetBrains");
            if let Ok(entries) = std::fs::read_dir(&jetbrains) {
                for entry in entries.flatten() {
                    let options = entry.path().join("options/github.xml");
                    paths.push(options);
                }
            }
        }
    }

    paths
}
