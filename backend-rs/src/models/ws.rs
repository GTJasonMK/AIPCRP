//! WebSocket 消息类型定义
//!
//! 与前端 WebSocket 协议保持一致

use serde::{Deserialize, Serialize};

/// 聊天上下文
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChatContext {
    /// 项目路径
    #[serde(default)]
    pub project_path: Option<String>,
    /// 当前文件路径
    #[serde(default)]
    pub current_file: Option<String>,
    /// 当前文件内容
    #[serde(default)]
    pub current_file_content: Option<String>,
    /// 选中的代码
    #[serde(default)]
    pub selected_code: Option<String>,
    /// 文件树摘要
    #[serde(default)]
    pub file_tree_summary: Option<String>,
}

/// 入站 WebSocket 消息
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsInbound {
    /// 心跳
    Ping,
    /// 聊天消息
    #[serde(rename = "chat_message")]
    ChatMessage {
        #[serde(rename = "conversationId")]
        conversation_id: String,
        content: String,
        #[serde(default)]
        context: Option<ChatContext>,
    },
}

/// 出站 WebSocket 消息
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsOutbound {
    /// 心跳响应
    Pong,
    /// 聊天内容块
    #[serde(rename = "chat_chunk")]
    ChatChunk {
        #[serde(rename = "conversationId")]
        conversation_id: String,
        content: String,
    },
    /// 聊天完成
    #[serde(rename = "chat_done")]
    ChatDone {
        #[serde(rename = "conversationId")]
        conversation_id: String,
    },
    /// 聊天错误
    #[serde(rename = "chat_error")]
    ChatError {
        #[serde(rename = "conversationId")]
        conversation_id: String,
        error: String,
    },
}

impl WsOutbound {
    /// 创建聊天块消息
    pub fn chat_chunk(conversation_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::ChatChunk {
            conversation_id: conversation_id.into(),
            content: content.into(),
        }
    }

    /// 创建聊天完成消息
    pub fn chat_done(conversation_id: impl Into<String>) -> Self {
        Self::ChatDone {
            conversation_id: conversation_id.into(),
        }
    }

    /// 创建聊天错误消息
    pub fn chat_error(conversation_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::ChatError {
            conversation_id: conversation_id.into(),
            error: error.into(),
        }
    }

    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}
