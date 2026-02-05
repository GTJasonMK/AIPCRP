//! 健康检查端点

use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

use crate::state::AppState;
use std::sync::Arc;

/// 健康检查处理器
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok"
    }))
}

/// 创建健康检查路由
pub fn health_routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/health", get(health_check))
}
