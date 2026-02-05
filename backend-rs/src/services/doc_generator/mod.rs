//! 文档生成器模块
//!
//! 提供基于 LLM 的代码文档自动生成功能
//!
//! # 功能
//!
//! - 扫描源码目录，构建文件树
//! - 调用 LLM 分析代码文件，生成文档
//! - 层级处理：文件 → 目录 → 项目
//! - 断点续传：支持中断后继续
//! - WebSocket 进度推送
//!
//! # 使用示例
//!
//! ```ignore
//! use std::sync::Arc;
//! use backend_rs::services::doc_generator::{DocGenService, DocGenConfig};
//! use backend_rs::llm::client::LlmClient;
//!
//! let service = DocGenService::with_default_config();
//! let llm_client = Arc::new(LlmClient::new("api_key", "https://api.openai.com/v1", false)?);
//!
//! let (task, progress_rx) = service.start_generation(
//!     source_path,
//!     None,  // 自动生成文档路径
//!     llm_client,
//!     "gpt-4".to_string(),
//!     true,  // 启用断点续传
//! ).await?;
//!
//! // 监听进度
//! while let Ok(msg) = progress_rx.recv().await {
//!     println!("Progress: {:?}", msg);
//! }
//! ```

mod checkpoint;
mod generator;
mod processor;
pub mod prompts;
mod scanner;
pub mod types;

pub use processor::DocGenService;
pub use types::{ProjectGraphData, SharedDocTask, TaskStats, WsDocMessage};
