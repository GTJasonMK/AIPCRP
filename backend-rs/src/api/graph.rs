//! 知识图谱 API 端点

use axum::{
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::error::AppResult;
use crate::services::code_analyzer::types::GraphData;
use crate::services::CodeAnalyzer;
use crate::state::AppState;

/// 图谱响应
#[derive(Serialize)]
pub struct GraphResponse {
    pub nodes: Vec<serde_json::Value>,
    pub edges: Vec<serde_json::Value>,
}

impl From<GraphData> for GraphResponse {
    fn from(data: GraphData) -> Self {
        Self {
            nodes: data.nodes.into_iter()
                .filter_map(|n| serde_json::to_value(n).ok())
                .collect(),
            edges: data.edges.into_iter()
                .filter_map(|e| serde_json::to_value(e).ok())
                .collect(),
        }
    }
}

/// 项目图谱请求
#[derive(Deserialize)]
pub struct ProjectGraphRequest {
    pub project_path: String,
}

/// 模块图谱请求
#[derive(Deserialize)]
pub struct ModuleGraphRequest {
    pub project_path: String,
    pub file_path: String,
}

/// 获取项目级知识图谱
async fn get_project_graph(
    Json(req): Json<ProjectGraphRequest>,
) -> AppResult<Json<GraphResponse>> {
    let analyzer = CodeAnalyzer::new(&req.project_path);

    let graph = analyzer.analyze_project();
    info!(
        "项目图谱生成完成: {} 节点, {} 边",
        graph.nodes.len(),
        graph.edges.len()
    );

    Ok(Json(GraphResponse::from(graph)))
}

/// 获取模块级知识图谱
async fn get_module_graph(
    Json(req): Json<ModuleGraphRequest>,
) -> AppResult<Json<GraphResponse>> {
    let analyzer = CodeAnalyzer::new(&req.project_path);

    let graph = analyzer.analyze_module(&req.file_path);
    info!(
        "模块图谱生成完成 {}: {} 节点, {} 边",
        req.file_path,
        graph.nodes.len(),
        graph.edges.len()
    );

    Ok(Json(GraphResponse::from(graph)))
}

/// 创建图谱路由
pub fn graph_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/graph/project", post(get_project_graph))
        .route("/api/graph/module", post(get_module_graph))
}
