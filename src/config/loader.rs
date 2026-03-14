use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::types::{NeoConfig, Permission};

pub fn load_config() -> Result<NeoConfig> {
    let mut config = NeoConfig::default();

    // Layer 1: Global config (~/.config/neo/config.toml)
    if let Some(global_path) = global_config_path() {
        if global_path.exists() {
            let content = fs::read_to_string(&global_path)
                .with_context(|| format!("failed to read {}", global_path.display()))?;
            let overlay: NeoConfig = toml::from_str(&content)
                .with_context(|| format!("failed to parse {}", global_path.display()))?;
            merge_config(&mut config, overlay);
        }
    }

    // Layer 2: Workspace config (<cwd>/.neo/config.toml)
    let cwd = env::current_dir().unwrap_or_default();
    let workspace_config = cwd.join(".neo").join("config.toml");
    if workspace_config.exists() {
        let content = fs::read_to_string(&workspace_config)
            .with_context(|| format!("failed to read {}", workspace_config.display()))?;
        let overlay: NeoConfig = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", workspace_config.display()))?;
        merge_config(&mut config, overlay);
    }

    // Layer 3: Workspace local config (<cwd>/.neo/local.toml)
    let local_config = cwd.join(".neo").join("local.toml");
    if local_config.exists() {
        let content = fs::read_to_string(&local_config)
            .with_context(|| format!("failed to read {}", local_config.display()))?;
        let overlay: NeoConfig = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", local_config.display()))?;
        merge_config(&mut config, overlay);
    }

    // Layer 4: Environment variable overrides
    apply_env_overrides(&mut config);

    Ok(config)
}

pub fn save_global_config(config: &NeoConfig) -> Result<()> {
    let path = global_config_path().context("could not determine global config directory")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let content =
        toml::to_string_pretty(config).context("failed to serialize config to TOML")?;
    fs::write(&path, content)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn get_api_key(config: &NeoConfig) -> Option<String> {
    env::var(&config.providers.openrouter.api_key_env).ok()
}

fn global_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("neo").join("config.toml"))
}

fn merge_config(base: &mut NeoConfig, overlay: NeoConfig) {
    *base = overlay;
}

fn apply_env_overrides(config: &mut NeoConfig) {
    if let Ok(val) = env::var("NEO_DEFAULT_MODEL") {
        config.core.default_model = val;
    }
    if let Ok(val) = env::var("NEO_BUDGET_MAX_PER_DAY") {
        if let Ok(v) = val.parse() {
            config.budget.max_per_day = v;
        }
    }
    if let Ok(val) = env::var("NEO_BUDGET_MAX_PER_REQUEST") {
        if let Ok(v) = val.parse() {
            config.budget.max_per_request = v;
        }
    }
    if let Ok(val) = env::var("NEO_PERMISSIONS_SHELL") {
        match val.to_lowercase().as_str() {
            "auto" => config.permissions.shell_tools = Permission::Auto,
            "confirm" => config.permissions.shell_tools = Permission::Confirm,
            "deny" => config.permissions.shell_tools = Permission::Deny,
            _ => {}
        }
    }
    if let Ok(val) = env::var("NEO_WORKFLOW_AUTO_TEST") {
        match val.to_lowercase().as_str() {
            "true" | "1" | "yes" => config.workflow.auto_test = true,
            "false" | "0" | "no" => config.workflow.auto_test = false,
            _ => {}
        }
    }
}
