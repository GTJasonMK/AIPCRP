//! API 路由模块

mod chat;
mod config;
mod docs;
mod graph;
mod health;

pub use chat::chat_routes;
pub use config::config_routes;
pub use docs::docs_routes;
pub use graph::graph_routes;
pub use health::health_routes;

use axum::Router;

use crate::state::AppState;
use std::sync::Arc;

/// 创建所有 API 路由
pub fn create_api_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .merge(health_routes())
        .merge(config_routes())
        .merge(chat_routes())
        .merge(graph_routes())
        .merge(docs_routes())
        .with_state(state)
}
