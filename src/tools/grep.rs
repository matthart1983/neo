use anyhow::{Context, Result};
use ignore::WalkBuilder;
use regex::RegexBuilder;
use serde_json::json;
use std::fs;
use std::path::Path;

pub struct GrepTool;

impl GrepTool {
    pub fn name(&self) -> &str {
        "grep"
    }

    pub fn description(&self) -> &str {
        "Search for a regex pattern in files. Returns matching lines as file:line:content. Respects .gitignore. Max 100 matches, 10 per file."
    }

    pub fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (default: workspace root)"
                },
                "glob_pattern": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. '*.rs')"
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Case-sensitive search (default: false)"
                }
            },
            "required": ["pattern"]
        })
    }

    pub fn execute(&self, args: serde_json::Value, workspace: &Path) -> Result<String> {
        let pattern_str = args["pattern"]
            .as_str()
            .context("missing required parameter: pattern")?;
        let search_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .map(|p| {
                let path = Path::new(p);
                if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    workspace.join(p)
                }
            })
            .unwrap_or_else(|| workspace.to_path_buf());
        let glob_pattern = args.get("glob_pattern").and_then(|v| v.as_str());
        let case_sensitive = args
            .get("case_sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let re = RegexBuilder::new(pattern_str)
            .case_insensitive(!case_sensitive)
            .build()
            .with_context(|| format!("invalid regex: {}", pattern_str))?;

        let mut results = Vec::new();
        let mut total_matches = 0;
        const MAX_TOTAL: usize = 100;
        const MAX_PER_FILE: usize = 10;
        const MAX_LINE_LEN: usize = 200;

        let mut walker = WalkBuilder::new(&search_path);
        walker.hidden(false).git_ignore(true);

        if let Some(glob) = glob_pattern {
            let mut types = ignore::types::TypesBuilder::new();
            types.add("custom", glob)?;
            types.select("custom");
            walker.types(types.build()?);
        }

        for entry in walker.build() {
            if total_matches >= MAX_TOTAL {
                break;
            }
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }

            let file_path = entry.path();
            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue, // skip binary/unreadable files
            };

            let mut file_matches = 0;
            let display_path = file_path
                .strip_prefix(workspace)
                .unwrap_or(file_path)
                .display();

            for (line_num, line) in content.lines().enumerate() {
                if total_matches >= MAX_TOTAL || file_matches >= MAX_PER_FILE {
                    break;
                }
                if re.is_match(line) {
                    let truncated = if line.len() > MAX_LINE_LEN {
                        format!("{}...", &line[..MAX_LINE_LEN])
                    } else {
                        line.to_string()
                    };
                    results.push(format!("{}:{}:{}", display_path, line_num + 1, truncated));
                    total_matches += 1;
                    file_matches += 1;
                }
            }
        }

        if results.is_empty() {
            Ok("No matches found.".to_string())
        } else {
            let mut output = results.join("\n");
            if total_matches >= MAX_TOTAL {
                output.push_str(&format!("\n\n(results truncated at {} matches)", MAX_TOTAL));
            }
            Ok(output)
        }
    }
}
