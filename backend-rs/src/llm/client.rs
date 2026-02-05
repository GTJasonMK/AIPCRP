//! 统一 LLM 客户端

use futures::{Stream, StreamExt};
use reqwest::Client;
use std::pin::Pin;
use std::time::Duration;
use tracing::info;

use super::anthropic::stream_anthropic;
use super::format::{detect_api_format, ApiFormat};
use super::openai::stream_openai;
use super::types::{
    ChatChunk, ChatMessage, ChatOptions, CollectMode, LlmError, StreamCollectResult,
};

/// 统一 LLM 客户端
///
/// 支持 OpenAI 和 Anthropic API 格式，根据模型名称自动选择
pub struct LlmClient {
    client: Client,
    api_key: String,
    base_url: String,
    simulate_browser: bool,
}

impl LlmClient {
    /// 创建新的 LLM 客户端
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>, simulate_browser: bool) -> Result<Self, LlmError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(LlmError::ConfigError("API Key is required".to_string()));
        }

        // 构建 HTTP 客户端
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(5)
            .build()
            .map_err(LlmError::HttpError)?;

        Ok(Self {
            client,
            api_key,
            base_url: base_url.into(),
            simulate_browser,
        })
    }

    /// 流式聊天（自动检测 API 格式）
    pub fn stream_chat(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
        options: ChatOptions,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>> {
        let api_format = detect_api_format(model);
        info!("LLM request: model={}, api_format={:?}", model, api_format);

        match api_format {
            ApiFormat::OpenAi => stream_openai(
                &self.client,
                &self.api_key,
                &self.base_url,
                messages,
                model,
                &options,
                self.simulate_browser,
            ),
            ApiFormat::Anthropic => stream_anthropic(
                &self.client,
                &self.api_key,
                &self.base_url,
                messages,
                model,
                &options,
                self.simulate_browser,
            ),
        }
    }

    /// 流式请求并收集完整响应
    pub async fn stream_and_collect(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
        options: ChatOptions,
        collect_mode: CollectMode,
    ) -> Result<StreamCollectResult, LlmError> {
        let mut stream = self.stream_chat(messages, model, options);
        let mut result = StreamCollectResult::default();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            result.chunk_count += 1;

            // 根据收集模式处理内容
            match collect_mode {
                CollectMode::ContentOnly | CollectMode::WithReasoning => {
                    if let Some(content) = chunk.content {
                        result.content.push_str(&content);
                    }
                }
                CollectMode::ReasoningOnly => {}
            }

            match collect_mode {
                CollectMode::WithReasoning | CollectMode::ReasoningOnly => {
                    if let Some(reasoning) = chunk.reasoning_content {
                        result.reasoning.push_str(&reasoning);
                    }
                }
                CollectMode::ContentOnly => {}
            }

            if chunk.finish_reason.is_some() {
                result.finish_reason = chunk.finish_reason;
            }
        }

        Ok(result)
    }
}
