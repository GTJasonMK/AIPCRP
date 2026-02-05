//! OpenAI Chat Completions API 流式实现

use async_stream::try_stream;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tracing::{debug, error};

use super::format::{build_openai_endpoint, get_browser_headers};
use super::types::{ChatChunk, ChatMessage, ChatOptions, LlmError};

/// OpenAI 请求载荷
#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

/// OpenAI SSE 响应块
#[derive(Deserialize, Debug)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize, Debug)]
struct OpenAiChoice {
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OpenAiDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
}

/// 流式调用 OpenAI API
pub fn stream_openai(
    client: &Client,
    api_key: &str,
    base_url: &str,
    messages: Vec<ChatMessage>,
    model: &str,
    options: &ChatOptions,
    simulate_browser: bool,
) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, LlmError>> + Send>> {
    let endpoint = build_openai_endpoint(base_url);
    let api_key = api_key.to_string();
    let model = model.to_string();
    let options = options.clone();
    let client = client.clone();
    let simulate_browser = simulate_browser;

    Box::pin(try_stream! {
        // 构建请求体
        let payload = OpenAiRequest {
            model: model.clone(),
            messages,
            stream: true,
            temperature: options.temperature,
            top_p: options.top_p,
            max_tokens: options.max_tokens,
            response_format: options.response_format.as_ref().map(|t| ResponseFormat {
                format_type: t.clone(),
            }),
        };

        // 构建请求
        let mut request = client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json");

        // 添加浏览器模拟头
        if simulate_browser {
            for (key, value) in get_browser_headers() {
                request = request.header(key, value);
            }
        }

        debug!("OpenAI API request: endpoint={}, model={}", endpoint, model);

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
            error!("OpenAI API error: status={}, body={}", status_code, &error_text[..error_text.len().min(500)]);
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

                    match serde_json::from_str::<OpenAiStreamChunk>(data) {
                        Ok(chunk) => {
                            if let Some(choice) = chunk.choices.first() {
                                let chat_chunk = ChatChunk {
                                    content: choice.delta.content.clone(),
                                    finish_reason: choice.finish_reason.clone(),
                                    reasoning_content: choice.delta.reasoning_content.clone(),
                                };
                                yield chat_chunk;
                            }
                        }
                        Err(e) => {
                            debug!("Failed to parse OpenAI response: {}, data: {}", e, data);
                            // 继续处理，不中断流
                        }
                    }
                }
            }
        }
    })
}
