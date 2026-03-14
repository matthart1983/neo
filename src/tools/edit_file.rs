use anyhow::{bail, Context, Result};
use serde_json::json;
use std::fs;
use std::path::Path;

use super::read_file::resolve_and_validate;

pub struct EditFileTool;

impl EditFileTool {
    pub fn name(&self) -> &str {
        "edit_file"
    }

    pub fn description(&self) -> &str {
        "Edit an existing file by replacing an exact string match. The old_str must exist in the file. If replace_all is false (default), old_str must appear exactly once."
    }

    pub fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_str": {
                    "type": "string",
                    "description": "The exact string to find and replace"
                },
                "new_str": {
                    "type": "string",
                    "description": "The replacement string"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences (default: false)"
                }
            },
            "required": ["path", "old_str", "new_str"]
        })
    }

    pub fn execute(&self, args: serde_json::Value, workspace: &Path) -> Result<String> {
        let path_str = args["path"]
            .as_str()
            .context("missing required parameter: path")?;
        let old_str = args["old_str"]
            .as_str()
            .context("missing required parameter: old_str")?;
        let new_str = args["new_str"]
            .as_str()
            .context("missing required parameter: new_str")?;
        let replace_all = args
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let resolved = resolve_and_validate(path_str, workspace)?;

        let content = fs::read_to_string(&resolved)
            .with_context(|| format!("failed to read file: {}", resolved.display()))?;

        let count = content.matches(old_str).count();
        if count == 0 {
            bail!("old_str not found in {}", resolved.display());
        }
        if !replace_all && count > 1 {
            bail!(
                "old_str appears {} times in {}. Use replace_all=true or provide more context to make it unique.",
                count,
                resolved.display()
            );
        }

        let new_content = if replace_all {
            content.replace(old_str, new_str)
        } else {
            content.replacen(old_str, new_str, 1)
        };

        fs::write(&resolved, &new_content)
            .with_context(|| format!("failed to write file: {}", resolved.display()))?;

        // Compute affected line numbers
        let old_lines: Vec<&str> = old_str.lines().collect();
        let new_lines: Vec<&str> = new_str.lines().collect();

        // Find first occurrence line number
        let prefix = content.split(old_str).next().unwrap_or("");
        let start_line = prefix.lines().count();

        let replacements = if replace_all { count } else { 1 };

        let mut summary = format!(
            "Edited {}: {} replacement(s) starting at line {}\n",
            path_str, replacements, start_line
        );
        summary.push_str(&format!(
            "  removed {} line(s), inserted {} line(s)\n",
            old_lines.len(),
            new_lines.len()
        ));

        // Show brief diff
        summary.push_str("--- old\n");
        for line in old_lines.iter().take(5) {
            summary.push_str(&format!("- {}\n", line));
        }
        if old_lines.len() > 5 {
            summary.push_str(&format!("  ... ({} more lines)\n", old_lines.len() - 5));
        }
        summary.push_str("+++ new\n");
        for line in new_lines.iter().take(5) {
            summary.push_str(&format!("+ {}\n", line));
        }
        if new_lines.len() > 5 {
            summary.push_str(&format!("  ... ({} more lines)\n", new_lines.len() - 5));
        }

        Ok(summary)
    }
}
