mod bash;
mod create_file;
mod edit_file;
mod git;
mod glob_tool;
mod grep;
mod read_file;

pub use bash::BashTool;
pub use create_file::CreateFileTool;
pub use edit_file::EditFileTool;
pub use git::{GitDiffTool, GitLogTool};
pub use glob_tool::GlobTool;
pub use grep::GrepTool;
pub use read_file::ReadFileTool;

use crate::api::types::{FunctionDef, Tool};
use crate::config::types::ShellConfig;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub enum ToolKind {
    ReadFile(ReadFileTool),
    EditFile(EditFileTool),
    CreateFile(CreateFileTool),
    Grep(GrepTool),
    Glob(GlobTool),
    Bash(BashTool),
    GitDiff(GitDiffTool),
    GitLog(GitLogTool),
}

impl ToolKind {
    pub fn name(&self) -> &str {
        match self {
            ToolKind::ReadFile(t) => t.name(),
            ToolKind::EditFile(t) => t.name(),
            ToolKind::CreateFile(t) => t.name(),
            ToolKind::Grep(t) => t.name(),
            ToolKind::Glob(t) => t.name(),
            ToolKind::Bash(t) => t.name(),
            ToolKind::GitDiff(t) => t.name(),
            ToolKind::GitLog(t) => t.name(),
        }
    }

    pub fn description(&self) -> &str {
        match self {
            ToolKind::ReadFile(t) => t.description(),
            ToolKind::EditFile(t) => t.description(),
            ToolKind::CreateFile(t) => t.description(),
            ToolKind::Grep(t) => t.description(),
            ToolKind::Glob(t) => t.description(),
            ToolKind::Bash(t) => t.description(),
            ToolKind::GitDiff(t) => t.description(),
            ToolKind::GitLog(t) => t.description(),
        }
    }

    pub fn parameters_schema(&self) -> serde_json::Value {
        match self {
            ToolKind::ReadFile(t) => t.parameters_schema(),
            ToolKind::EditFile(t) => t.parameters_schema(),
            ToolKind::CreateFile(t) => t.parameters_schema(),
            ToolKind::Grep(t) => t.parameters_schema(),
            ToolKind::Glob(t) => t.parameters_schema(),
            ToolKind::Bash(t) => t.parameters_schema(),
            ToolKind::GitDiff(t) => t.parameters_schema(),
            ToolKind::GitLog(t) => t.parameters_schema(),
        }
    }

    pub fn execute(&self, args: serde_json::Value, workspace: &Path) -> Result<String> {
        match self {
            ToolKind::ReadFile(t) => t.execute(args, workspace),
            ToolKind::EditFile(t) => t.execute(args, workspace),
            ToolKind::CreateFile(t) => t.execute(args, workspace),
            ToolKind::Grep(t) => t.execute(args, workspace),
            ToolKind::Glob(t) => t.execute(args, workspace),
            ToolKind::Bash(t) => t.execute(args, workspace),
            ToolKind::GitDiff(t) => t.execute(args, workspace),
            ToolKind::GitLog(t) => t.execute(args, workspace),
        }
    }

    fn to_api_tool(&self) -> Tool {
        Tool {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: self.name().to_string(),
                description: self.description().to_string(),
                parameters: Some(self.parameters_schema()),
            },
        }
    }
}

pub struct ToolRegistry {
    tools: HashMap<String, ToolKind>,
    workspace: PathBuf,
}

impl ToolRegistry {
    pub fn new(workspace: PathBuf, shell_config: &ShellConfig) -> Self {
        let mut tools = HashMap::new();

        let all_tools: Vec<ToolKind> = vec![
            ToolKind::ReadFile(ReadFileTool),
            ToolKind::EditFile(EditFileTool),
            ToolKind::CreateFile(CreateFileTool),
            ToolKind::Grep(GrepTool),
            ToolKind::Glob(GlobTool),
            ToolKind::Bash(BashTool::new(shell_config)),
            ToolKind::GitDiff(GitDiffTool),
            ToolKind::GitLog(GitLogTool),
        ];

        for tool in all_tools {
            tools.insert(tool.name().to_string(), tool);
        }

        Self { tools, workspace }
    }

    pub fn get(&self, name: &str) -> Option<&ToolKind> {
        self.tools.get(name)
    }

    pub fn execute(&self, name: &str, args: serde_json::Value) -> Result<String> {
        match self.tools.get(name) {
            Some(tool) => tool.execute(args, &self.workspace),
            None => anyhow::bail!("unknown tool: {}", name),
        }
    }

    pub fn to_api_tools(&self) -> Vec<Tool> {
        self.tools.values().map(|t| t.to_api_tool()).collect()
    }

    pub fn to_api_tools_for(&self, names: &[&str]) -> Vec<Tool> {
        names
            .iter()
            .filter_map(|name| self.tools.get(*name))
            .map(|t| t.to_api_tool())
            .collect()
    }
}
