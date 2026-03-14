use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "neo",
    about = "Agentic AI development platform — multi-model, CLI-first",
    version,
    author
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// One-shot question
    Ask {
        /// The prompt to send
        prompt: String,
    },

    /// Execute a task end-to-end
    Do {
        /// The task description
        task: String,
    },

    /// Review changes
    Review {
        /// Specific commit to review (defaults to staged/working changes)
        commit: Option<String>,
    },

    /// Generate tests for changed files
    Test,

    /// Diagnose an error
    Debug {
        /// The error message or description
        error: String,
    },

    /// Produce an implementation plan
    Plan {
        /// The task to plan
        task: String,
    },

    /// Generate or update documentation
    Doc,

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },

    /// List or search conversation threads
    Threads {
        /// Search query to filter threads
        #[arg(long)]
        search: Option<String>,
    },

    /// Resume a previous conversation
    Resume {
        /// Thread ID to resume
        thread_id: String,
    },

    /// Show cost summary
    Cost {
        /// Time period (today, week, month)
        #[arg(long, default_value = "today")]
        period: String,
    },

    /// List available models
    Models {
        /// Sort field (name, cost, speed)
        #[arg(long, default_value = "name")]
        sort: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        val: String,
    },

    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,
    },
}
