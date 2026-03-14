use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::stream::{self, Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

use crate::config::types::OpenRouterConfig;

use super::types::{
    ChatRequest, ChatResponse, ModelInfo, ModelsResponse, StreamChunk, Usage,
};

pub struct OpenRouterClient {
    http: reqwest::Client,
    base_url: String,
    _api_key: String,
    max_retries: u8,
}

impl OpenRouterClient {
    pub fn new(config: &OpenRouterConfig, api_key: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .context("invalid API key for header")?,
        );
        headers.insert(
            "HTTP-Referer",
            HeaderValue::from_static("https://github.com/matthart1983/neo"),
        );
        headers.insert("X-Title", HeaderValue::from_static("Neo CLI"));

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .default_headers(headers)
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            http,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            _api_key: api_key,
            max_retries: config.max_retries,
        })
    }

    pub async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let mut last_err = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let backoff = Duration::from_secs(1 << (attempt - 1));
                tokio::time::sleep(backoff).await;
            }

            let response = self
                .http
                .post(&url)
                .json(request)
                .send()
                .await
                .context("failed to send chat request");

            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    last_err = Some(e);
                    continue;
                }
            };

            let status = response.status();

            if status.is_success() {
                let body = response
                    .text()
                    .await
                    .context("failed to read response body")?;
                let chat_response: ChatResponse = serde_json::from_str(&body)
                    .with_context(|| {
                        format!("failed to parse chat response: {}", body)
                    })?;
                return Ok(chat_response);
            }

            if status.as_u16() == 429 || status.is_server_error() {
                let body = response.text().await.unwrap_or_default();
                last_err = Some(anyhow::anyhow!(
                    "HTTP {} from OpenRouter: {}",
                    status,
                    body
                ));
                continue;
            }

            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {} from OpenRouter: {}", status, body);
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("chat request failed after retries")))
    }

    pub async fn chat_stream(
        &self,
        request: &ChatRequest,
    ) -> Result<impl Stream<Item = Result<StreamChunk>>> {
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .http
            .post(&url)
            .json(request)
            .send()
            .await
            .context("failed to send streaming chat request")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {} from OpenRouter: {}", status, body);
        }

        let byte_stream = response.bytes_stream();

        let chunk_stream = stream::unfold(
            (byte_stream, String::new()),
            |(mut byte_stream, mut buffer)| async move {
                loop {
                    if let Some(line_end) = buffer.find('\n') {
                        let line = buffer[..line_end].trim().to_string();
                        buffer = buffer[line_end + 1..].to_string();

                        if line.is_empty() {
                            continue;
                        }

                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                return None;
                            }

                            let chunk: Result<StreamChunk> =
                                serde_json::from_str(data).with_context(|| {
                                    format!("failed to parse stream chunk: {}", data)
                                });

                            return Some((chunk, (byte_stream, buffer)));
                        }

                        continue;
                    }

                    match byte_stream.next().await {
                        Some(Ok(bytes)) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));
                        }
                        Some(Err(e)) => {
                            return Some((
                                Err(anyhow::anyhow!("stream read error: {}", e)),
                                (byte_stream, buffer),
                            ));
                        }
                        None => {
                            if buffer.trim().is_empty() {
                                return None;
                            }
                            let line = buffer.trim().to_string();
                            buffer.clear();
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    return None;
                                }
                                let chunk: Result<StreamChunk> =
                                    serde_json::from_str(data).with_context(|| {
                                        format!("failed to parse stream chunk: {}", data)
                                    });
                                return Some((chunk, (byte_stream, buffer)));
                            }
                            return None;
                        }
                    }
                }
            },
        );

        Ok(chunk_stream)
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/models", self.base_url);

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("failed to fetch models")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {} from OpenRouter: {}", status, body);
        }

        let body = response
            .text()
            .await
            .context("failed to read models response")?;
        let models_response: ModelsResponse = serde_json::from_str(&body)
            .with_context(|| format!("failed to parse models response: {}", body))?;

        Ok(models_response.data)
    }

    pub fn calculate_cost(model: &ModelInfo, usage: &Usage) -> f64 {
        let pricing = match &model.pricing {
            Some(p) => p,
            None => return 0.0,
        };

        let prompt_cost: f64 = pricing.prompt.parse().unwrap_or(0.0);
        let completion_cost: f64 = pricing.completion.parse().unwrap_or(0.0);

        (prompt_cost * usage.prompt_tokens as f64)
            + (completion_cost * usage.completion_tokens as f64)
    }
}
