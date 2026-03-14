use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapability {
    pub model_id: String,
    pub strengths: Vec<String>,
    pub context: usize,
    pub coding_score: u8,
    pub reasoning_score: u8,
    pub speed_tier: SpeedTier,
    pub cost_tier: CostTier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SpeedTier {
    Fast,
    Medium,
    Slow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CostTier {
    Budget,
    Mid,
    Premium,
}

pub fn default_capabilities() -> Vec<ModelCapability> {
    vec![
        ModelCapability {
            model_id: "anthropic/claude-sonnet-4-20250514".into(),
            strengths: vec![
                "code_generation".into(),
                "code_edit".into(),
                "review".into(),
                "debugging".into(),
                "tool_use".into(),
            ],
            context: 200_000,
            coding_score: 95,
            reasoning_score: 92,
            speed_tier: SpeedTier::Fast,
            cost_tier: CostTier::Premium,
        },
        ModelCapability {
            model_id: "openai/o3".into(),
            strengths: vec![
                "planning".into(),
                "debugging".into(),
                "review".into(),
                "reasoning".into(),
            ],
            context: 200_000,
            coding_score: 90,
            reasoning_score: 98,
            speed_tier: SpeedTier::Slow,
            cost_tier: CostTier::Premium,
        },
        ModelCapability {
            model_id: "openai/gpt-4o".into(),
            strengths: vec![
                "code_generation".into(),
                "code_edit".into(),
                "review".into(),
                "conversation".into(),
            ],
            context: 128_000,
            coding_score: 88,
            reasoning_score: 85,
            speed_tier: SpeedTier::Fast,
            cost_tier: CostTier::Mid,
        },
        ModelCapability {
            model_id: "openai/gpt-4o-mini".into(),
            strengths: vec![
                "conversation".into(),
                "documentation".into(),
                "search".into(),
            ],
            context: 128_000,
            coding_score: 72,
            reasoning_score: 70,
            speed_tier: SpeedTier::Fast,
            cost_tier: CostTier::Budget,
        },
        ModelCapability {
            model_id: "deepseek/deepseek-chat-v3-0324".into(),
            strengths: vec![
                "code_generation".into(),
                "code_edit".into(),
            ],
            context: 128_000,
            coding_score: 88,
            reasoning_score: 82,
            speed_tier: SpeedTier::Fast,
            cost_tier: CostTier::Budget,
        },
        ModelCapability {
            model_id: "google/gemini-2.5-pro-preview-06-05".into(),
            strengths: vec![
                "code_generation".into(),
                "review".into(),
                "planning".into(),
            ],
            context: 1_000_000,
            coding_score: 90,
            reasoning_score: 90,
            speed_tier: SpeedTier::Medium,
            cost_tier: CostTier::Mid,
        },
        ModelCapability {
            model_id: "anthropic/claude-3.5-haiku-20241022".into(),
            strengths: vec![
                "conversation".into(),
                "search".into(),
                "documentation".into(),
            ],
            context: 200_000,
            coding_score: 75,
            reasoning_score: 72,
            speed_tier: SpeedTier::Fast,
            cost_tier: CostTier::Budget,
        },
    ]
}
