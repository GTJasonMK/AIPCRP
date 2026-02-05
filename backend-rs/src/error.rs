//! 统一错误处理模块
//!
//! 定义应用级错误类型，并实现 axum 的 IntoResponse trait 以便自动转换为 HTTP 响应。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// 应用错误枚举
#[derive(Error, Debug)]
pub enum AppError {
    /// 配置相关错误
    #[error("配置错误: {0}")]
    Config(String),

    /// LLM 调用错误
    #[error("LLM 错误: {0}")]
    Llm(String),

    /// 代码分析错误
    #[error("分析错误: {0}")]
    Analyzer(String),

    /// 请求参数错误
    #[error("请求错误: {0}")]
    BadRequest(String),

    /// 资源未找到
    #[error("未找到: {0}")]
    NotFound(String),

    /// 内部错误
    #[error("内部错误: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::Llm(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            AppError::Analyzer(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = Json(json!({
            "success": false,
            "error": error_message
        }));

        (status, body).into_response()
    }
}

/// 便捷类型别名
pub type AppResult<T> = Result<T, AppError>;
