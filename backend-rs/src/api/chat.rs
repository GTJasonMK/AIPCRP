//! 聊天相关端点

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::models::{
    ChatContext, SuggestQuestionsRequest, SuggestQuestionsResponse, WsInbound, WsOutbound,
};
use crate::services::{LlmService, PromptService};
use crate::state::AppState;

/// 获取建议问题
async fn suggest_questions(
    Json(req): Json<SuggestQuestionsRequest>,
) -> Json<SuggestQuestionsResponse> {
    let prompt_service = PromptService::new();
    let questions = prompt_service.generate_suggested_questions(
        req.project_path.as_deref(),
        req.current_file.as_deref(),
        req.file_tree_summary.as_deref(),
    );
    Json(SuggestQuestionsResponse { questions })
}

/// WebSocket 升级处理
async fn websocket_upgrade(
    ws: WebSocketUpgrade,
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_websocket)
}

/// WebSocket 连接处理
async fn handle_websocket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();

    info!("WebSocket connected");

    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) => {
                info!("WebSocket client disconnected");
                break;
            }
            Ok(_) => continue,
            Err(e) => {
                error!("WebSocket receive error: {}", e);
                break;
            }
        };

        // 解析入站消息
        let inbound: WsInbound = match serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to parse WebSocket message: {}", e);
                continue;
            }
        };

        match inbound {
            WsInbound::Ping => {
                let pong = WsOutbound::Pong.to_json();
                if let Err(e) = sender.send(Message::Text(pong)).await {
                    error!("Failed to send pong: {}", e);
                    break;
                }
            }
            WsInbound::ChatMessage {
                conversation_id,
                content,
                context,
            } => {
                info!("Received chat message: conversation_id={}", conversation_id);

                // 处理聊天消息
                if let Err(e) = handle_chat_message(
                    &mut sender,
                    &conversation_id,
                    &content,
                    context.as_ref(),
                )
                .await
                {
                    error!("Failed to process chat message: {}", e);
                }
            }
        }
    }

    info!("WebSocket connection closed");
}

/// 处理聊天消息
async fn handle_chat_message(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    conversation_id: &str,
    content: &str,
    context: Option<&ChatContext>,
) -> Result<(), String> {
    let prompt_service = PromptService::new();
    let llm_service = LlmService::new();

    // 构建消息
    let messages = prompt_service.build_chat_messages(
        content,
        context.and_then(|c| c.project_path.as_deref()),
        context.and_then(|c| c.current_file.as_deref()),
        context.and_then(|c| c.current_file_content.as_deref()),
        context.and_then(|c| c.selected_code.as_deref()),
        context.and_then(|c| c.file_tree_summary.as_deref()),
    );

    // 流式调用 LLM
    let stream = match llm_service.stream_chat(messages, None) {
        Ok(s) => s,
        Err(e) => {
            // 配置错误
            let error_msg = WsOutbound::chat_error(conversation_id, e.to_string()).to_json();
            sender
                .send(Message::Text(error_msg))
                .await
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
    };

    // 流式发送响应
    let mut stream = std::pin::pin!(stream);
    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                if let Some(text) = chunk.content {
                    let msg = WsOutbound::chat_chunk(conversation_id, text).to_json();
                    if let Err(e) = sender.send(Message::Text(msg)).await {
                        return Err(format!("Failed to send message: {}", e));
                    }
                }
            }
            Err(e) => {
                let error_msg =
                    WsOutbound::chat_error(conversation_id, format!("AI service error: {}", e))
                        .to_json();
                sender
                    .send(Message::Text(error_msg))
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
    }

    // 发送完成消息
    let done_msg = WsOutbound::chat_done(conversation_id).to_json();
    sender
        .send(Message::Text(done_msg))
        .await
        .map_err(|e| e.to_string())?;

    info!("Chat completed: conversation_id={}", conversation_id);
    Ok(())
}

/// 创建聊天路由
pub fn chat_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/chat/suggest", post(suggest_questions))
        .route("/ws/chat", get(websocket_upgrade))
}
