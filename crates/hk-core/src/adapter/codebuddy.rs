// MCP config reference: https://www.codebuddy.cn/docs/cli/mcp
// Config file: ~/.codebuddy/.mcp.json (user scope), .mcp.json (project scope)
// Format: JSON, top-level key "mcpServers", sub-keys: command, args, env, type, url, headers
//
// Skills reference: https://www.codebuddy.cn/docs/cli/skills
// Skills: ~/.codebuddy/skills/, .codebuddy/skills/
//
// Hooks reference: https://www.codebuddy.cn/docs/cli/hooks-guide
// Hooks: ~/.codebuddy/settings.json "hooks" key
// Format: ClaudeLike (PascalCase events: PreToolUse, PostToolUse, Stop, etc.)

use super::{AgentAdapter, HookEntry, McpServerEntry};
use std::path::{Path, PathBuf};

pub struct CodeBuddyAdapter {
    home: PathBuf,
}

impl Default for CodeBuddyAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeBuddyAdapter {
    pub fn new() -> Self {
        Self {
            home: dirs::home_dir().unwrap_or_default(),
        }
    }

    #[cfg(test)]
    pub fn with_home(home: PathBuf) -> Self {
        Self { home }
    }

    fn parse_json(path: &Path) -> Option<serde_json::Value> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }
}

impl AgentAdapter for CodeBuddyAdapter {
    fn name(&self) -> &str {
        "codebuddy"
    }

    fn base_dir(&self) -> PathBuf {
        self.home.join(".codebuddy")
    }

    fn detect(&self) -> bool {
        self.base_dir().exists()
    }

    fn skill_dirs(&self) -> Vec<PathBuf> {
        vec![self.base_dir().join("skills")]
    }

    fn mcp_config_path(&self) -> PathBuf {
        self.base_dir().join(".mcp.json")
    }

    fn hook_config_path(&self) -> PathBuf {
        self.base_dir().join("settings.json")
    }

    fn plugin_dirs(&self) -> Vec<PathBuf> {
        vec![]
    }

    fn read_mcp_servers(&self) -> Vec<McpServerEntry> {
        self.read_mcp_servers_from(&self.mcp_config_path())
    }

    fn read_mcp_servers_from(&self, path: &Path) -> Vec<McpServerEntry> {
        let Some(settings) = Self::parse_json(path) else {
            return vec![];
        };
        let Some(servers) = settings.get("mcpServers").and_then(|v| v.as_object()) else {
            return vec![];
        };

        servers
            .iter()
            .map(|(name, val)| McpServerEntry {
                name: name.clone(),
                command: val
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .into(),
                args: val
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                env: val
                    .get("env")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect()
    }

    fn translate_hook_event(&self, event: &str) -> Option<String> {
        super::hook_events::to_claude(event)
    }

    fn read_hooks(&self) -> Vec<HookEntry> {
        self.read_hooks_from(&self.hook_config_path())
    }

    fn read_hooks_from(&self, path: &Path) -> Vec<HookEntry> {
        let Some(settings) = Self::parse_json(path) else {
            return vec![];
        };
        let Some(hooks) = settings.get("hooks").and_then(|v| v.as_object()) else {
            return vec![];
        };

        let mut entries = Vec::new();
        for (event, hook_list) in hooks {
            let Some(arr) = hook_list.as_array() else {
                continue;
            };
            for hook in arr {
                let matcher = hook
                    .get("matcher")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                if let Some(cmds) = hook.get("hooks").and_then(|v| v.as_array()) {
                    for cmd in cmds {
                        // String format: "echo test"
                        let cmd_str = if let Some(s) = cmd.as_str() {
                            Some(s.to_string())
                        }
                        // Object format: {"type": "command", "command": "echo test"}
                        else if let Some(s) = cmd.get("command").and_then(|v| v.as_str()) {
                            Some(s.to_string())
                        }
                        // Prompt/agent hook: {"type": "prompt", "prompt": "..."}
                        else {
                            cmd.get("prompt")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        };
                        if let Some(command) = cmd_str {
                            entries.push(HookEntry {
                                event: event.clone(),
                                matcher: matcher.clone(),
                                command,
                            });
                        }
                    }
                }
            }
        }
        entries
    }

    fn global_rules_files(&self) -> Vec<PathBuf> {
        vec![]
    }

    fn global_memory_files(&self) -> Vec<PathBuf> {
        let memories_dir = self.base_dir().join("memories");
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&memories_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().is_some_and(|e| e == "md") {
                    files.push(p);
                }
            }
        }
        files
    }

    fn global_settings_files(&self) -> Vec<PathBuf> {
        vec![
            self.base_dir().join(".mcp.json"),
            self.base_dir().join("settings.json"),
            self.base_dir().join("settings.local.json"),
        ]
    }

    fn project_rules_patterns(&self) -> Vec<String> {
        vec!["AGENTS.md".into()]
    }

    fn project_settings_patterns(&self) -> Vec<String> {
        vec![
            ".codebuddy/settings.json".into(),
            ".codebuddy/settings.local.json".into(),
            ".mcp.json".into(),
        ]
    }

    fn project_ignore_patterns(&self) -> Vec<String> {
        vec![]
    }

    fn project_skill_dirs(&self) -> Vec<String> {
        vec![".codebuddy/skills".into()]
    }

    fn project_mcp_config_relpath(&self) -> Option<String> {
        Some(".mcp.json".into())
    }

    fn project_hook_config_relpath(&self) -> Option<String> {
        Some(".codebuddy/settings.json".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_codebuddy_adapter_name() {
        let adapter = CodeBuddyAdapter::new();
        assert_eq!(adapter.name(), "codebuddy");
    }

    #[test]
    fn test_codebuddy_detect_with_dir() {
        let dir = TempDir::new().unwrap();
        let codebuddy_dir = dir.path().join(".codebuddy");
        std::fs::create_dir_all(&codebuddy_dir).unwrap();
        let adapter = CodeBuddyAdapter::with_home(dir.path().to_path_buf());
        assert!(adapter.detect());
    }

    #[test]
    fn test_codebuddy_detect_without_dir() {
        let dir = TempDir::new().unwrap();
        let adapter = CodeBuddyAdapter::with_home(dir.path().to_path_buf());
        assert!(!adapter.detect());
    }

    #[test]
    fn test_codebuddy_skill_dirs() {
        let dir = TempDir::new().unwrap();
        let adapter = CodeBuddyAdapter::with_home(dir.path().to_path_buf());
        let dirs = adapter.skill_dirs();
        assert_eq!(dirs.len(), 1);
        assert!(dirs[0].ends_with(".codebuddy/skills"));
    }

    #[test]
    fn test_codebuddy_read_mcp_servers() {
        let dir = TempDir::new().unwrap();
        let codebuddy_dir = dir.path().join(".codebuddy");
        std::fs::create_dir_all(&codebuddy_dir).unwrap();
        std::fs::write(
            codebuddy_dir.join(".mcp.json"),
            r#"{"mcpServers":{"github":{"command":"npx","args":["-y","@modelcontextprotocol/server-github"],"env":{"GITHUB_TOKEN":"ghp_test"}}}}"#,
        ).unwrap();
        let adapter = CodeBuddyAdapter::with_home(dir.path().to_path_buf());
        let servers = adapter.read_mcp_servers();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "github");
        assert_eq!(servers[0].command, "npx");
    }

    #[test]
    fn test_codebuddy_read_hooks_string_format() {
        let dir = TempDir::new().unwrap();
        let codebuddy_dir = dir.path().join(".codebuddy");
        std::fs::create_dir_all(&codebuddy_dir).unwrap();
        std::fs::write(
            codebuddy_dir.join("settings.json"),
            r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":["echo test"]}]}}"#,
        )
        .unwrap();
        let adapter = CodeBuddyAdapter::with_home(dir.path().to_path_buf());
        let hooks = adapter.read_hooks();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].event, "PreToolUse");
        assert_eq!(hooks[0].command, "echo test");
    }

    #[test]
    fn test_codebuddy_read_hooks_object_format() {
        let dir = TempDir::new().unwrap();
        let codebuddy_dir = dir.path().join(".codebuddy");
        std::fs::create_dir_all(&codebuddy_dir).unwrap();
        std::fs::write(
            codebuddy_dir.join("settings.json"),
            r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"afplay /System/Library/Sounds/Glass.aiff"}]}]}}"#,
        ).unwrap();
        let adapter = CodeBuddyAdapter::with_home(dir.path().to_path_buf());
        let hooks = adapter.read_hooks();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].event, "Stop");
        assert_eq!(hooks[0].command, "afplay /System/Library/Sounds/Glass.aiff");
    }

    #[test]
    fn test_codebuddy_config_methods() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = CodeBuddyAdapter::with_home(tmp.path().to_path_buf());

        let global_rules = adapter.global_rules_files();
        assert!(global_rules.is_empty());

        let global_settings = adapter.global_settings_files();
        assert_eq!(global_settings.len(), 3);
        assert!(global_settings[0].ends_with(".mcp.json"));
        assert!(global_settings[1].ends_with("settings.json"));
        assert!(global_settings[2].ends_with("settings.local.json"));

        let project_rules = adapter.project_rules_patterns();
        assert!(project_rules.contains(&"AGENTS.md".to_string()));

        let project_settings = adapter.project_settings_patterns();
        assert!(project_settings.contains(&".codebuddy/settings.json".to_string()));
        assert!(project_settings.contains(&".codebuddy/settings.local.json".to_string()));
        assert!(project_settings.contains(&".mcp.json".to_string()));

        let project_ignore = adapter.project_ignore_patterns();
        assert!(project_ignore.is_empty());
    }
}
