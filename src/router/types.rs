#[derive(Debug, Clone)]
pub struct TaskProfile {
    pub category: TaskCategory,
    pub estimated_complexity: Complexity,
    pub context_tokens: usize,
    pub output_expectation: OutputSize,
    pub latency_sensitivity: Latency,
    pub requires_tool_use: bool,
    pub language: Option<String>,
}

impl Default for TaskProfile {
    fn default() -> Self {
        Self {
            category: TaskCategory::Conversation,
            estimated_complexity: Complexity::Medium,
            context_tokens: 4000,
            output_expectation: OutputSize::Medium,
            latency_sensitivity: Latency::Interactive,
            requires_tool_use: false,
            language: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskCategory {
    CodeGeneration,
    CodeEdit,
    Review,
    Planning,
    Debugging,
    Search,
    Documentation,
    TestGeneration,
    Conversation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Complexity {
    Low,
    Medium,
    High,
    Extreme,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputSize {
    Short,
    Medium,
    Long,
    VeryLong,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Latency {
    Realtime,
    Interactive,
    Batch,
}
