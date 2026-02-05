//! LLM 类型定义

use serde::{Deserialize, Serialize};

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 角色：system, user, assistant
    pub role: String,
    /// 消息内容
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// 流式响应块
#[derive(Debug, Clone, Default)]
pub struct ChatChunk {
    /// 文本内容
    pub content: Option<String>,
    /// 完成原因
    pub finish_reason: Option<String>,
    /// 推理内容（用于 o1 等模型）
    pub reasoning_content: Option<String>,
}

/// 聊天选项
#[derive(Debug, Clone, Default)]
pub struct ChatOptions {
    /// 温度参数
    pub temperature: Option<f64>,
    /// top_p 参数
    pub top_p: Option<f64>,
    /// 最大 token 数
    pub max_tokens: Option<u32>,
    /// 超时时间（秒）
    pub timeout: Option<u64>,
    /// 响应格式（如 "json_object"）
    pub response_format: Option<String>,
}

/// 流式收集结果
#[derive(Debug, Clone, Default)]
pub struct StreamCollectResult {
    /// 完整响应内容
    pub content: String,
    /// 推理过程
    pub reasoning: String,
    /// 完成原因
    pub finish_reason: Option<String>,
    /// chunk 数量
    pub chunk_count: usize,
}

/// 内容收集模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CollectMode {
    /// 仅收集内容
    #[default]
    ContentOnly,
    /// 同时收集内容和推理
    WithReasoning,
    /// 仅收集推理
    ReasoningOnly,
}

/// LLM 错误类型
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    /// HTTP 请求错误
    #[error("HTTP 请求失败: {0}")]
    HttpError(#[from] reqwest::Error),

    /// API 返回错误
    #[error("API 错误 ({status}): {message}")]
    ApiError { status: u16, message: String },

    /// 超时错误
    #[error("请求超时")]
    Timeout,

    /// 配置错误
    #[error("配置错误: {0}")]
    ConfigError(String),

    /// JSON 解析错误
    #[error("JSON 解析失败: {0}")]
    JsonError(#[from] serde_json::Error),

    /// 流解析错误
    #[error("流解析错误: {0}")]
    StreamError(String),
}
