use anyhow::{Context, Result};
use serde_json::json;
use std::path::Path;
use std::process::Command;

pub struct GitDiffTool;

impl GitDiffTool {
    pub fn name(&self) -> &str {
        "git_diff"
    }

    pub fn description(&self) -> &str {
        "Show git diff output. Can diff working tree, staged changes, or against a specific ref."
    }

    pub fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "ref": {
                    "type": "string",
                    "description": "Git ref to diff against (e.g. 'HEAD~1', 'main'). Use '--cached' for staged changes. Omit for unstaged changes."
                }
            }
        })
    }

    pub fn execute(&self, args: serde_json::Value, workspace: &Path) -> Result<String> {
        let git_ref = args.get("ref").and_then(|v| v.as_str());

        let mut cmd = Command::new("git");
        cmd.arg("diff").current_dir(workspace);

        if let Some(r) = git_ref {
            if r == "--cached" {
                cmd.arg("--cached");
            } else {
                cmd.arg(r);
            }
        }

        let output = cmd.output().context("failed to run git diff")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Ok(format!("git diff failed:\n{}", stderr));
        }

        if stdout.is_empty() {
            Ok("No changes.".to_string())
        } else {
            Ok(stdout.to_string())
        }
    }
}

pub struct GitLogTool;

impl GitLogTool {
    pub fn name(&self) -> &str {
        "git_log"
    }

    pub fn description(&self) -> &str {
        "Show recent git log entries in oneline format."
    }

    pub fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "count": {
                    "type": "integer",
                    "description": "Number of log entries to show (default: 20)"
                },
                "ref": {
                    "type": "string",
                    "description": "Git ref to start from (e.g. 'main', 'HEAD~5')"
                }
            }
        })
    }

    pub fn execute(&self, args: serde_json::Value, workspace: &Path) -> Result<String> {
        let count = args
            .get("count")
            .and_then(|v| v.as_u64())
            .unwrap_or(20);
        let git_ref = args.get("ref").and_then(|v| v.as_str());

        let mut cmd = Command::new("git");
        cmd.arg("log")
            .arg("--oneline")
            .arg(format!("-{}", count))
            .current_dir(workspace);

        if let Some(r) = git_ref {
            cmd.arg(r);
        }

        let output = cmd.output().context("failed to run git log")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Ok(format!("git log failed:\n{}", stderr));
        }

        if stdout.is_empty() {
            Ok("No commits found.".to_string())
        } else {
            Ok(stdout.to_string())
        }
    }
}
