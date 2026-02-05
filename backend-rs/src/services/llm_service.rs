//! LLM 服务封装
//!
//! 封装 LlmClient，与配置系统集成

use futures::Stream;
use std::pin::Pin;

use crate::config::get_config;
use crate::llm::{ChatChunk, ChatMessage, ChatOptions, LlmClient, LlmError};

/// LLM 服务
pub struct LlmService {
    client: Option<LlmClient>,
    model: String,
    temperature: f64,
    max_tokens: u32,
}

impl LlmService {
    /// 创建新的 LLM 服务
    pub fn new() -> Self {
        let mut service = Self {
            client: None,
            model: String::new(),
            temperature: 0.7,
            max_tokens: 4096,
        };
        service.refresh_client();
        service
    }

    /// 刷新客户端（重新读取配置）
    pub fn refresh_client(&mut self) {
        let config = get_config();

        if config.api_key.is_empty() {
            self.client = None;
            return;
        }

        match LlmClient::new(&config.api_key, &config.base_url, true) {
            Ok(client) => {
                self.client = Some(client);
                self.model = config.model;
                self.temperature = config.temperature;
                self.max_tokens = config.max_tokens;
            }
            Err(_) => {
                self.client = None;
            }
        }
    }

    /// 流式聊天
    pub fn stream_chat(
        &self,
        messages: Vec<ChatMessage>,
        model: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>>, LlmError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| LlmError::ConfigError("API Key not configured. Please set it in Settings.".to_string()))?;

        let model = model.unwrap_or(&self.model);
        let options = ChatOptions {
            temperature: Some(self.temperature),
            max_tokens: Some(self.max_tokens),
            ..Default::default()
        };

        Ok(client.stream_chat(messages, model, options))
    }
}

impl Default for LlmService {
    fn default() -> Self {
        Self::new()
    }
}
