//! 文档生成 API 端点
//!
//! 提供文档生成任务的 REST API 和 WebSocket 接口

use axum::{
    extract::{Path, State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

use crate::config::get_config;
use crate::error::AppError;
use crate::llm::LlmClient;
use crate::services::doc_generator::{DocGenService, ProjectGraphData, TaskStats, WsDocMessage};
use crate::services::doc_generator::types::{DirGraphData, FileGraphData};
use crate::state::{AppState, CompletedPathType, InProgressPathType, TaskState};

/// 创建文档生成路由
pub fn docs_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/docs/generate", post(generate_docs))
        .route("/api/docs/tasks/:id", get(get_task_status))
        .route("/api/docs/tasks/:id/cancel", post(cancel_task))
        .route("/api/docs/graph", post(get_project_graph))
        .route("/api/docs/file-graph", post(get_file_graph))
        .route("/api/docs/dir-graph", post(get_dir_graph))
        .route("/ws/docs/:id", get(ws_handler))
}

/// 生成文档请求
#[derive(Debug, Deserialize)]
pub struct GenerateDocsRequest {
    /// 源码路径
    pub source_path: String,
    /// 文档输出路径（可选，默认为 {source}_docs）
    pub docs_path: Option<String>,
    /// 是否启用断点续传（默认 true）
    pub resume: Option<bool>,
}

/// 生成文档响应
#[derive(Debug, Serialize)]
pub struct GenerateDocsResponse {
    /// 任务 ID
    pub task_id: String,
    /// 文档输出路径
    pub docs_path: String,
}

/// 任务状态响应
#[derive(Debug, Serialize)]
pub struct TaskStatusResponse {
    /// 任务 ID
    pub id: String,
    /// 任务状态
    pub status: String,
    /// 进度百分比 (0-100)
    pub progress: f32,
    /// 当前处理的文件
    pub current_file: Option<String>,
    /// 统计信息
    pub stats: TaskStats,
    /// 错误信息
    pub error: Option<String>,
}

/// 启动文档生成任务
async fn generate_docs(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GenerateDocsRequest>,
) -> Result<Json<GenerateDocsResponse>, AppError> {
    info!("Received document generation request: source_path={}", req.source_path);

    // 验证源码路径
    let source_path = PathBuf::from(&req.source_path);
    if !source_path.exists() {
        return Err(AppError::BadRequest(format!(
            "源码路径不存在: {}",
            req.source_path
        )));
    }
    if !source_path.is_dir() {
        return Err(AppError::BadRequest(format!(
            "源码路径不是目录: {}",
            req.source_path
        )));
    }

    // 获取配置
    let config = get_config();

    // 创建 LLM 客户端
    let llm_client = Arc::new(
        LlmClient::new(&config.api_key, &config.base_url, false)
            .map_err(|e| AppError::Internal(format!("创建 LLM 客户端失败: {}", e)))?,
    );

    // 计算文档路径：默认放在项目根目录下的 .docs 目录
    let docs_path = req.docs_path.map(PathBuf::from).unwrap_or_else(|| {
        source_path.join(".docs")
    });

    // 创建文档生成服务
    let service = DocGenService::with_default_config();

    // 启动生成任务
    let (task, progress_rx) = service
        .start_generation(
            source_path,
            Some(docs_path.clone()),
            llm_client,
            config.model.clone(),
            req.resume.unwrap_or(true),
        )
        .await
        .map_err(|e| AppError::Internal(format!("启动文档生成失败: {}", e)))?;

    // 获取任务 ID
    let task_id = task.read().await.id.clone();

    // 创建广播通道（用于 WebSocket）
    // 保留一个接收器以防止在 WebSocket 客户端连接前 send 失败
    let (tx, _keep_alive_rx) = broadcast::channel(100);

    // 创建任务状态
    let task_state = Arc::new(TaskState::new(task, tx.clone()));

    // 注册任务
    state.doc_tasks.insert(task_id.clone(), task_state.clone());

    // 启动进度转发任务
    let task_id_clone = task_id.clone();
    let tx_clone = tx.clone();
    let task_state_clone = task_state.clone();
    tokio::spawn(async move {
        // 保持接收器存活，防止在 WebSocket 客户端连接前 tx.send 因无接收器而失败
        let _rx_guard = _keep_alive_rx;
        let mut rx = progress_rx;
        while let Ok(msg) = rx.recv().await {
            // 记录路径状态，用于 WebSocket 连接时重放
            match &msg {
                WsDocMessage::FileStarted { path } => {
                    task_state_clone.mark_file_started(path.clone());
                }
                WsDocMessage::FileCompleted { path } => {
                    task_state_clone.mark_file_completed(path.clone());
                }
                WsDocMessage::DirStarted { path } => {
                    task_state_clone.mark_dir_started(path.clone());
                }
                WsDocMessage::DirCompleted { path } => {
                    task_state_clone.mark_dir_completed(path.clone());
                }
                _ => {}
            }

            // 即使当前没有 WebSocket 订阅者，也继续转发（不因 send 失败退出）
            let _ = tx_clone.send(msg.clone());

            // 如果任务完成或失败，退出循环
            match &msg {
                WsDocMessage::Completed { .. }
                | WsDocMessage::Error { .. }
                | WsDocMessage::Cancelled => {
                    break;
                }
                _ => {}
            }
        }
        info!("Task {} progress forwarding ended", task_id_clone);
    });

    Ok(Json(GenerateDocsResponse {
        task_id,
        docs_path: docs_path.to_string_lossy().to_string(),
    }))
}

/// 获取任务状态
async fn get_task_status(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskStatusResponse>, AppError> {
    let entry = state
        .doc_tasks
        .get(&task_id)
        .ok_or_else(|| AppError::NotFound(format!("Task not found: {}", task_id)))?;

    let task_state = entry.value();
    let task = task_state.task.read().await;

    Ok(Json(TaskStatusResponse {
        id: task.id.clone(),
        status: format!("{:?}", task.status).to_lowercase(),
        progress: task.progress,
        current_file: task.current_file.clone(),
        stats: task.stats.clone(),
        error: task.error.clone(),
    }))
}

/// 取消任务
async fn cancel_task(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let entry = state
        .doc_tasks
        .get(&task_id)
        .ok_or_else(|| AppError::NotFound(format!("Task not found: {}", task_id)))?;

    let task_state = entry.value();
    {
        let mut task = task_state.task.write().await;
        task.cancel();
    }

    // 发送取消消息
    let _ = task_state.tx.send(WsDocMessage::Cancelled);

    info!("Task cancelled: {}", task_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Task cancelled"
    })))
}

/// WebSocket 进度推送处理器
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state, task_id))
}

/// 处理 WebSocket 连接
async fn handle_ws_connection(
    socket: axum::extract::ws::WebSocket,
    state: Arc<AppState>,
    task_id: String,
) {
    let (mut sender, mut receiver) = socket.split();

    // 获取任务状态
    let task_state = match state.doc_tasks.get(&task_id) {
        Some(entry) => entry.value().clone(),
        None => {
            let _ = sender
                .send(axum::extract::ws::Message::Text(
                    serde_json::to_string(&WsDocMessage::Error {
                        message: format!("Task not found: {}", task_id),
                    })
                    .unwrap(),
                ))
                .await;
            return;
        }
    };

    info!("WebSocket connection established: task_id={}", task_id);

    // 发送当前状态
    {
        let task = task_state.task.read().await;
        let msg = WsDocMessage::Progress {
            progress: task.progress,
            current_file: task.current_file.clone(),
            stats: task.stats.clone(),
        };
        let _ = sender
            .send(axum::extract::ws::Message::Text(
                serde_json::to_string(&msg).unwrap(),
            ))
            .await;
    }

    // 重放已完成的文件/目录消息
    // 这样前端可以正确显示在 WebSocket 连接前已处理完成的文件状态
    let completed_paths = task_state.get_completed_paths();
    info!("Replaying {} completed paths for task {}", completed_paths.len(), task_id);
    for path_type in completed_paths {
        let msg = match path_type {
            CompletedPathType::File(path) => WsDocMessage::FileCompleted { path },
            CompletedPathType::Dir(path) => WsDocMessage::DirCompleted { path },
        };
        if sender
            .send(axum::extract::ws::Message::Text(
                serde_json::to_string(&msg).unwrap(),
            ))
            .await
            .is_err()
        {
            return;
        }
    }

    // 重放正在处理中的文件/目录状态（FileStarted/DirStarted）
    // 因为 Started 消息可能在 WebSocket 连接前就已发送，前端未收到
    let in_progress_paths = task_state.get_in_progress_paths();
    info!("Replaying {} in-progress paths for task {}", in_progress_paths.len(), task_id);
    for path_type in in_progress_paths {
        let msg = match path_type {
            InProgressPathType::File(path) => WsDocMessage::FileStarted { path },
            InProgressPathType::Dir(path) => WsDocMessage::DirStarted { path },
        };
        if sender
            .send(axum::extract::ws::Message::Text(
                serde_json::to_string(&msg).unwrap(),
            ))
            .await
            .is_err()
        {
            return;
        }
    }

    // 订阅广播通道以接收后续消息
    let mut rx = task_state.tx.subscribe();

    // 监听进度消息
    loop {
        tokio::select! {
            // 接收进度消息并发送给客户端
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        let json = serde_json::to_string(&msg).unwrap();
                        if sender.send(axum::extract::ws::Message::Text(json)).await.is_err() {
                            break;
                        }

                        // 如果任务完成，关闭连接
                        match msg {
                            WsDocMessage::Completed { .. }
                            | WsDocMessage::Error { .. }
                            | WsDocMessage::Cancelled => {
                                break;
                            }
                            _ => {}
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // 跳过延迟的消息
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }

            // 处理客户端消息（主要是 ping/pong）
            result = receiver.next() => {
                match result {
                    Some(Ok(axum::extract::ws::Message::Ping(data))) => {
                        let _ = sender.send(axum::extract::ws::Message::Pong(data)).await;
                    }
                    Some(Ok(axum::extract::ws::Message::Close(_))) | None => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    info!("WebSocket connection closed: task_id={}", task_id);
}

/// 获取项目图谱请求
#[derive(Debug, Deserialize)]
pub struct GetProjectGraphRequest {
    /// 文档路径（.docs 目录的路径）
    pub docs_path: String,
}

/// 获取项目级知识图谱
///
/// 读取 .docs/_project_graph.json 文件并返回
async fn get_project_graph(
    Json(req): Json<GetProjectGraphRequest>,
) -> Result<Json<ProjectGraphData>, AppError> {
    let docs_path = PathBuf::from(&req.docs_path);

    // 验证路径存在
    if !docs_path.exists() {
        return Err(AppError::NotFound(format!(
            "文档目录不存在: {}",
            req.docs_path
        )));
    }

    // 构建项目图谱路径
    let graph_path = docs_path.join("_project_graph.json");

    if !graph_path.exists() {
        return Err(AppError::NotFound(format!(
            "项目图谱文件不存在: {}。请先生成文档以创建知识图谱。",
            graph_path.display()
        )));
    }

    // 读取并解析文件
    let content = tokio::fs::read_to_string(&graph_path)
        .await
        .map_err(|e| AppError::Internal(format!("读取项目图谱文件失败: {}", e)))?;

    let graph_data: ProjectGraphData = serde_json::from_str(&content)
        .map_err(|e| AppError::Internal(format!("解析项目图谱数据失败: {}", e)))?;

    info!(
        "返回项目图谱: {} 节点, {} 边",
        graph_data.nodes.len(),
        graph_data.edges.len()
    );

    Ok(Json(graph_data))
}

/// 获取单文件图谱请求
#[derive(Debug, Deserialize)]
pub struct GetFileGraphRequest {
    /// 文档路径（.docs 目录的路径）
    pub docs_path: String,
    /// 文件相对路径（相对于项目根目录）
    pub file_path: String,
}

/// 获取单文件知识图谱
///
/// 读取 .docs/{dir}/{filename}.graph.json 文件并返回
async fn get_file_graph(
    Json(req): Json<GetFileGraphRequest>,
) -> Result<Json<FileGraphData>, AppError> {
    let docs_path = PathBuf::from(&req.docs_path);

    // 验证 docs 路径存在
    if !docs_path.exists() {
        return Err(AppError::NotFound(format!(
            "文档目录不存在: {}",
            req.docs_path
        )));
    }

    // 构建文件图谱路径
    // 例如: file_path = "src/utils/helper.py" -> docs_path/src/utils/helper.py.graph.json
    let file_path = std::path::Path::new(&req.file_path);
    let file_name = file_path.file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AppError::BadRequest("无效的文件路径".to_string()))?;

    let graph_name = format!("{}.graph.json", file_name);
    let graph_path = match file_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => {
            docs_path.join(parent).join(graph_name)
        }
        _ => docs_path.join(graph_name),
    };

    if !graph_path.exists() {
        return Err(AppError::NotFound(format!(
            "文件图谱不存在: {}",
            graph_path.display()
        )));
    }

    // 读取并解析文件
    let content = tokio::fs::read_to_string(&graph_path)
        .await
        .map_err(|e| AppError::Internal(format!("读取文件图谱失败: {}", e)))?;

    let graph_data: FileGraphData = serde_json::from_str(&content)
        .map_err(|e| AppError::Internal(format!("解析文件图谱数据失败: {}", e)))?;

    info!(
        "返回文件图谱 {}: {} 节点, {} 边",
        req.file_path,
        graph_data.nodes.len(),
        graph_data.edges.len()
    );

    Ok(Json(graph_data))
}

/// 获取目录图谱请求
#[derive(Debug, Deserialize)]
pub struct GetDirGraphRequest {
    /// 文档路径（.docs 目录的路径）
    pub docs_path: String,
    /// 目录相对路径（相对于项目根目录，根目录传空字符串）
    pub dir_path: String,
}

/// 获取目录知识图谱
///
/// 读取 .docs/{dir_path}/_dir.graph.json 文件并返回
async fn get_dir_graph(
    Json(req): Json<GetDirGraphRequest>,
) -> Result<Json<DirGraphData>, AppError> {
    let docs_path = PathBuf::from(&req.docs_path);

    // 验证 docs 路径存在
    if !docs_path.exists() {
        return Err(AppError::NotFound(format!(
            "文档目录不存在: {}",
            req.docs_path
        )));
    }

    // 构建目录图谱路径
    // 例如: dir_path = "src/utils" -> docs_path/src/utils/_dir.graph.json
    // 根目录: dir_path = "" -> docs_path/_dir.graph.json
    let graph_path = if req.dir_path.is_empty() {
        docs_path.join("_dir.graph.json")
    } else {
        docs_path.join(&req.dir_path).join("_dir.graph.json")
    };

    if !graph_path.exists() {
        return Err(AppError::NotFound(format!(
            "目录图谱不存在: {}",
            graph_path.display()
        )));
    }

    // 读取并解析文件
    let content = tokio::fs::read_to_string(&graph_path)
        .await
        .map_err(|e| AppError::Internal(format!("读取目录图谱失败: {}", e)))?;

    let graph_data: DirGraphData = serde_json::from_str(&content)
        .map_err(|e| AppError::Internal(format!("解析目录图谱数据失败: {}", e)))?;

    info!(
        "返回目录图谱 {}: {} 节点, {} 边",
        if req.dir_path.is_empty() { "(根目录)" } else { &req.dir_path },
        graph_data.nodes.len(),
        graph_data.edges.len()
    );

    Ok(Json(graph_data))
}
