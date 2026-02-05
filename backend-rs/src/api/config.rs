//! 配置管理端点

use axum::{
    routing::{get, post, put},
    Json, Router,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::{get_config, update_config, AppConfig};
use crate::error::{AppError, AppResult};
use crate::llm::{ChatMessage, ChatOptions, LlmClient};
use crate::state::AppState;

/// 配置响应（隐藏 api_key 的实际值）
#[derive(Serialize)]
pub struct ConfigResponse {
    /// 是否已设置 API 密钥
    pub api_key_set: bool,
    /// API 基础 URL
    pub base_url: String,
    /// 模型名称
    pub model: String,
    /// 温度参数
    pub temperature: f64,
    /// 最大 token 数
    pub max_tokens: u32,
}

impl From<AppConfig> for ConfigResponse {
    fn from(config: AppConfig) -> Self {
        Self {
            api_key_set: !config.api_key.is_empty(),
            base_url: config.base_url,
            model: config.model,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        }
    }
}

/// 配置更新请求
#[derive(Deserialize)]
pub struct ConfigUpdateRequest {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

/// 配置更新响应
#[derive(Serialize)]
pub struct ConfigUpdateResponse {
    pub success: bool,
    pub message: String,
}

/// 连接测试请求
#[derive(Deserialize)]
pub struct TestConnectionRequest {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

/// 连接测试响应
#[derive(Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
    pub model: String,
}

/// 获取当前配置
async fn get_config_handler() -> Json<ConfigResponse> {
    let config = get_config();
    Json(ConfigResponse::from(config))
}

/// 更新配置
async fn update_config_handler(
    Json(req): Json<ConfigUpdateRequest>,
) -> AppResult<Json<ConfigUpdateResponse>> {
    update_config(|config| {
        if let Some(api_key) = req.api_key {
            config.api_key = api_key;
        }
        if let Some(base_url) = req.base_url {
            config.base_url = base_url;
        }
        if let Some(model) = req.model {
            config.model = model;
        }
        if let Some(temperature) = req.temperature {
            config.temperature = temperature;
        }
        if let Some(max_tokens) = req.max_tokens {
            config.max_tokens = max_tokens;
        }
    })?;

    Ok(Json(ConfigUpdateResponse {
        success: true,
        message: "Config updated successfully".to_string(),
    }))
}

/// 测试 LLM 连接
async fn test_connection_handler(
    Json(req): Json<TestConnectionRequest>,
) -> AppResult<Json<TestConnectionResponse>> {
    let config = get_config();

    // 确定使用的参数
    let api_key = req.api_key.unwrap_or(config.api_key);
    let base_url = req.base_url.unwrap_or(config.base_url);
    let model = req.model.unwrap_or(config.model.clone());

    // 检查 API 密钥
    if api_key.is_empty() {
        return Err(AppError::BadRequest("API Key is required".to_string()));
    }

    // 创建 LLM 客户端
    let client = LlmClient::new(&api_key, &base_url, true)
        .map_err(|e| AppError::BadRequest(format!("创建客户端失败: {}", e)))?;

    // 发送测试消息
    let messages = vec![ChatMessage::user("Hi")];
    let options = ChatOptions {
        max_tokens: Some(10),
        ..Default::default()
    };

    let mut stream = client.stream_chat(messages, &model, options);

    // 等待至少一个有效响应
    let mut got_response = false;
    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                if chunk.content.is_some() {
                    got_response = true;
                    break;
                }
            }
            Err(e) => {
                return Err(AppError::BadRequest(format!("Connection failed: {}", e)));
            }
        }
    }

    if got_response {
        Ok(Json(TestConnectionResponse {
            success: true,
            message: "Connection successful".to_string(),
            model,
        }))
    } else {
        Err(AppError::BadRequest("No response from API".to_string()))
    }
}

/// 创建配置路由
pub fn config_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/config", get(get_config_handler))
        .route("/api/config", put(update_config_handler))
        .route("/api/config/test", post(test_connection_handler))
}
