//! AI Code Review Platform - Rust Backend
//!
//! 使用 axum 框架构建的后端服务，提供 LLM 聊天和代码分析功能。

use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;
mod error;
mod llm;
mod models;
mod services;
mod state;
mod utils;

use api::create_api_routes;
use state::create_shared_state;

/// 在 Windows 上设置控制台代码页为 UTF-8
#[cfg(windows)]
fn setup_console_encoding() {
    unsafe {
        // 设置控制台输出代码页为 UTF-8 (65001)
        extern "system" {
            fn SetConsoleOutputCP(code_page: u32) -> i32;
            fn SetConsoleCP(code_page: u32) -> i32;
        }
        SetConsoleOutputCP(65001);
        SetConsoleCP(65001);
    }
}

#[cfg(not(windows))]
fn setup_console_encoding() {
    // 非 Windows 平台不需要特殊处理
}

#[tokio::main]
async fn main() {
    // 设置控制台编码
    setup_console_encoding();

    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend_rs=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting AI Code Review Platform backend...");

    // 创建共享状态
    let state = create_shared_state();

    // 配置 CORS（允许所有来源，与 Python 版保持一致）
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 构建路由
    let app = Router::new()
        .merge(create_api_routes(Arc::clone(&state)))
        .layer(cors);

    // 绑定地址（与 Python 版相同：127.0.0.1:8765）
    let addr = SocketAddr::from(([127, 0, 0, 1], 8765));
    info!("Server listening on: {}", addr);

    // 启动服务器
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
