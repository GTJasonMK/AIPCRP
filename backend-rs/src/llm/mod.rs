//! LLM 模块
//!
//! 提供统一的 LLM 客户端，支持 OpenAI 和 Anthropic API 格式。

mod anthropic;
mod client;
mod format;
mod openai;
mod types;

pub use client::LlmClient;
pub use format::{detect_api_format, ApiFormat};
pub use types::*;
