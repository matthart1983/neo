use anyhow::{bail, Context, Result};
use serde_json::json;
use std::fs;
use std::path::Path;

pub struct ReadFileTool;

impl ReadFileTool {
    pub fn name(&self) -> &str {
        "read_file"
    }

    pub fn description(&self) -> &str {
        "Read the contents of a file. Returns line-numbered content. Supports optional line range."
    }

    pub fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read (relative to workspace or absolute)"
                },
                "start_line": {
                    "type": "integer",
                    "description": "First line to return (1-indexed, optional)"
                },
                "end_line": {
                    "type": "integer",
                    "description": "Last line to return (1-indexed, optional)"
                }
            },
            "required": ["path"]
        })
    }

    pub fn execute(&self, args: serde_json::Value, workspace: &Path) -> Result<String> {
        let path_str = args["path"]
            .as_str()
            .context("missing required parameter: path")?;

        let resolved = resolve_and_validate(path_str, workspace)?;

        let content = fs::read_to_string(&resolved)
            .with_context(|| format!("failed to read file: {}", resolved.display()))?;

        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();

        let start_line = args
            .get("start_line")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1);
        let end_line = args
            .get("end_line")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let start = start_line.saturating_sub(1).min(total);
        let end = end_line.unwrap_or_else(|| {
            if start_line == 1 {
                500.min(total)
            } else {
                total
            }
        }).min(total);

        if start >= end {
            return Ok(format!("(empty range: lines {}-{} of {} total)", start_line, end.max(start_line), total));
        }

        let mut output = String::new();
        for (i, line) in lines[start..end].iter().enumerate() {
            let line_num = start + i + 1;
            output.push_str(&format!("{}: {}\n", line_num, line));
        }

        if end < total && end_line.is_none() {
            output.push_str(&format!(
                "\n... ({} more lines, {} total. Use start_line/end_line to read more.)\n",
                total - end,
                total
            ));
        }

        Ok(output)
    }
}

pub fn resolve_and_validate(path_str: &str, workspace: &Path) -> Result<std::path::PathBuf> {
    let path = Path::new(path_str);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    };

    let canonical_workspace = workspace
        .canonicalize()
        .with_context(|| format!("cannot canonicalize workspace: {}", workspace.display()))?;
    let canonical_path = resolved
        .canonicalize()
        .with_context(|| format!("path does not exist: {}", resolved.display()))?;

    if !canonical_path.starts_with(&canonical_workspace) {
        bail!(
            "path {} is outside workspace {}",
            canonical_path.display(),
            canonical_workspace.display()
        );
    }

    Ok(canonical_path)
}

/// Like resolve_and_validate but the file doesn't need to exist yet.
/// Validates the parent directory is within workspace.
pub fn resolve_and_validate_new(path_str: &str, workspace: &Path) -> Result<std::path::PathBuf> {
    let path = Path::new(path_str);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    };

    let canonical_workspace = workspace
        .canonicalize()
        .with_context(|| format!("cannot canonicalize workspace: {}", workspace.display()))?;

    // For new files, check the parent directory
    let parent = resolved
        .parent()
        .context("path has no parent directory")?;

    // Parent might not exist yet either, walk up to find an existing ancestor
    let mut ancestor = parent.to_path_buf();
    loop {
        if ancestor.exists() {
            let canonical_ancestor = ancestor.canonicalize()?;
            if !canonical_ancestor.starts_with(&canonical_workspace) {
                bail!(
                    "path {} is outside workspace {}",
                    resolved.display(),
                    canonical_workspace.display()
                );
            }
            break;
        }
        if !ancestor.pop() {
            bail!("no existing ancestor directory for path: {}", resolved.display());
        }
    }

    Ok(resolved)
}
