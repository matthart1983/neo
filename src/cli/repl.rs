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
                        );
                        println!("{}", footer.dimmed());
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

fn print_banner(orchestrator: &Orchestrator) {
    let budget = orchestrator.config().budget.max_per_day;
    println!(
        "{}",
        format!(
            "neo v0.1.0 | model: auto | budget: ${:.2} remaining today",
            budget
        )
        .bold()
        .cyan()
    );
    println!();
}
