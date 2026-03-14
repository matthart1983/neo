use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::types::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub workspace: String,
    pub messages: Vec<Message>,
    pub cost_total: f64,
    pub models_used: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_cost: f64,
    pub total_tokens_in: usize,
    pub total_tokens_out: usize,
    pub request_count: usize,
    pub models_used: Vec<(String, usize)>,
}

impl Default for SessionStats {
    fn default() -> Self {
        Self {
            total_cost: 0.0,
            total_tokens_in: 0,
            total_tokens_out: 0,
            request_count: 0,
            models_used: Vec::new(),
        }
    }
}
