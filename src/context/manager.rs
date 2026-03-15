use crate::api::types::{Message, Role};
use crate::config::types::ContextConfig;

/// Manages context window sizing: estimates token usage, truncates tool results,
/// summarises old messages, and enforces a token budget before each API call.
pub struct ContextManager {
    config: ContextConfig,
}

/// Result of preparing messages for an API call.
pub struct PreparedContext {
    pub messages: Vec<Message>,
    pub estimated_tokens: usize,
    pub messages_summarised: usize,
    pub tool_results_truncated: usize,
}

impl ContextManager {
    pub fn new(config: ContextConfig) -> Self {
        Self { config }
    }

    /// Prepare messages to fit within `model_context_limit` tokens.
    ///
    /// Strategy (applied in order):
    /// 1. Truncate large tool results in older messages
    /// 2. Collapse old tool call/result pairs into summaries
    /// 3. Summarise oldest user/assistant exchanges into a context block
    /// 4. Drop oldest messages if still over budget
    ///
    /// The system message (index 0) and the last `preserve_recent` messages
    /// are never modified.
    pub fn prepare(
        &self,
        messages: &[Message],
        model_context_limit: usize,
        max_output_tokens: usize,
    ) -> PreparedContext {
        let preserve_recent = 6; // keep last N messages verbatim
        let token_budget = model_context_limit.saturating_sub(max_output_tokens);

        let mut msgs = messages.to_vec();
        let mut tool_results_truncated = 0;
        let mut messages_summarised = 0;

        // --- Phase 1: Truncate large tool results in non-recent messages ---
        let safe_end = msgs.len().saturating_sub(preserve_recent);
        for msg in msgs.iter_mut().take(safe_end) {
            if msg.role == Role::Tool {
                if let Some(ref content) = msg.content {
                    if estimate_tokens(content) > 800 {
                        let truncated = truncate_tool_result(content, 600);
                        msg.content = Some(truncated);
                        tool_results_truncated += 1;
                    }
                }
            }
        }

        // --- Phase 2: If still over budget, collapse old tool call/result pairs ---
        let estimated = estimate_messages_tokens(&msgs);
        if estimated > token_budget {
            msgs = collapse_tool_pairs(msgs, preserve_recent);
            let new_est = estimate_messages_tokens(&msgs);
            if new_est < estimated {
                messages_summarised += (estimated - new_est) / 100; // rough count
            }
        }

        // --- Phase 3: If still over, summarise oldest user/assistant exchanges ---
        let estimated = estimate_messages_tokens(&msgs);
        if estimated > token_budget && msgs.len() > preserve_recent + 2 {
            msgs = summarise_old_exchanges(msgs, preserve_recent, token_budget);
            messages_summarised += 1;
        }

        // --- Phase 4: Hard drop oldest messages (except system) if still over ---
        loop {
            let estimated = estimate_messages_tokens(&msgs);
            if estimated <= token_budget || msgs.len() <= preserve_recent + 1 {
                break;
            }
            // Remove the message right after system (index 1)
            if msgs.len() > 1 {
                msgs.remove(1);
                messages_summarised += 1;
            } else {
                break;
            }
        }

        let estimated_tokens = estimate_messages_tokens(&msgs);

        PreparedContext {
            messages: msgs,
            estimated_tokens,
            messages_summarised,
            tool_results_truncated,
        }
    }

    /// Convenience: prepare using the configured summary_threshold as the limit.
    pub fn prepare_with_defaults(
        &self,
        messages: &[Message],
        model_context_limit: usize,
    ) -> PreparedContext {
        // Use the configured threshold if it's smaller than the model limit
        let effective_limit = self.config.summary_threshold.min(model_context_limit);
        self.prepare(messages, effective_limit, 1024)
    }

    /// Quick check: does the message list likely exceed the budget?
    pub fn exceeds_budget(&self, messages: &[Message], model_context_limit: usize) -> bool {
        let effective = self.config.summary_threshold.min(model_context_limit);
        estimate_messages_tokens(messages) > effective.saturating_sub(1024)
    }

    pub fn config(&self) -> &ContextConfig {
        &self.config
    }
}

// ---------------------------------------------------------------------------
// Token estimation
// ---------------------------------------------------------------------------

/// Estimate token count for a single string.
/// Uses the ~4 chars per token heuristic (conservative for English + code).
pub fn estimate_tokens(text: &str) -> usize {
    // Every message has ~4 tokens of overhead (role, delimiters)
    (text.len() + 3) / 4
}

/// Estimate total tokens for a message list.
pub fn estimate_messages_tokens(messages: &[Message]) -> usize {
    let mut total: usize = 0;
    for msg in messages {
        // Base overhead per message
        total += 4;
        if let Some(ref content) = msg.content {
            total += estimate_tokens(content);
        }
        if let Some(ref tool_calls) = msg.tool_calls {
            for tc in tool_calls {
                total += estimate_tokens(&tc.function.name);
                total += estimate_tokens(&tc.function.arguments);
                total += 8; // overhead for tool call structure
            }
        }
        if let Some(ref name) = msg.name {
            total += estimate_tokens(name);
        }
    }
    total
}

// ---------------------------------------------------------------------------
// Truncation and summarisation helpers
// ---------------------------------------------------------------------------

/// Truncate a tool result to approximately `max_tokens` tokens,
/// keeping the first and last portions for context.
fn truncate_tool_result(content: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens * 4;
    if content.len() <= max_chars {
        return content.to_string();
    }

    let keep_start = max_chars * 2 / 3;
    let keep_end = max_chars / 3;
    let start = &content[..keep_start.min(content.len())];
    let end_start = content.len().saturating_sub(keep_end);
    let end = &content[end_start..];
    let omitted_lines = content[keep_start..end_start].matches('\n').count();

    format!(
        "{}\n\n[... {} lines omitted ({} chars) ...]\n\n{}",
        start.trim_end(),
        omitted_lines,
        content.len() - keep_start - keep_end,
        end.trim_start()
    )
}

/// Collapse old tool-call + tool-result pairs into a single summary message.
/// Keeps system message and the last `preserve_recent` messages intact.
fn collapse_tool_pairs(messages: Vec<Message>, preserve_recent: usize) -> Vec<Message> {
    if messages.len() <= preserve_recent + 1 {
        return messages;
    }

    let split_point = messages.len().saturating_sub(preserve_recent);
    let old = &messages[1..split_point]; // skip system (index 0)
    let recent = &messages[split_point..];

    let mut collapsed: Vec<Message> = vec![messages[0].clone()]; // system
    let mut i = 0;

    while i < old.len() {
        let msg = &old[i];

        // If this is an assistant message with tool_calls, collapse it + following tool results
        if msg.role == Role::Assistant && msg.tool_calls.is_some() {
            let tool_calls = msg.tool_calls.as_ref().unwrap();
            let mut summary_parts = Vec::new();

            for tc in tool_calls {
                summary_parts.push(format!("Called {}({})", tc.function.name, abbreviate(&tc.function.arguments, 80)));
            }

            // Consume following tool result messages
            let mut j = i + 1;
            while j < old.len() && old[j].role == Role::Tool {
                if let Some(ref content) = old[j].content {
                    let tool_name = old[j].name.as_deref().unwrap_or("tool");
                    let brief = abbreviate(content, 150);
                    summary_parts.push(format!("{} returned: {}", tool_name, brief));
                }
                j += 1;
            }

            // Create a single collapsed message
            collapsed.push(Message {
                role: Role::Assistant,
                content: Some(format!("[Tool interaction summary]\n{}", summary_parts.join("\n"))),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            });

            i = j;
            continue;
        }

        // Non-tool messages: keep user/assistant but compress if long
        if msg.role == Role::User || msg.role == Role::Assistant {
            let mut compressed = msg.clone();
            if let Some(ref content) = compressed.content {
                if estimate_tokens(content) > 500 {
                    compressed.content = Some(abbreviate(content, 400));
                }
            }
            collapsed.push(compressed);
        }
        // Drop standalone tool messages (orphaned)

        i += 1;
    }

    collapsed.extend(recent.iter().cloned());
    collapsed
}

/// Summarise old user/assistant exchanges into a single context block.
fn summarise_old_exchanges(
    messages: Vec<Message>,
    preserve_recent: usize,
    _token_budget: usize,
) -> Vec<Message> {
    if messages.len() <= preserve_recent + 2 {
        return messages;
    }

    let split_point = messages.len().saturating_sub(preserve_recent);
    let old = &messages[1..split_point]; // skip system
    let recent = &messages[split_point..];

    // Build a summary of old exchanges
    let mut summary_lines = Vec::new();
    for msg in old {
        let role_label = match msg.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::Tool => "Tool",
            Role::System => continue,
        };
        if let Some(ref content) = msg.content {
            let brief = abbreviate(content, 100);
            summary_lines.push(format!("{}: {}", role_label, brief));
        }
    }

    let summary = format!(
        "[Conversation summary — {} earlier messages]\n{}",
        old.len(),
        summary_lines.join("\n")
    );

    let mut result = vec![messages[0].clone()]; // system
    result.push(Message {
        role: Role::System,
        content: Some(summary),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    });
    result.extend(recent.iter().cloned());
    result
}

/// Abbreviate a string to roughly `max_tokens` tokens (in chars / 4).
fn abbreviate(text: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens * 4;
    if text.len() <= max_chars {
        return text.to_string();
    }
    let truncated = &text[..max_chars.min(text.len())];
    format!("{}...", truncated.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(role: Role, content: &str) -> Message {
        Message {
            role,
            content: Some(content.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    #[test]
    fn test_estimate_tokens() {
        // ~4 chars per token
        assert_eq!(estimate_tokens("hello"), 2); // 5+3 / 4 = 2
        assert_eq!(estimate_tokens(""), 0); // (0+3)/4 = 0
        let long = "a".repeat(1000);
        assert_eq!(estimate_tokens(&long), 250); // (1000+3)/4 = 250
    }

    #[test]
    fn test_small_context_passes_through() {
        let config = ContextConfig::default();
        let cm = ContextManager::new(config);

        let msgs = vec![
            make_msg(Role::System, "You are a helpful assistant."),
            make_msg(Role::User, "Hello"),
            make_msg(Role::Assistant, "Hi there!"),
        ];

        let result = cm.prepare(&msgs, 100_000, 1024);
        assert_eq!(result.messages.len(), 3);
        assert_eq!(result.messages_summarised, 0);
        assert_eq!(result.tool_results_truncated, 0);
    }

    #[test]
    fn test_large_tool_result_truncated() {
        let config = ContextConfig::default();
        let cm = ContextManager::new(config);

        let big_content = "x".repeat(20_000); // ~5000 tokens
        let msgs = vec![
            make_msg(Role::System, "system"),
            Message {
                role: Role::Tool,
                content: Some(big_content),
                tool_calls: None,
                tool_call_id: Some("tc1".into()),
                name: Some("read".into()),
            },
            // Recent messages that won't be touched
            make_msg(Role::User, "recent1"),
            make_msg(Role::Assistant, "recent2"),
            make_msg(Role::User, "recent3"),
            make_msg(Role::Assistant, "recent4"),
            make_msg(Role::User, "recent5"),
            make_msg(Role::Assistant, "recent6"),
        ];

        let result = cm.prepare(&msgs, 100_000, 1024);
        assert_eq!(result.tool_results_truncated, 1);
        // The tool result should be much smaller now
        let tool_msg = &result.messages[1];
        assert!(tool_msg.content.as_ref().unwrap().len() < 5000);
    }

    #[test]
    fn test_hard_drop_when_way_over_budget() {
        let config = ContextConfig::default();
        let cm = ContextManager::new(config);

        // Create way more messages than can fit in 200 tokens
        let mut msgs = vec![make_msg(Role::System, "system")];
        for i in 0..50 {
            msgs.push(make_msg(Role::User, &format!("question {}", i)));
            msgs.push(make_msg(Role::Assistant, &"answer ".repeat(100)));
        }

        let result = cm.prepare(&msgs, 500, 100);
        // Should have been heavily pruned
        assert!(result.messages.len() < msgs.len());
        // System message should still be first
        assert_eq!(result.messages[0].role, Role::System);
    }

    #[test]
    fn test_exceeds_budget() {
        let config = ContextConfig {
            summary_threshold: 100,
            ..Default::default()
        };
        let cm = ContextManager::new(config);

        let msgs = vec![
            make_msg(Role::System, &"x".repeat(500)),
        ];
        assert!(cm.exceeds_budget(&msgs, 200_000));
    }
}
