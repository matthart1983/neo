use std::env;
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::agents::{AgentExecutor, AgentId};
use crate::api::{Message, ModelInfo, OpenRouterClient, Role};
use crate::config::{get_api_key, NeoConfig};
use crate::context::ContextManager;
use crate::router::{default_capabilities, ModelRouter};
use crate::session::{SessionManager, SessionStats};
use crate::tools::ToolRegistry;

pub struct Orchestrator {
    executor: AgentExecutor,
    session: SessionManager,
    config: NeoConfig,
}

pub struct OrchestratorResponse {
    pub content: String,
    pub agent_used: AgentId,
    pub model_used: String,
    pub tokens_in: usize,
    pub tokens_out: usize,
    pub cost_usd: f64,
    pub session_cost: f64,
}

impl Orchestrator {
    pub fn new(config: NeoConfig) -> Result<Self> {
        let api_key = get_api_key(&config)
            .context("OPENROUTER_API_KEY is not set. Export it or add it to your environment:\n  export OPENROUTER_API_KEY=sk-or-...")?;

        let client = Arc::new(
            OpenRouterClient::new(&config.providers.openrouter, api_key)
                .context("failed to create OpenRouter client")?,
        );

        let workspace = env::current_dir().unwrap_or_default();
        let tool_registry = Arc::new(ToolRegistry::new(workspace.clone(), &config.shell));

        let capabilities = default_capabilities();

        // We build the router with capabilities and an empty model list initially;
        // live model fetching happens async in `init()`.
        let router = Arc::new(ModelRouter::new(
            capabilities.clone(),
            fallback_models(&capabilities),
            config.budget.clone(),
        ));

        let context_manager = Arc::new(ContextManager::new(config.context.clone()));
        let executor = AgentExecutor::new(client, tool_registry, router, context_manager);

        let mut session = SessionManager::new()?;
        session.start_thread(&workspace);

        Ok(Self {
            executor,
            session,
            config,
        })
    }

    /// Optionally fetch live models from OpenRouter. Call after construction.
    /// If the network call fails we silently keep the fallback models.
    pub async fn init(&mut self) {
        // Re-create executor with live model list if possible.
        // Because ModelRouter is behind Arc, we rebuild the whole stack.
        // This is a one-time startup cost.
        let api_key = match get_api_key(&self.config) {
            Some(k) => k,
            None => return,
        };
        let client = match OpenRouterClient::new(&self.config.providers.openrouter, api_key) {
            Ok(c) => Arc::new(c),
            Err(_) => return,
        };

        let models = match client.list_models().await {
            Ok(m) => m,
            Err(_) => return, // keep fallback
        };

        let capabilities = default_capabilities();
        let workspace = env::current_dir().unwrap_or_default();
        let tool_registry = Arc::new(ToolRegistry::new(workspace, &self.config.shell));
        let router = Arc::new(ModelRouter::new(
            capabilities,
            models,
            self.config.budget.clone(),
        ));
        let context_manager = Arc::new(ContextManager::new(self.config.context.clone()));

        self.executor = AgentExecutor::new(client, tool_registry, router, context_manager);
    }

    pub async fn handle_message(&mut self, input: &str) -> Result<OrchestratorResponse> {
        let user_msg = Message {
            role: Role::User,
            content: Some(input.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };
        self.session.add_message(user_msg);

        let agent_id = if is_simple_request(input) {
            AgentId::Coder
        } else {
            self.classify_request(input).await
        };

        let messages = self.current_messages();
        let result = self.executor.run(&agent_id, messages).await?;

        let assistant_msg = Message {
            role: Role::Assistant,
            content: Some(result.content.clone()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };
        self.session.add_message(assistant_msg);
        self.session.record_cost(
            &result.model_used,
            result.cost_usd,
            result.tokens_in,
            result.tokens_out,
        );
        let _ = self.session.save_thread();

        Ok(OrchestratorResponse {
            content: result.content,
            agent_used: agent_id,
            model_used: result.model_used,
            tokens_in: result.tokens_in,
            tokens_out: result.tokens_out,
            cost_usd: result.cost_usd,
            session_cost: self.session.current_stats().total_cost,
        })
    }

    pub async fn handle_command(
        &mut self,
        command: &str,
        args: &str,
    ) -> Result<OrchestratorResponse> {
        let agent_id = match command {
            "review" => AgentId::Reviewer,
            "plan" => AgentId::Planner,
            "debug" => AgentId::Debugger,
            "test" => AgentId::Tester,
            "doc" => AgentId::Documenter,
            _ => AgentId::Coder,
        };

        let prompt = if args.is_empty() {
            format!("Please {} the current project.", command)
        } else {
            args.to_string()
        };

        let user_msg = Message {
            role: Role::User,
            content: Some(prompt),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };
        self.session.add_message(user_msg);

        let messages = self.current_messages();
        let result = self.executor.run(&agent_id, messages).await?;

        let assistant_msg = Message {
            role: Role::Assistant,
            content: Some(result.content.clone()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };
        self.session.add_message(assistant_msg);
        self.session.record_cost(
            &result.model_used,
            result.cost_usd,
            result.tokens_in,
            result.tokens_out,
        );
        let _ = self.session.save_thread();

        Ok(OrchestratorResponse {
            content: result.content,
            agent_used: agent_id,
            model_used: result.model_used,
            tokens_in: result.tokens_in,
            tokens_out: result.tokens_out,
            cost_usd: result.cost_usd,
            session_cost: self.session.current_stats().total_cost,
        })
    }

    pub fn session_stats(&self) -> &SessionStats {
        self.session.current_stats()
    }

    pub fn session_manager(&self) -> &SessionManager {
        &self.session
    }

    pub fn session_manager_mut(&mut self) -> &mut SessionManager {
        &mut self.session
    }

    pub fn config(&self) -> &NeoConfig {
        &self.config
    }

    fn current_messages(&self) -> Vec<Message> {
        self.session
            .current_thread_messages()
            .cloned()
            .unwrap_or_default()
    }

    async fn classify_request(&self, input: &str) -> AgentId {
        // Use the Router agent to classify. If it fails, fall back to Coder.
        let messages = vec![Message {
            role: Role::User,
            content: Some(input.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }];

        let result = match self.executor.run(&AgentId::Router, messages).await {
            Ok(r) => r,
            Err(_) => return AgentId::Coder,
        };

        parse_router_response(&result.content)
    }
}

fn is_simple_request(input: &str) -> bool {
    if input.len() >= 200 {
        return false;
    }
    let complex_words = ["plan", "review", "test", "debug", "document", "analyze", "refactor"];
    let lower = input.to_lowercase();
    !complex_words.iter().any(|w| lower.contains(w))
}

fn parse_router_response(content: &str) -> AgentId {
    // Try to parse JSON response from router
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(agent) = v.get("agent").and_then(|a| a.as_str()) {
            return match agent {
                "planner" => AgentId::Planner,
                "reviewer" => AgentId::Reviewer,
                "debugger" => AgentId::Debugger,
                "tester" => AgentId::Tester,
                "documenter" => AgentId::Documenter,
                "oracle" => AgentId::Oracle,
                _ => AgentId::Coder,
            };
        }
    }

    // Fallback: look for agent name in raw text
    let lower = content.to_lowercase();
    if lower.contains("planner") {
        AgentId::Planner
    } else if lower.contains("reviewer") {
        AgentId::Reviewer
    } else if lower.contains("debugger") {
        AgentId::Debugger
    } else if lower.contains("tester") {
        AgentId::Tester
    } else if lower.contains("documenter") {
        AgentId::Documenter
    } else {
        AgentId::Coder
    }
}

/// Build dummy ModelInfo entries from capabilities for offline fallback.
fn fallback_models(capabilities: &[crate::router::capabilities::ModelCapability]) -> Vec<ModelInfo> {
    capabilities
        .iter()
        .map(|cap| ModelInfo {
            id: cap.model_id.clone(),
            name: cap.model_id.clone(),
            context_length: cap.context,
            pricing: None,
            top_provider: None,
        })
        .collect()
}
