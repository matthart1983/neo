use std::sync::Arc;

use anyhow::{Context, Result};

use crate::api::client::OpenRouterClient;
use crate::api::types::{ChatRequest, Message, Role};
use crate::router::selector::ModelRouter;
use crate::tools::ToolRegistry;

use super::definitions::get_agent_config;
use super::types::AgentId;

pub struct AgentExecutor {
    client: Arc<OpenRouterClient>,
    tool_registry: Arc<ToolRegistry>,
    router: Arc<ModelRouter>,
}

pub struct AgentResult {
    pub content: String,
    pub model_used: String,
    pub tokens_in: usize,
    pub tokens_out: usize,
    pub cost_usd: f64,
    pub tool_calls_made: usize,
    pub iterations: usize,
}

impl AgentExecutor {
    pub fn new(
        client: Arc<OpenRouterClient>,
        tool_registry: Arc<ToolRegistry>,
        router: Arc<ModelRouter>,
    ) -> Self {
        Self {
            client,
            tool_registry,
            router,
        }
    }

    pub async fn run(
        &self,
        agent_id: &AgentId,
        messages: Vec<Message>,
    ) -> Result<AgentResult> {
        let config = get_agent_config(agent_id);

        let selected = self
            .router
            .select_model(&config.default_profile)
            .context("failed to select model for agent")?;

        let model_id = selected.model_id.clone();

        let system_msg = Message {
            role: Role::System,
            content: Some(config.system_prompt.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };

        let mut all_messages = vec![system_msg];
        all_messages.extend(messages);

        let tools = if config.available_tools.is_empty() {
            None
        } else {
            let api_tools = self.tool_registry.to_api_tools_for(&config.available_tools);
            if api_tools.is_empty() {
                None
            } else {
                Some(api_tools)
            }
        };

        let mut total_tokens_in: usize = 0;
        let mut total_tokens_out: usize = 0;
        let mut total_tool_calls: usize = 0;
        let mut iterations: usize = 0;

        loop {
            iterations += 1;

            let request = ChatRequest {
                model: model_id.clone(),
                messages: all_messages.clone(),
                tools: tools.clone(),
                stream: false,
                temperature: Some(config.temperature),
                max_tokens: None,
            };

            let response = self
                .client
                .chat(&request)
                .await
                .context("agent chat request failed")?;

            if let Some(usage) = &response.usage {
                total_tokens_in += usage.prompt_tokens;
                total_tokens_out += usage.completion_tokens;
            }

            let choice = response
                .choices
                .into_iter()
                .next()
                .context("no choices in response")?;

            let assistant_msg = choice.message;

            if let Some(ref tool_calls) = assistant_msg.tool_calls {
                if !tool_calls.is_empty() {
                    total_tool_calls += tool_calls.len();

                    all_messages.push(assistant_msg.clone());

                    for tc in tool_calls {
                        let args: serde_json::Value =
                            serde_json::from_str(&tc.function.arguments)
                                .unwrap_or(serde_json::Value::Null);

                        let result = self
                            .tool_registry
                            .execute(&tc.function.name, args)
                            .unwrap_or_else(|e| format!("Tool error: {}", e));

                        let tool_msg = Message {
                            role: Role::Tool,
                            content: Some(result),
                            tool_calls: None,
                            tool_call_id: Some(tc.id.clone()),
                            name: Some(tc.function.name.clone()),
                        };

                        all_messages.push(tool_msg);
                    }

                    if iterations >= config.max_iterations {
                        let content = format!(
                            "[Agent {} reached maximum iterations ({})]. Last tool calls were executed but the agent was stopped.",
                            config.name, config.max_iterations
                        );
                        return Ok(AgentResult {
                            content,
                            model_used: model_id,
                            tokens_in: total_tokens_in,
                            tokens_out: total_tokens_out,
                            cost_usd: 0.0,
                            tool_calls_made: total_tool_calls,
                            iterations,
                        });
                    }

                    continue;
                }
            }

            let content = assistant_msg
                .content
                .unwrap_or_default();

            return Ok(AgentResult {
                content,
                model_used: model_id,
                tokens_in: total_tokens_in,
                tokens_out: total_tokens_out,
                cost_usd: 0.0,
                tool_calls_made: total_tool_calls,
                iterations,
            });
        }
    }
}
