use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeoConfig {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub budget: BudgetConfig,
    #[serde(default)]
    pub permissions: PermissionsConfig,
    #[serde(default)]
    pub workflow: WorkflowConfig,
    #[serde(default)]
    pub shell: ShellConfig,
    #[serde(default)]
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub context: ContextConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

impl Default for NeoConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig::default(),
            budget: BudgetConfig::default(),
            permissions: PermissionsConfig::default(),
            workflow: WorkflowConfig::default(),
            shell: ShellConfig::default(),
            providers: ProvidersConfig::default(),
            context: ContextConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    #[serde(default = "default_auto")]
    pub default_model: String,
    #[serde(default = "default_auto")]
    pub interactive_model: String,
    #[serde(default = "default_auto")]
    pub planning_model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            default_model: "auto".into(),
            interactive_model: "auto".into(),
            planning_model: "auto".into(),
            temperature: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    #[serde(default = "default_max_per_request")]
    pub max_per_request: f64,
    #[serde(default = "default_max_per_session")]
    pub max_per_session: f64,
    #[serde(default = "default_max_per_day")]
    pub max_per_day: f64,
    #[serde(default = "default_warn_at_percentage")]
    pub warn_at_percentage: u8,
    #[serde(default = "default_any")]
    pub preferred_cost_tier: String,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_per_request: 0.50,
            max_per_session: 5.00,
            max_per_day: 20.00,
            warn_at_percentage: 80,
            preferred_cost_tier: "any".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    Auto,
    Confirm,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionsConfig {
    #[serde(default = "Permission::auto")]
    pub read_tools: Permission,
    #[serde(default = "Permission::auto")]
    pub write_tools: Permission,
    #[serde(default = "Permission::confirm")]
    pub shell_tools: Permission,
    #[serde(default = "Permission::confirm")]
    pub git_write_tools: Permission,
    #[serde(default = "Permission::auto")]
    pub network_tools: Permission,
}

impl Permission {
    fn auto() -> Self {
        Permission::Auto
    }
    fn confirm() -> Self {
        Permission::Confirm
    }
}

impl Default for PermissionsConfig {
    fn default() -> Self {
        Self {
            read_tools: Permission::Auto,
            write_tools: Permission::Auto,
            shell_tools: Permission::Confirm,
            git_write_tools: Permission::Confirm,
            network_tools: Permission::Auto,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    #[serde(default = "default_implement")]
    pub default_strategy: String,
    #[serde(default = "default_true")]
    pub auto_test: bool,
    #[serde(default = "default_true")]
    pub auto_lint: bool,
    #[serde(default)]
    pub auto_format: bool,
    #[serde(default = "default_review_threshold")]
    pub review_threshold: String,
    #[serde(default = "default_max_review_cycles")]
    pub max_review_cycles: u8,
    #[serde(default = "default_true")]
    pub parallel_agents: bool,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            default_strategy: "implement".into(),
            auto_test: true,
            auto_lint: true,
            auto_format: false,
            review_threshold: "multi_file".into(),
            max_review_cycles: 3,
            parallel_agents: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    #[serde(default = "default_deny_patterns")]
    pub deny_patterns: Vec<String>,
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    #[serde(default = "default_shell_timeout")]
    pub timeout_seconds: u64,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            deny_patterns: vec![
                "rm -rf /".into(),
                "sudo".into(),
                "mkfs".into(),
                "dd if=".into(),
                "> /dev/sda".into(),
                "chmod -R 777 /".into(),
            ],
            allowed_commands: Vec::new(),
            timeout_seconds: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub openrouter: OpenRouterConfig,
    #[serde(default)]
    pub ollama: OllamaConfig,
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            openrouter: OpenRouterConfig::default(),
            ollama: OllamaConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,
    #[serde(default = "default_openrouter_base_url")]
    pub base_url: String,
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
    #[serde(default = "default_provider_timeout")]
    pub timeout_seconds: u64,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            api_key_env: "OPENROUTER_API_KEY".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            max_retries: 3,
            timeout_seconds: 120,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_ollama_endpoint")]
    pub endpoint: String,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default = "default_fallback")]
    pub priority: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://localhost:11434".into(),
            models: Vec::new(),
            priority: "fallback".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    #[serde(default = "default_max_file_lines")]
    pub max_file_lines: usize,
    #[serde(default = "default_summary_threshold")]
    pub summary_threshold: usize,
    #[serde(default = "default_true")]
    pub pin_system_messages: bool,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_file_lines: 500,
            summary_threshold: 50000,
            pin_system_messages: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_auto")]
    pub color: String,
    #[serde(default = "default_true")]
    pub spinner: bool,
    #[serde(default = "default_true")]
    pub streaming: bool,
    #[serde(default = "default_true")]
    pub show_cost: bool,
    #[serde(default = "default_true")]
    pub show_model: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            color: "auto".into(),
            spinner: true,
            streaming: true,
            show_cost: true,
            show_model: true,
        }
    }
}

// Serde default helper functions
fn default_auto() -> String {
    "auto".into()
}
fn default_any() -> String {
    "any".into()
}
fn default_implement() -> String {
    "implement".into()
}
fn default_fallback() -> String {
    "fallback".into()
}
fn default_true() -> bool {
    true
}
fn default_temperature() -> f32 {
    0.3
}
fn default_max_per_request() -> f64 {
    0.50
}
fn default_max_per_session() -> f64 {
    5.00
}
fn default_max_per_day() -> f64 {
    20.00
}
fn default_warn_at_percentage() -> u8 {
    80
}
fn default_review_threshold() -> String {
    "multi_file".into()
}
fn default_max_review_cycles() -> u8 {
    3
}
fn default_shell_timeout() -> u64 {
    30
}
fn default_deny_patterns() -> Vec<String> {
    vec![
        "rm -rf /".into(),
        "sudo".into(),
        "mkfs".into(),
        "dd if=".into(),
        "> /dev/sda".into(),
        "chmod -R 777 /".into(),
    ]
}
fn default_api_key_env() -> String {
    "OPENROUTER_API_KEY".into()
}
fn default_openrouter_base_url() -> String {
    "https://openrouter.ai/api/v1".into()
}
fn default_max_retries() -> u8 {
    3
}
fn default_provider_timeout() -> u64 {
    120
}
fn default_ollama_endpoint() -> String {
    "http://localhost:11434".into()
}
fn default_max_file_lines() -> usize {
    500
}
fn default_summary_threshold() -> usize {
    50000
}
