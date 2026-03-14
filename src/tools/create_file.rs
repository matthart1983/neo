use anyhow::{Context, Result};
use serde_json::json;
use std::fs;
use std::path::Path;

use super::read_file::resolve_and_validate_new;

pub struct CreateFileTool;

impl CreateFileTool {
    pub fn name(&self) -> &str {
        "create_file"
    }

    pub fn description(&self) -> &str {
        "Create a new file with the given content. Creates parent directories if needed."
    }

    pub fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path for the new file"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    pub fn execute(&self, args: serde_json::Value, workspace: &Path) -> Result<String> {
        let path_str = args["path"]
            .as_str()
            .context("missing required parameter: path")?;
        let content = args["content"]
            .as_str()
            .context("missing required parameter: content")?;

        let resolved = resolve_and_validate_new(path_str, workspace)?;

        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directories: {}", parent.display()))?;
        }

        fs::write(&resolved, content)
            .with_context(|| format!("failed to write file: {}", resolved.display()))?;

        let line_count = content.lines().count();
        Ok(format!(
            "Created {} ({} lines)",
            path_str, line_count
        ))
    }
}
