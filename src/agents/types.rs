use crate::router::types::TaskProfile;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentId {
    Router,
    Planner,
    Coder,
    Reviewer,
    Debugger,
    Tester,
    Documenter,
    Oracle,
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            AgentId::Router => "router",
            AgentId::Planner => "planner",
            AgentId::Coder => "coder",
            AgentId::Reviewer => "reviewer",
            AgentId::Debugger => "debugger",
            AgentId::Tester => "tester",
            AgentId::Documenter => "documenter",
            AgentId::Oracle => "oracle",
        };
        write!(f, "{}", name)
    }
}

pub struct AgentConfig {
    pub id: AgentId,
    pub name: &'static str,
    pub description: &'static str,
    pub system_prompt: &'static str,
    pub available_tools: Vec<&'static str>,
    pub default_profile: TaskProfile,
    pub max_iterations: usize,
    pub temperature: f32,
}
