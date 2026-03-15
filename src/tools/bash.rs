use anyhow::{bail, Context, Result};
use serde_json::json;
use std::path::Path;
use std::time::Duration;

use crate::config::types::ShellConfig;

pub struct BashTool {
    deny_patterns: Vec<String>,
    _timeout: Duration,
}

impl BashTool {
    pub fn new(shell_config: &ShellConfig) -> Self {
        Self {
            deny_patterns: shell_config.deny_patterns.clone(),
            _timeout: Duration::from_secs(shell_config.timeout_seconds),
        }
    }

    pub fn name(&self) -> &str {
        "bash"
    }

    pub fn description(&self) -> &str {
        "Execute a shell command. The command runs in the workspace directory. Returns stdout, stderr, and exit code."
    }

    pub fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                }
            },
            "required": ["command"]
        })
    }

    pub fn execute(&self, args: serde_json::Value, workspace: &Path) -> Result<String> {
        let command = args["command"]
            .as_str()
            .context("missing required parameter: command")?;

        // Check deny patterns
        for pattern in &self.deny_patterns {
            if command.contains(pattern.as_str()) {
                bail!("command denied: matches blocked pattern '{}'", pattern);
            }
        }

        let output = std::process::Command::new("bash")
            .arg("-c")
            .arg(command)
            .current_dir(workspace)
            .output()
            .context("failed to execute command")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        let mut combined = String::new();
        if !stdout.is_empty() {
            combined.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str("[stderr]\n");
            combined.push_str(&stderr);
        }

        // Truncate to last 10000 chars
        const MAX_OUTPUT: usize = 10000;
        if combined.len() > MAX_OUTPUT {
            let truncated = &combined[combined.len() - MAX_OUTPUT..];
            combined = format!("(output truncated)\n...{}", truncated);
        }

        combined.push_str(&format!("\n[exit code: {}]", exit_code));
        Ok(combined)
    }
}
