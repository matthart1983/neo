use anyhow::{Context, Result};
use glob::glob as glob_iter;
use serde_json::json;
use std::path::Path;

pub struct GlobTool;

impl GlobTool {
    pub fn name(&self) -> &str {
        "glob"
    }

    pub fn description(&self) -> &str {
        "Find files matching a glob pattern. Returns matching paths, one per line. Max 200 results."
    }

    pub fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g. '**/*.rs', 'src/**/*.ts')"
                }
            },
            "required": ["pattern"]
        })
    }

    pub fn execute(&self, args: serde_json::Value, workspace: &Path) -> Result<String> {
        let pattern_str = args["pattern"]
            .as_str()
            .context("missing required parameter: pattern")?;

        let full_pattern = if Path::new(pattern_str).is_absolute() {
            pattern_str.to_string()
        } else {
            format!("{}/{}", workspace.display(), pattern_str)
        };

        let mut results = Vec::new();
        const MAX_RESULTS: usize = 200;

        for entry in glob_iter(&full_pattern)? {
            if results.len() >= MAX_RESULTS {
                break;
            }
            match entry {
                Ok(path) => {
                    let display = path
                        .strip_prefix(workspace)
                        .unwrap_or(&path)
                        .display()
                        .to_string();
                    results.push(display);
                }
                Err(_) => continue,
            }
        }

        if results.is_empty() {
            Ok("No files matched.".to_string())
        } else {
            let mut output = results.join("\n");
            if results.len() >= MAX_RESULTS {
                output.push_str(&format!("\n\n(truncated at {} results)", MAX_RESULTS));
            }
            Ok(output)
        }
    }
}
