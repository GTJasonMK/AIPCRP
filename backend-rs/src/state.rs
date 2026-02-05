//! 应用状态管理
//!
//! 定义在请求处理器之间共享的状态。

use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::services::doc_generator::{SharedDocTask, WsDocMessage};

/// 已完成路径的类型
#[derive(Clone)]
pub enum CompletedPathType {
    File(String),
    Dir(String),
}

/// 正在处理中的路径类型
#[derive(Clone)]
pub enum InProgressPathType {
    File(String),
    Dir(String),
}

/// 任务状态，包含任务、广播通道和历史消息
pub struct TaskState {
    pub task: SharedDocTask,
    pub tx: broadcast::Sender<WsDocMessage>,
    /// 已完成的文件/目录路径，用于 WebSocket 连接时重放
    pub completed_paths: RwLock<Vec<CompletedPathType>>,
    /// 正在处理中的文件/目录路径（已发送 Started 但未 Completed）
    pub in_progress_files: RwLock<HashSet<String>>,
    pub in_progress_dirs: RwLock<HashSet<String>>,
}

impl TaskState {
    pub fn new(task: SharedDocTask, tx: broadcast::Sender<WsDocMessage>) -> Self {
        Self {
            task,
            tx,
            completed_paths: RwLock::new(Vec::new()),
            in_progress_files: RwLock::new(HashSet::new()),
            in_progress_dirs: RwLock::new(HashSet::new()),
        }
    }

    /// 记录文件开始处理
    pub fn mark_file_started(&self, path: String) {
        self.in_progress_files.write().insert(path);
    }

    /// 记录已完成的文件
    pub fn mark_file_completed(&self, path: String) {
        self.in_progress_files.write().remove(&path);
        self.completed_paths.write().push(CompletedPathType::File(path));
    }

    /// 记录目录开始处理
    pub fn mark_dir_started(&self, path: String) {
        self.in_progress_dirs.write().insert(path);
    }

    /// 记录已完成的目录
    pub fn mark_dir_completed(&self, path: String) {
        self.in_progress_dirs.write().remove(&path);
        self.completed_paths.write().push(CompletedPathType::Dir(path));
    }

    /// 获取所有已完成的路径
    pub fn get_completed_paths(&self) -> Vec<CompletedPathType> {
        self.completed_paths.read().clone()
    }

    /// 获取所有正在处理中的路径
    pub fn get_in_progress_paths(&self) -> Vec<InProgressPathType> {
        let mut result = Vec::new();
        for path in self.in_progress_files.read().iter() {
            result.push(InProgressPathType::File(path.clone()));
        }
        for path in self.in_progress_dirs.read().iter() {
            result.push(InProgressPathType::Dir(path.clone()));
        }
        result
    }
}

/// 文档生成任务注册表
pub type DocTaskRegistry = DashMap<String, Arc<TaskState>>;

/// 应用共享状态
///
/// 使用 Arc 包裹以便在多个处理器之间安全共享
#[derive(Clone)]
pub struct AppState {
    /// 文档生成任务注册表
    pub doc_tasks: Arc<DocTaskRegistry>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new() -> Self {
        Self {
            doc_tasks: Arc::new(DashMap::new()),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// 创建可共享的应用状态
pub fn create_shared_state() -> Arc<AppState> {
    Arc::new(AppState::new())
}
