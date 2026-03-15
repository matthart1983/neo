use anyhow::{Context, Result};
use colored::Colorize;

use crate::agents::{AgentExecutor, AgentId, AgentResult};
use crate::api::types::{Message, Role};

use super::plan::{parse_plan, PlanStep};

/// Maximum review→fix cycles before accepting the result.
const MAX_REVIEW_CYCLES: usize = 3;

/// Result of a complete pipeline execution.
#[derive(Debug)]
pub struct PipelineResult {
    pub final_content: String,
    pub steps_executed: usize,
    pub review_cycles: usize,
    pub total_tokens_in: usize,
    pub total_tokens_out: usize,
    pub total_cost: f64,
    pub models_used: Vec<(String, AgentId)>,
}

/// Run the Planner → Coder → Reviewer pipeline for a complex task.
///
/// 1. Planner decomposes the task into steps
/// 2. Steps are executed in dependency order (parallel within each group)
/// 3. Reviewer checks the combined output
/// 4. If Reviewer finds issues, Coder fixes and Reviewer re-checks (up to MAX_REVIEW_CYCLES)
pub async fn run_pipeline(
    executor: &AgentExecutor,
    task: &str,
    context_messages: Vec<Message>,
) -> Result<PipelineResult> {
    let mut tokens_in: usize = 0;
    let mut tokens_out: usize = 0;
    let mut cost: f64 = 0.0;
    let mut models_used: Vec<(String, AgentId)> = Vec::new();

    // --- Phase 1: Plan ---
    eprintln!("{}", "▸ Phase 1: Planning...".cyan().bold());

    let plan_messages = build_agent_messages(
        &context_messages,
        &format!(
            "Decompose this task into numbered steps. For each step, indicate the agent \
             (coder/reviewer/tester/documenter), dependencies on prior steps, and files involved.\n\
             Use this format per step:\n\
             N. **Title** — description [agent: X] [depends: N1, N2] [files: path1, path2]\n\n\
             Task: {}",
            task
        ),
    );

    let plan_result = executor
        .run(&AgentId::Planner, plan_messages)
        .await
        .context("planner failed")?;

    track_result(&plan_result, AgentId::Planner, &mut tokens_in, &mut tokens_out, &mut cost, &mut models_used);

    let plan = parse_plan(&plan_result.content);

    if plan.steps.is_empty() {
        // Planner didn't produce structured steps — fall back to direct Coder execution
        eprintln!("{}", "  ℹ No structured plan; falling back to direct execution.".dimmed());
        return run_direct_with_review(executor, task, context_messages, tokens_in, tokens_out, cost, models_used).await;
    }

    eprintln!(
        "  {} steps planned{}",
        plan.steps.len(),
        if plan.steps.len() > 1 {
            format!(
                " ({} parallel groups)",
                plan.parallel_groups().len()
            )
        } else {
            String::new()
        }
    );

    // --- Phase 2: Execute steps ---
    eprintln!("{}", "▸ Phase 2: Executing...".cyan().bold());

    let groups = plan.parallel_groups();
    let mut step_outputs: Vec<(usize, String)> = Vec::new();
    let mut steps_executed: usize = 0;

    for (group_idx, group) in groups.iter().enumerate() {
        if group.len() > 1 {
            eprintln!("  Group {} ({} steps in parallel)", group_idx + 1, group.len());
        }

        // Execute steps in the group. For now, run sequentially within a group
        // since they share the executor (true parallelism requires separate API clients).
        // The dependency ordering is the important part.
        for &step_idx in group {
            let step = &plan.steps[step_idx];
            eprintln!(
                "  {} Step {}: {}",
                "→".green(),
                step.id,
                step.title
            );

            let step_result = execute_step(
                executor,
                step,
                &context_messages,
                &step_outputs,
            )
            .await?;

            track_result(&step_result, step.agent.clone(), &mut tokens_in, &mut tokens_out, &mut cost, &mut models_used);

            step_outputs.push((step.id, step_result.content));
            steps_executed += 1;
        }
    }

    // --- Phase 3: Review loop ---
    let combined_output = combine_step_outputs(&plan.steps, &step_outputs);

    let (final_content, review_cycles) = run_review_loop(
        executor,
        task,
        &combined_output,
        &context_messages,
        &mut tokens_in,
        &mut tokens_out,
        &mut cost,
        &mut models_used,
    )
    .await?;

    Ok(PipelineResult {
        final_content,
        steps_executed,
        review_cycles,
        total_tokens_in: tokens_in,
        total_tokens_out: tokens_out,
        total_cost: cost,
        models_used,
    })
}

/// Run a task directly with Coder then Reviewer (no planning step).
async fn run_direct_with_review(
    executor: &AgentExecutor,
    task: &str,
    context_messages: Vec<Message>,
    mut tokens_in: usize,
    mut tokens_out: usize,
    mut cost: f64,
    mut models_used: Vec<(String, AgentId)>,
) -> Result<PipelineResult> {
    eprintln!("{}", "▸ Executing task...".cyan().bold());

    let coder_messages = build_agent_messages(&context_messages, task);
    let coder_result = executor
        .run(&AgentId::Coder, coder_messages)
        .await
        .context("coder failed")?;

    track_result(&coder_result, AgentId::Coder, &mut tokens_in, &mut tokens_out, &mut cost, &mut models_used);

    let (final_content, review_cycles) = run_review_loop(
        executor,
        task,
        &coder_result.content,
        &context_messages,
        &mut tokens_in,
        &mut tokens_out,
        &mut cost,
        &mut models_used,
    )
    .await?;

    Ok(PipelineResult {
        final_content,
        steps_executed: 1,
        review_cycles,
        total_tokens_in: tokens_in,
        total_tokens_out: tokens_out,
        total_cost: cost,
        models_used,
    })
}

/// Run the Reviewer → Coder fix loop, up to MAX_REVIEW_CYCLES.
/// Returns the final accepted content and number of review cycles executed.
async fn run_review_loop(
    executor: &AgentExecutor,
    original_task: &str,
    initial_output: &str,
    context_messages: &[Message],
    tokens_in: &mut usize,
    tokens_out: &mut usize,
    cost: &mut f64,
    models_used: &mut Vec<(String, AgentId)>,
) -> Result<(String, usize)> {
    let mut current_output = initial_output.to_string();
    let mut cycles: usize = 0;

    for cycle in 0..MAX_REVIEW_CYCLES {
        eprintln!(
            "{} (cycle {}/{})",
            "▸ Phase 3: Reviewing...".cyan().bold(),
            cycle + 1,
            MAX_REVIEW_CYCLES,
        );

        let review_prompt = format!(
            "Review the following code changes for correctness, bugs, security, and style.\n\
             Original task: {}\n\n\
             Changes:\n{}\n\n\
             If everything looks good, respond with exactly: LGTM\n\
             If there are issues, list them with specific fixes needed.",
            original_task, current_output
        );
        let review_messages = build_agent_messages(context_messages, &review_prompt);
        let review_result = executor
            .run(&AgentId::Reviewer, review_messages)
            .await
            .context("reviewer failed")?;

        track_result(&review_result, AgentId::Reviewer, tokens_in, tokens_out, cost, models_used);
        cycles += 1;

        // Check if reviewer approved
        if is_approved(&review_result.content) {
            eprintln!("  {} Review passed", "✓".green().bold());
            break;
        }

        // Not approved — have Coder fix the issues
        if cycle + 1 < MAX_REVIEW_CYCLES {
            eprintln!(
                "  {} Issues found, sending back to Coder...",
                "⟳".yellow()
            );

            let fix_prompt = format!(
                "The reviewer found issues with your previous work. Fix them.\n\n\
                 Original task: {}\n\n\
                 Your previous output:\n{}\n\n\
                 Reviewer feedback:\n{}",
                original_task, current_output, review_result.content
            );
            let fix_messages = build_agent_messages(context_messages, &fix_prompt);
            let fix_result = executor
                .run(&AgentId::Coder, fix_messages)
                .await
                .context("coder fix iteration failed")?;

            track_result(&fix_result, AgentId::Coder, tokens_in, tokens_out, cost, models_used);
            current_output = fix_result.content;
        } else {
            eprintln!(
                "  {} Max review cycles reached, accepting current output",
                "⚠".yellow()
            );
            // Append the review feedback to the final output
            current_output = format!(
                "{}\n\n---\n**Reviewer notes (not fully resolved):**\n{}",
                current_output, review_result.content
            );
        }
    }

    Ok((current_output, cycles))
}

/// Check if the reviewer approved the changes.
fn is_approved(review_content: &str) -> bool {
    let lower = review_content.to_lowercase();
    // "LGTM" or variations
    if lower.contains("lgtm") {
        return true;
    }
    // Short approving responses
    if lower.len() < 200 {
        let approvals = ["looks good", "approved", "no issues", "ship it", "all good"];
        if approvals.iter().any(|a| lower.contains(a)) {
            return true;
        }
    }
    false
}

/// Execute a single plan step.
async fn execute_step(
    executor: &AgentExecutor,
    step: &PlanStep,
    context_messages: &[Message],
    prior_outputs: &[(usize, String)],
) -> Result<AgentResult> {
    // Build context including outputs from dependency steps
    let mut prompt = step.description.clone();

    let dep_outputs: Vec<&(usize, String)> = prior_outputs
        .iter()
        .filter(|(id, _)| step.depends_on.contains(id))
        .collect();

    if !dep_outputs.is_empty() {
        prompt.push_str("\n\nContext from prior steps:\n");
        for (id, output) in dep_outputs {
            let brief = if output.len() > 1500 {
                format!("{}...", &output[..1500])
            } else {
                output.clone()
            };
            prompt.push_str(&format!("\n--- Step {} output ---\n{}\n", id, brief));
        }
    }

    let messages = build_agent_messages(context_messages, &prompt);
    executor
        .run(&step.agent, messages)
        .await
        .with_context(|| format!("step {} ({}) failed", step.id, step.title))
}

/// Combine all step outputs into a single summary.
fn combine_step_outputs(steps: &[PlanStep], outputs: &[(usize, String)]) -> String {
    let mut combined = String::new();
    for (id, output) in outputs {
        let title = steps
            .iter()
            .find(|s| s.id == *id)
            .map(|s| s.title.as_str())
            .unwrap_or("Step");
        combined.push_str(&format!("### Step {} — {}\n\n{}\n\n", id, title, output));
    }
    combined
}

/// Build a message list with context + a new user prompt.
fn build_agent_messages(context: &[Message], prompt: &str) -> Vec<Message> {
    let mut messages = context.to_vec();
    messages.push(Message {
        role: Role::User,
        content: Some(prompt.to_string()),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    });
    messages
}

fn track_result(
    result: &AgentResult,
    agent: AgentId,
    tokens_in: &mut usize,
    tokens_out: &mut usize,
    cost: &mut f64,
    models_used: &mut Vec<(String, AgentId)>,
) {
    *tokens_in += result.tokens_in;
    *tokens_out += result.tokens_out;
    *cost += result.cost_usd;
    models_used.push((result.model_used.clone(), agent));
}
