use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use uuid::Uuid;

use crate::api::types::Message;

use super::types::{SessionStats, Thread};

pub struct SessionManager {
    data_dir: PathBuf,
    current_thread: Option<Thread>,
    stats: SessionStats,
}

impl SessionManager {
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .context("could not determine system data directory")?
            .join("neo")
            .join("threads");

        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("failed to create data directory: {}", data_dir.display()))?;

        Ok(Self {
            data_dir,
            current_thread: None,
            stats: SessionStats::default(),
        })
    }

    pub fn start_thread(&mut self, workspace: &Path) -> Thread {
        let thread = Thread {
            id: format!("T-{}", Uuid::new_v4()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            workspace: workspace.display().to_string(),
            messages: Vec::new(),
            cost_total: 0.0,
            models_used: Vec::new(),
            tags: Vec::new(),
        };

        self.current_thread = Some(thread.clone());
        thread
    }

    pub fn add_message(&mut self, msg: Message) {
        if let Some(thread) = &mut self.current_thread {
            thread.messages.push(msg);
            thread.updated_at = Utc::now();
        }
    }

    pub fn record_cost(
        &mut self,
        model: &str,
        cost: f64,
        tokens_in: usize,
        tokens_out: usize,
    ) {
        self.stats.total_cost += cost;
        self.stats.total_tokens_in += tokens_in;
        self.stats.total_tokens_out += tokens_out;
        self.stats.request_count += 1;

        if let Some((_, count)) = self
            .stats
            .models_used
            .iter_mut()
            .find(|(m, _)| m == model)
        {
            *count += 1;
        } else {
            self.stats.models_used.push((model.to_string(), 1));
        }

        if let Some(thread) = &mut self.current_thread {
            thread.cost_total += cost;
            thread.updated_at = Utc::now();
            if !thread.models_used.iter().any(|m| m == model) {
                thread.models_used.push(model.to_string());
            }
        }
    }

    pub fn save_thread(&self) -> Result<()> {
        let thread = self
            .current_thread
            .as_ref()
            .context("no active thread to save")?;

        let path = self.data_dir.join(format!("{}.json", thread.id));
        let json = serde_json::to_string_pretty(thread)
            .context("failed to serialize thread")?;

        std::fs::write(&path, json)
            .with_context(|| format!("failed to write thread file: {}", path.display()))?;

        Ok(())
    }

    pub fn load_thread(&self, id: &str) -> Result<Thread> {
        let path = self.data_dir.join(format!("{}.json", id));
        let json = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read thread file: {}", path.display()))?;

        let thread: Thread =
            serde_json::from_str(&json).context("failed to parse thread JSON")?;

        Ok(thread)
    }

    pub fn list_threads(&self) -> Result<Vec<Thread>> {
        let mut threads = Vec::new();

        let entries = std::fs::read_dir(&self.data_dir)
            .with_context(|| {
                format!(
                    "failed to read threads directory: {}",
                    self.data_dir.display()
                )
            })?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let json = std::fs::read_to_string(&path)?;
                if let Ok(thread) = serde_json::from_str::<Thread>(&json) {
                    threads.push(Thread {
                        messages: Vec::new(),
                        ..thread
                    });
                }
            }
        }

        threads.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(threads)
    }

    pub fn search_threads(&self, query: &str) -> Result<Vec<Thread>> {
        let query_lower = query.to_lowercase();
        let mut matches = Vec::new();

        let entries = std::fs::read_dir(&self.data_dir)
            .with_context(|| {
                format!(
                    "failed to read threads directory: {}",
                    self.data_dir.display()
                )
            })?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let json = std::fs::read_to_string(&path)?;
                if let Ok(thread) = serde_json::from_str::<Thread>(&json) {
                    let has_match = thread.messages.iter().any(|msg| {
                        msg.content
                            .as_ref()
                            .map(|c| c.to_lowercase().contains(&query_lower))
                            .unwrap_or(false)
                    });
                    if has_match {
                        matches.push(thread);
                    }
                }
            }
        }

        matches.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(matches)
    }

    pub fn current_stats(&self) -> &SessionStats {
        &self.stats
    }

    pub fn current_thread_messages(&self) -> Option<&Vec<Message>> {
        self.current_thread.as_ref().map(|t| &t.messages)
    }

    pub fn set_current_thread(&mut self, thread: Thread) {
        self.current_thread = Some(thread);
    }

    pub fn format_cost_footer(
        &self,
        model: &str,
        tokens_in: usize,
        tokens_out: usize,
        cost: f64,
    ) -> String {
        format!(
            "[{} · {}↑ {}↓ · ${:.6} · session ${:.4}]",
            model, tokens_in, tokens_out, cost, self.stats.total_cost
        )
    }
}
