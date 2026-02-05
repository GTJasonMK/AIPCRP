//! Anthropic Messages API 流式实现

use async_stream::try_stream;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tracing::{debug, error};

use super::format::{build_anthropic_endpoint, get_browser_headers};
use super::types::{ChatChunk, ChatMessage, ChatOptions, LlmError};

/// Anthropic 请求载荷
#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// Anthropic SSE 事件
#[derive(Deserialize, Debug)]
struct AnthropicEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<AnthropicDelta>,
}

#[derive(Deserialize, Debug)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    delta_type: Option<String>,
    text: Option<String>,
    stop_reason: Option<String>,
}

/// 流式调用 Anthropic API
pub fn stream_anthropic(
    client: &Client,
    api_key: &str,
    base_url: &str,
    messages: Vec<ChatMessage>,
    model: &str,
    options: &ChatOptions,
    simulate_browser: bool,
) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>> {
    let endpoint = build_anthropic_endpoint(base_url);
    let api_key = api_key.to_string();
    let model = model.to_string();
    let options = options.clone();
    let client = client.clone();
    let simulate_browser = simulate_browser;

    Box::pin(try_stream! {
        // 分离系统消息
        let mut system_content: Option<String> = None;
        let mut anthropic_messages: Vec<AnthropicMessage> = Vec::new();

        for msg in messages {
            if msg.role == "system" {
                system_content = Some(msg.content);
            } else {
                anthropic_messages.push(AnthropicMessage {
                    role: msg.role,
                    content: msg.content,
                });
            }
        }

        // 构建请求体
        let payload = AnthropicRequest {
            model: model.clone(),
            messages: anthropic_messages,
            system: system_content,
            stream: true,
            max_tokens: options.max_tokens.unwrap_or(4096),
            temperature: options.temperature,
        };

        // 构建请求头
        let mut request = client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("anthropic-version", "2023-06-01");

        // 添加浏览器模拟头
        if simulate_browser {
            for (key, value) in get_browser_headers() {
                request = request.header(key, value);
            }
        }

        debug!("Anthropic API request: endpoint={}, model={}", endpoint, model);

        // 发送请求
        let response = request
            .json(&payload)
            .send()
            .await?;

        // 检查状态码
        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let error_text = response.text().await.unwrap_or_default();
            error!("Anthropic API error: status={}, body={}", status_code, &error_text[..error_text.len().min(500)]);
            Err(LlmError::ApiError {
                status: status_code,
                message: error_text,
            })?;
            // 不会执行到这里
            unreachable!();
        }

        // 处理 SSE 流
        let mut buffer = String::new();
        let mut stream = response.bytes_stream();

        use futures::StreamExt;
        while let Some(chunk_result) = stream.next().await {
            let bytes = chunk_result?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            // 按行处理
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                // 解析 SSE 数据
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        return;
                    }

                    match serde_json::from_str::<AnthropicEvent>(data) {
                        Ok(event) => {
                            match event.event_type.as_str() {
                                "content_block_delta" => {
                                    if let Some(delta) = &event.delta {
                                        if delta.delta_type.as_deref() == Some("text_delta") {
                                            if let Some(text) = &delta.text {
                                                yield ChatChunk {
                                                    content: Some(text.clone()),
                                                    finish_reason: None,
                                                    reasoning_content: None,
                                                };
                                            }
                                        }
                                    }
                                }
                                "message_delta" => {
                                    if let Some(delta) = &event.delta {
                                        if let Some(stop_reason) = &delta.stop_reason {
                                            yield ChatChunk {
                                                content: None,
                                                finish_reason: Some(stop_reason.clone()),
                                                reasoning_content: None,
                                            };
                                        }
                                    }
                                }
                                "message_stop" => {
                                    yield ChatChunk {
                                        content: None,
                                        finish_reason: Some("stop".to_string()),
                                        reasoning_content: None,
                                    };
                                }
                                _ => {
                                    // 忽略其他事件类型
                                }
                            }
                        }
                        Err(e) => {
                            debug!("Failed to parse Anthropic response: {}, data: {}", e, data);
                            // 继续处理，不中断流
                        }
                    }
                }
            }
        }
    })
}
