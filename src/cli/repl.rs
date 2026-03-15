use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::orchestrator::Orchestrator;

pub async fn start(orchestrator: &mut Orchestrator) -> Result<()> {
    print_banner(orchestrator);

    let mut rl = DefaultEditor::new()?;

    loop {
        match rl.readline("neo> ") {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                rl.add_history_entry(input)?;

                // Handle REPL commands
                if input.starts_with('/') {
                    match input {
                        "/handoff" => {
                            handle_handoff(orchestrator);
                            continue;
                        }
                        "/context" => {
                            print_context_status(orchestrator);
                            continue;
                        }
                        "/help" => {
                            print_repl_help();
                            continue;
                        }
                        _ => {
                            eprintln!(
                                "{}: unknown command '{}'. Type {} for available commands.",
                                "Error".red(),
                                input,
                                "/help".bold()
                            );
                            continue;
                        }
                    }
                }

                let spinner = ProgressBar::new_spinner();
                spinner.set_style(
                    ProgressStyle::default_spinner()
                        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                        .template("{spinner:.cyan} {msg}")
                        .unwrap(),
                );
                spinner.set_message("Thinking...");
                spinner.enable_steady_tick(std::time::Duration::from_millis(80));

                let result = orchestrator.handle_message(input).await;
                spinner.finish_and_clear();

                match result {
                    Ok(response) => {
                        println!("{}", response.content);
                        let footer = orchestrator.session_manager().format_cost_footer(
                            &response.model_used,
                            response.tokens_in,
                            response.tokens_out,
                            response.cost_usd,
                            response.context_tokens,
                            response.context_limit,
                        );
                        println!("{}", footer.dimmed());

                        // Warn if context is getting full
                        let fill_pct = orchestrator.context_fill_percentage();
                        if fill_pct >= 90 {
                            println!(
                                "{}",
                                "⚠ Context window is >90% full. Type /handoff to start a fresh thread with a summary of this conversation.".yellow().bold()
                            );
                        } else if fill_pct >= 70 {
                            println!(
                                "{}",
                                "ℹ Context window is >70% full. Use /handoff when ready to continue in a new thread.".yellow()
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("{}: {:#}", "Error".red(), e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "Cancelled.".yellow());
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "Goodbye!".dimmed());
                break;
            }
            Err(err) => {
                eprintln!("{}: {}", "Error".red(), err);
                break;
            }
        }
    }

    Ok(())
}

fn handle_handoff(orchestrator: &mut Orchestrator) {
    let (tokens, limit) = orchestrator.context_usage();
    let fill_pct = if limit > 0 { (tokens * 100) / limit } else { 0 };

    match orchestrator.handoff_thread() {
        Ok(old_id) => {
            let new_id = orchestrator
                .session_manager()
                .current_thread_id()
                .unwrap_or("unknown");
            println!(
                "{} Handed off from {} ({}% full) → new thread {}",
                "✓".green().bold(),
                old_id.dimmed(),
                fill_pct,
                new_id.cyan().bold(),
            );
            println!(
                "{}",
                "  Previous conversation summarised and carried forward.".dimmed()
            );
            let (new_tokens, new_limit) = orchestrator.context_usage();
            let new_pct = if new_limit > 0 { (new_tokens * 100) / new_limit } else { 0 };
            println!(
                "  Context: {} → {} ({}%)",
                format!("{}%", fill_pct).red(),
                format!("{}%", new_pct).green(),
                new_pct,
            );
        }
        Err(e) => {
            eprintln!("{}: handoff failed: {:#}", "Error".red(), e);
        }
    }
}

fn print_context_status(orchestrator: &Orchestrator) {
    let (tokens, limit) = orchestrator.context_usage();
    let fill_pct = if limit > 0 { (tokens * 100) / limit } else { 0 };
    let messages = orchestrator
        .session_manager()
        .current_thread_messages()
        .map(|m| m.len())
        .unwrap_or(0);

    let bar_width = 30;
    let filled = (fill_pct * bar_width / 100).min(bar_width);
    let empty = bar_width - filled;
    let bar = format!(
        "{}{}",
        "█".repeat(filled),
        "░".repeat(empty),
    );

    let bar_colored = if fill_pct >= 90 {
        bar.red().bold().to_string()
    } else if fill_pct >= 70 {
        bar.yellow().to_string()
    } else {
        bar.green().to_string()
    };

    println!("{}", "Context Window Status".bold());
    println!("  {} {}%", bar_colored, fill_pct);
    println!(
        "  Tokens:   ~{} / {} (threshold)",
        tokens, limit
    );
    println!("  Messages: {}", messages);
    println!(
        "  Thread:   {}",
        orchestrator
            .session_manager()
            .current_thread_id()
            .unwrap_or("none")
    );

    if fill_pct >= 70 {
        println!(
            "\n  {}",
            "Tip: Type /handoff to continue in a fresh thread.".yellow()
        );
    }
}

fn print_repl_help() {
    println!("{}", "REPL Commands".bold());
    println!("  {}      Show context window status", "/context".cyan());
    println!("  {}      Hand off to a new thread (summarises current)", "/handoff".cyan());
    println!("  {}         Show this help", "/help".cyan());
    println!("  {}          Exit", "Ctrl+D".cyan());
    println!("  {}          Cancel current operation", "Ctrl+C".cyan());
}

fn print_banner(orchestrator: &Orchestrator) {
    let budget = orchestrator.config().budget.max_per_day;
    let ctx_limit = orchestrator.config().context.summary_threshold;
    println!(
        "{}",
        format!(
            "neo v0.1.0 | model: auto | budget: ${:.2}/day | context: {}k tokens",
            budget,
            ctx_limit / 1000,
        )
        .bold()
        .cyan()
    );
    println!(
        "{}",
        "Type /help for REPL commands, /context for window status, /handoff to start fresh."
            .dimmed()
    );
    println!();
}
