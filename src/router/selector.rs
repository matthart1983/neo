use anyhow::{anyhow, Result};

use crate::api::types::ModelInfo;
use crate::config::types::BudgetConfig;

use super::capabilities::{CostTier, ModelCapability, SpeedTier};
use super::types::{Complexity, Latency, TaskCategory, TaskProfile};

pub struct ModelRouter {
    capabilities: Vec<ModelCapability>,
    available_models: Vec<ModelInfo>,
    budget_config: BudgetConfig,
}

#[derive(Debug, Clone)]
pub struct SelectedModel {
    pub model_id: String,
    pub score: f64,
    pub fallbacks: Vec<String>,
    pub estimated_cost_per_1k: f64,
}

impl ModelRouter {
    pub fn new(
        capabilities: Vec<ModelCapability>,
        available_models: Vec<ModelInfo>,
        budget_config: BudgetConfig,
    ) -> Self {
        Self {
            capabilities,
            available_models,
            budget_config,
        }
    }

    pub fn select_model(&self, profile: &TaskProfile) -> Result<SelectedModel> {
        let mut scored: Vec<(f64, &ModelCapability, f64)> = self
            .capabilities
            .iter()
            .filter_map(|cap| {
                // Must have enough context
                if cap.context < profile.context_tokens {
                    return None;
                }

                // Must support tool_use if required
                if profile.requires_tool_use
                    && !cap.strengths.iter().any(|s| s == "tool_use")
                {
                    return None;
                }

                // Look up cost from available_models
                let cost_per_1k = self.estimate_cost_per_1k(cap);

                let score = self.compute_score(cap, profile, cost_per_1k);

                Some((score, cap, cost_per_1k))
            })
            .collect();

        if scored.is_empty() {
            return Err(anyhow!(
                "No model satisfies the task profile requirements"
            ));
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let top = &scored[0];
        let fallbacks: Vec<String> = scored
            .iter()
            .skip(1)
            .take(2)
            .map(|(_, cap, _)| cap.model_id.clone())
            .collect();

        Ok(SelectedModel {
            model_id: top.1.model_id.clone(),
            score: top.0,
            fallbacks,
            estimated_cost_per_1k: top.2,
        })
    }

    pub fn select_for_category(&self, category: TaskCategory) -> Result<SelectedModel> {
        let profile = TaskProfile {
            category,
            ..TaskProfile::default()
        };
        self.select_model(&profile)
    }

    fn compute_score(
        &self,
        cap: &ModelCapability,
        profile: &TaskProfile,
        cost_per_1k: f64,
    ) -> f64 {
        let capability_match = self.score_capability_match(cap, profile);
        let cost_efficiency = self.score_cost_efficiency(cap, cost_per_1k);
        let latency = self.score_latency(cap, profile);
        let context_fit = self.score_context_fit(cap, profile);

        capability_match * 0.40
            + cost_efficiency * 0.25
            + latency * 0.20
            + context_fit * 0.15
    }

    fn score_capability_match(&self, cap: &ModelCapability, profile: &TaskProfile) -> f64 {
        let category_str = category_to_strength(&profile.category);

        let strength_match = if cap.strengths.iter().any(|s| s == category_str) {
            1.0
        } else {
            0.3
        };

        let skill_score = match profile.category {
            TaskCategory::CodeGeneration
            | TaskCategory::CodeEdit
            | TaskCategory::TestGeneration
            | TaskCategory::Debugging => cap.coding_score as f64 / 100.0,
            TaskCategory::Planning
            | TaskCategory::Review => {
                (cap.coding_score as f64 * 0.4 + cap.reasoning_score as f64 * 0.6) / 100.0
            }
            _ => cap.reasoning_score as f64 / 100.0,
        };

        let complexity_bonus = match profile.estimated_complexity {
            Complexity::Extreme => {
                if cap.reasoning_score >= 90 {
                    0.1
                } else {
                    -0.1
                }
            }
            Complexity::High => {
                if cap.reasoning_score >= 85 {
                    0.05
                } else {
                    -0.05
                }
            }
            _ => 0.0,
        };

        ((strength_match * 0.5 + skill_score * 0.5) + complexity_bonus).clamp(0.0, 1.0)
    }

    fn score_cost_efficiency(&self, cap: &ModelCapability, cost_per_1k: f64) -> f64 {
        let tier_score = match cap.cost_tier {
            CostTier::Budget => 1.0,
            CostTier::Mid => 0.6,
            CostTier::Premium => 0.3,
        };

        let preferred = &self.budget_config.preferred_cost_tier;
        let preference_bonus = if preferred == "any" {
            0.0
        } else if preferred == "budget" && cap.cost_tier == CostTier::Budget {
            0.2
        } else if preferred == "mid" && cap.cost_tier == CostTier::Mid {
            0.15
        } else if preferred == "premium" && cap.cost_tier == CostTier::Premium {
            0.1
        } else {
            0.0
        };

        let cost_score = if cost_per_1k <= 0.0 {
            1.0
        } else {
            (1.0 / (1.0 + cost_per_1k * 100.0)).clamp(0.0, 1.0)
        };

        (tier_score * 0.5 + cost_score * 0.5 + preference_bonus).clamp(0.0, 1.0)
    }

    fn score_latency(&self, cap: &ModelCapability, profile: &TaskProfile) -> f64 {
        match (&profile.latency_sensitivity, &cap.speed_tier) {
            (Latency::Realtime, SpeedTier::Fast) => 1.0,
            (Latency::Realtime, SpeedTier::Medium) => 0.4,
            (Latency::Realtime, SpeedTier::Slow) => 0.1,
            (Latency::Interactive, SpeedTier::Fast) => 1.0,
            (Latency::Interactive, SpeedTier::Medium) => 0.7,
            (Latency::Interactive, SpeedTier::Slow) => 0.3,
            (Latency::Batch, SpeedTier::Fast) => 0.8,
            (Latency::Batch, SpeedTier::Medium) => 0.9,
            (Latency::Batch, SpeedTier::Slow) => 1.0,
        }
    }

    fn score_context_fit(&self, cap: &ModelCapability, profile: &TaskProfile) -> f64 {
        let needed = profile.context_tokens as f64;
        let available = cap.context as f64;

        if available < needed {
            return 0.0;
        }

        let ratio = needed / available;

        // Sweet spot: model context is 1x-4x the needed tokens
        if ratio >= 0.25 {
            1.0
        } else if ratio >= 0.05 {
            // Gradually penalize wildly oversized context
            0.5 + (ratio - 0.05) / (0.25 - 0.05) * 0.5
        } else {
            0.5
        }
    }

    fn estimate_cost_per_1k(&self, cap: &ModelCapability) -> f64 {
        self.available_models
            .iter()
            .find(|m| m.id == cap.model_id)
            .and_then(|m| m.pricing.as_ref())
            .map(|p| {
                let prompt: f64 = p.prompt.parse().unwrap_or(0.0);
                let completion: f64 = p.completion.parse().unwrap_or(0.0);
                // Pricing is per-token; convert to per-1k
                (prompt + completion) * 1000.0 / 2.0
            })
            .unwrap_or_else(|| match cap.cost_tier {
                CostTier::Budget => 0.001,
                CostTier::Mid => 0.01,
                CostTier::Premium => 0.05,
            })
    }
}

fn category_to_strength(category: &TaskCategory) -> &'static str {
    match category {
        TaskCategory::CodeGeneration => "code_generation",
        TaskCategory::CodeEdit => "code_edit",
        TaskCategory::Review => "review",
        TaskCategory::Planning => "planning",
        TaskCategory::Debugging => "debugging",
        TaskCategory::Search => "search",
        TaskCategory::Documentation => "documentation",
        TaskCategory::TestGeneration => "test_generation",
        TaskCategory::Conversation => "conversation",
    }
}
