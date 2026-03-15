use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use neo_cli::cli::commands::{Cli, Command, ConfigAction};
use neo_cli::cli::repl;
use neo_cli::config::{get_api_key, load_config, save_global_config};
use neo_cli::orchestrator::Orchestrator;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let config = load_config()?;

    if get_api_key(&config).is_none() {
        eprintln!(
            "{}: OPENROUTER_API_KEY is not set.\n\
             Set it in your environment:\n\n  \
             export OPENROUTER_API_KEY=sk-or-v1-...\n\n\
             Or configure the env variable name in ~/.config/neo/config.toml",
            "Error".red().bold()
        );
        std::process::exit(1);
    }

    match cli.command {
        None => {
            let mut orchestrator = Orchestrator::new(config)?;
            orchestrator.init().await;
            repl::start(&mut orchestrator).await?;
        }
        Some(cmd) => dispatch(cmd, config).await?,
    }

    Ok(())
}

fn print_footer(
    orch: &Orchestrator,
    response: &neo_cli::orchestrator::OrchestratorResponse,
) -> String {
    orch.session_manager().format_cost_footer(
        &response.model_used,
        response.tokens_in,
        response.tokens_out,
        response.cost_usd,
        response.context_tokens,
        response.context_limit,
    )
}

async fn dispatch(cmd: Command, config: neo_cli::config::types::NeoConfig) -> Result<()> {
    match cmd {
        Command::Ask { prompt } => {
            let mut orch = Orchestrator::new(config)?;
            orch.init().await;
            let response = orch.handle_message(&prompt).await?;
            println!("{}", response.content);
            let footer = print_footer(&orch, &response);
            println!("{}", footer.dimmed());
        }
        Command::Do { task } => {
            let mut orch = Orchestrator::new(config)?;
            orch.init().await;
            let response = orch.handle_command("do", &task).await?;
            println!("{}", response.content);
            let footer = print_footer(&orch, &response);
            println!("{}", footer.dimmed());
        }
        Command::Review { commit } => {
            let mut orch = Orchestrator::new(config)?;
            orch.init().await;
            let args = commit.unwrap_or_else(|| "Review the current working changes.".into());
            let response = orch.handle_command("review", &args).await?;
            println!("{}", response.content);
            let footer = print_footer(&orch, &response);
            println!("{}", footer.dimmed());
        }
        Command::Plan { task } => {
            let mut orch = Orchestrator::new(config)?;
            orch.init().await;
            let response = orch.handle_command("plan", &task).await?;
            println!("{}", response.content);
            let footer = print_footer(&orch, &response);
            println!("{}", footer.dimmed());
        }
        Command::Debug { error } => {
            let mut orch = Orchestrator::new(config)?;
            orch.init().await;
            let response = orch.handle_command("debug", &error).await?;
            println!("{}", response.content);
            let footer = print_footer(&orch, &response);
            println!("{}", footer.dimmed());
        }
        Command::Test => {
            let mut orch = Orchestrator::new(config)?;
            orch.init().await;
            let response = orch.handle_command("test", "").await?;
            println!("{}", response.content);
            let footer = print_footer(&orch, &response);
            println!("{}", footer.dimmed());
        }
        Command::Doc => {
            let mut orch = Orchestrator::new(config)?;
            orch.init().await;
            let response = orch.handle_command("doc", "").await?;
            println!("{}", response.content);
            let footer = print_footer(&orch, &response);
            println!("{}", footer.dimmed());
        }
        Command::Config { action } => match action {
            Some(ConfigAction::Set { key, val }) => {
                let mut cfg = config;
                match key.as_str() {
                    "default_model" => cfg.core.default_model = val.clone(),
                    "budget.max_per_day" => {
                        if let Ok(v) = val.parse() {
                            cfg.budget.max_per_day = v;
                        } else {
                            eprintln!("{}: invalid number: {}", "Error".red(), val);
                            return Ok(());
                        }
                    }
                    "budget.max_per_request" => {
                        if let Ok(v) = val.parse() {
                            cfg.budget.max_per_request = v;
                        } else {
                            eprintln!("{}: invalid number: {}", "Error".red(), val);
                            return Ok(());
                        }
                    }
                    "context.summary_threshold" => {
                        if let Ok(v) = val.parse() {
                            cfg.context.summary_threshold = v;
                        } else {
                            eprintln!("{}: invalid number: {}", "Error".red(), val);
                            return Ok(());
                        }
                    }
                    "context.max_file_lines" => {
                        if let Ok(v) = val.parse() {
                            cfg.context.max_file_lines = v;
                        } else {
                            eprintln!("{}: invalid number: {}", "Error".red(), val);
                            return Ok(());
                        }
                    }
                    _ => {
                        eprintln!("{}: unknown config key: {}", "Error".red(), key);
                        return Ok(());
                    }
                }
                save_global_config(&cfg)?;
                println!("{} {} = {}", "Set".green(), key.bold(), val);
            }
            Some(ConfigAction::Get { key }) => {
                let value = match key.as_str() {
                    "default_model" => config.core.default_model.clone(),
                    "budget.max_per_day" => config.budget.max_per_day.to_string(),
                    "budget.max_per_request" => config.budget.max_per_request.to_string(),
                    "context.summary_threshold" => config.context.summary_threshold.to_string(),
                    "context.max_file_lines" => config.context.max_file_lines.to_string(),
                    _ => {
                        eprintln!("{}: unknown config key: {}", "Error".red(), key);
                        return Ok(());
                    }
                };
                println!("{} = {}", key.bold(), value);
            }
            None => {
                let toml_str = toml::to_string_pretty(&config)?;
                println!("{}", "Current configuration:".bold());
                println!("{}", toml_str);
            }
        },
        Command::Threads { search } => {
            let orch = Orchestrator::new(config)?;
            let threads = if let Some(query) = &search {
                orch.session_manager().search_threads(query)?
            } else {
                orch.session_manager().list_threads()?
            };
            if threads.is_empty() {
                println!("{}", "No threads found.".dimmed());
            } else {
                println!("{:<38} {:<20} {:<10} {}", "ID", "Updated", "Cost", "Messages");
                for t in &threads {
                    println!(
                        "{:<38} {:<20} ${:<9.4} {}",
                        t.id,
                        t.updated_at.format("%Y-%m-%d %H:%M"),
                        t.cost_total,
                        t.messages.len()
                    );
                }
            }
        }
        Command::Resume { thread_id } => {
            let mut orch = Orchestrator::new(config)?;
            orch.init().await;
            let thread = orch.session_manager().load_thread(&thread_id)?;
            orch.session_manager_mut().set_current_thread(thread);
            println!("{} {}", "Resumed thread:".green(), thread_id);
            repl::start(&mut orch).await?;
        }
        Command::Cost { period } => {
            let orch = Orchestrator::new(config)?;
            let stats = orch.session_stats();
            println!("{} ({})", "Cost Summary".bold(), period);
            println!("  Total cost:      ${:.6}", stats.total_cost);
            println!("  Total tokens in: {}", stats.total_tokens_in);
            println!("  Total tokens out:{}", stats.total_tokens_out);
            println!("  Requests:        {}", stats.request_count);
            if !stats.models_used.is_empty() {
                println!("  Models used:");
                for (model, count) in &stats.models_used {
                    println!("    {} ({}x)", model, count);
                }
            }
        }
        Command::Pipeline { task } => {
            let mut orch = Orchestrator::new(config)?;
            orch.init().await;
            let response = orch.handle_pipeline(&task).await?;
            println!("{}", response.content);
            if let Some(steps) = response.pipeline_steps {
                eprintln!(
                    "{}",
                    format!(
                        "[pipeline: {} steps, {} review cycles]",
                        steps, response.review_cycles
                    )
                    .dimmed()
                );
            }
            let footer = print_footer(&orch, &response);
            println!("{}", footer.dimmed());
        }
        Command::Models { sort } => {
            let api_key = get_api_key(&config).unwrap();
            let client = neo_cli::api::OpenRouterClient::new(
                &config.providers.openrouter,
                api_key,
            )?;
            match client.list_models().await {
                Ok(mut models) => {
                    match sort.as_str() {
                        "cost" => models.sort_by(|a, b| {
                            let cost_a = a.pricing.as_ref().map(|p| p.prompt.parse::<f64>().unwrap_or(0.0)).unwrap_or(0.0);
                            let cost_b = b.pricing.as_ref().map(|p| p.prompt.parse::<f64>().unwrap_or(0.0)).unwrap_or(0.0);
                            cost_a.partial_cmp(&cost_b).unwrap_or(std::cmp::Ordering::Equal)
                        }),
                        _ => models.sort_by(|a, b| a.name.cmp(&b.name)),
                    }
                    println!("{:<50} {:<12} {:<15} {}", "Model", "Context", "Prompt $/tok", "Completion $/tok");
                    for m in models.iter().take(50) {
                        let (prompt_price, comp_price) = match &m.pricing {
                            Some(p) => (p.prompt.as_str(), p.completion.as_str()),
                            None => ("-", "-"),
                        };
                        println!(
                            "{:<50} {:<12} {:<15} {}",
                            m.name, m.context_length, prompt_price, comp_price
                        );
                    }
                    if models.len() > 50 {
                        println!("{}", format!("... and {} more", models.len() - 50).dimmed());
                    }
                }
                Err(e) => {
                    eprintln!("{}: failed to fetch models: {:#}", "Error".red(), e);
                }
            }
        }
    }

    Ok(())
}
