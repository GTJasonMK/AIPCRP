//! 文档生成器类型定义
//!
//! 定义文件节点、任务状态等核心类型

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 节点处理状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    /// 待处理
    Pending,
    /// 处理中
    Processing,
    /// 已完成
    Completed,
    /// 处理失败
    Failed,
    /// 已跳过（如空目录）
    Skipped,
}

impl Default for NodeStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// 文件/目录节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    /// 节点名称（文件名或目录名）
    pub name: String,
    /// 完整路径
    pub path: PathBuf,
    /// 相对于源码根目录的路径
    pub relative_path: String,
    /// 是否为文件（否则为目录）
    pub is_file: bool,
    /// 子节点（仅目录有效）
    pub children: Vec<FileNode>,
    /// 目录深度（根目录为0）
    pub depth: u32,
    /// 生成的文档路径
    pub doc_path: Option<String>,
    /// 处理状态
    #[serde(default)]
    pub status: NodeStatus,
    /// 文件扩展名（仅文件有效）
    pub extension: Option<String>,
    /// 文件大小（字节）
    pub size: Option<u64>,
}

impl FileNode {
    /// 创建新的文件节点
    pub fn new_file(name: String, path: PathBuf, relative_path: String, depth: u32) -> Self {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase());

        Self {
            name,
            path,
            relative_path,
            is_file: true,
            children: Vec::new(),
            depth,
            doc_path: None,
            status: NodeStatus::Pending,
            extension,
            size: None,
        }
    }

    /// 创建新的目录节点
    pub fn new_dir(name: String, path: PathBuf, relative_path: String, depth: u32) -> Self {
        Self {
            name,
            path,
            relative_path,
            is_file: false,
            children: Vec::new(),
            depth,
            doc_path: None,
            status: NodeStatus::Pending,
            extension: None,
            size: None,
        }
    }

    /// 获取所有文件节点（递归）
    pub fn get_all_files(&self) -> Vec<&FileNode> {
        let mut files = Vec::new();
        self.collect_files(&mut files);
        files
    }

    fn collect_files<'a>(&'a self, files: &mut Vec<&'a FileNode>) {
        if self.is_file {
            files.push(self);
        } else {
            for child in &self.children {
                child.collect_files(files);
            }
        }
    }

    /// 获取所有目录节点（递归，按深度排序）
    pub fn get_all_dirs(&self) -> Vec<&FileNode> {
        let mut dirs = Vec::new();
        self.collect_dirs(&mut dirs);
        // 按深度降序排序（最深的先处理）
        dirs.sort_by(|a, b| b.depth.cmp(&a.depth));
        dirs
    }

    fn collect_dirs<'a>(&'a self, dirs: &mut Vec<&'a FileNode>) {
        if !self.is_file {
            dirs.push(self);
            for child in &self.children {
                child.collect_dirs(dirs);
            }
        }
    }

    /// 统计文件数量
    pub fn file_count(&self) -> usize {
        if self.is_file {
            1
        } else {
            self.children.iter().map(|c| c.file_count()).sum()
        }
    }

    /// 统计直接子目录数量
    pub fn dir_count(&self) -> usize {
        self.children.iter().filter(|c| !c.is_file).count()
    }
}

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// 待处理
    Pending,
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已取消
    Cancelled,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// 任务统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskStats {
    /// 总文件数
    pub total_files: usize,
    /// 已处理文件数
    pub processed_files: usize,
    /// 总目录数
    pub total_dirs: usize,
    /// 已处理目录数
    pub processed_dirs: usize,
    /// 失败数量
    pub failed_count: usize,
    /// 跳过数量
    pub skipped_count: usize,
    /// 开始时间（Unix时间戳，毫秒）
    pub start_time: Option<u64>,
    /// 结束时间（Unix时间戳，毫秒）
    pub end_time: Option<u64>,
}

impl TaskStats {
    /// 计算进度百分比
    pub fn progress(&self) -> f32 {
        let total = self.total_files + self.total_dirs;
        if total == 0 {
            return 0.0;
        }
        let processed = self.processed_files + self.processed_dirs;
        (processed as f32 / total as f32) * 100.0
    }

    /// 计算耗时（毫秒）
    pub fn elapsed_ms(&self) -> Option<u64> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => Some(end - start),
            (Some(start), None) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                Some(now - start)
            }
            _ => None,
        }
    }
}

/// 文档生成任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocTask {
    /// 任务ID
    pub id: String,
    /// 源码路径
    pub source_path: PathBuf,
    /// 文档输出路径
    pub docs_path: PathBuf,
    /// 任务状态
    pub status: TaskStatus,
    /// 进度百分比 (0-100)
    pub progress: f32,
    /// 当前处理的文件
    pub current_file: Option<String>,
    /// 错误信息
    pub error: Option<String>,
    /// 统计信息
    pub stats: TaskStats,
}

impl DocTask {
    /// 创建新任务
    pub fn new(id: String, source_path: PathBuf, docs_path: PathBuf) -> Self {
        Self {
            id,
            source_path,
            docs_path,
            status: TaskStatus::Pending,
            progress: 0.0,
            current_file: None,
            error: None,
            stats: TaskStats::default(),
        }
    }

    /// 标记任务开始
    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
        self.stats.start_time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );
    }

    /// 标记任务完成
    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.progress = 100.0;
        self.current_file = None;
        self.stats.end_time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );
    }

    /// 标记任务失败
    pub fn fail(&mut self, error: String) {
        self.status = TaskStatus::Failed;
        self.error = Some(error);
        self.stats.end_time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );
    }

    /// 标记任务取消
    pub fn cancel(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.stats.end_time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );
    }

    /// 更新进度
    pub fn update_progress(&mut self, current_file: Option<String>) {
        self.current_file = current_file;
        self.progress = self.stats.progress();
    }
}

/// 文档生成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocGenConfig {
    /// 文档目录后缀（默认 "_docs"）
    #[serde(default = "default_docs_suffix")]
    pub docs_suffix: String,

    /// 目录总结文件名（默认 "_dir_summary.md"）
    #[serde(default = "default_dir_summary_name")]
    pub dir_summary_name: String,

    /// README文件名（默认 "README.md"）
    #[serde(default = "default_readme_name")]
    pub readme_name: String,

    /// API文档文件名（默认 "API_DOC.md"）
    #[serde(default = "default_api_doc_name")]
    pub api_doc_name: String,

    /// 阅读指南文件名（默认 "READING_GUIDE.md"）
    #[serde(default = "default_reading_guide_name")]
    pub reading_guide_name: String,

    /// 忽略的目录模式
    #[serde(default = "default_ignore_patterns")]
    pub ignore_patterns: Vec<String>,

    /// 支持的文件扩展名
    #[serde(default = "default_supported_extensions")]
    pub supported_extensions: Vec<String>,

    /// 最大文件大小（字节，默认1MB）
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,

    /// 是否启用断点续传
    #[serde(default = "default_enable_checkpoint")]
    pub enable_checkpoint: bool,

    /// 并行处理数量（默认3，最大10）
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
}

fn default_docs_suffix() -> String {
    "_docs".to_string()
}

fn default_dir_summary_name() -> String {
    "_dir_summary.md".to_string()
}

fn default_readme_name() -> String {
    "README.md".to_string()
}

fn default_api_doc_name() -> String {
    "API_DOC.md".to_string()
}

fn default_reading_guide_name() -> String {
    "READING_GUIDE.md".to_string()
}

fn default_ignore_patterns() -> Vec<String> {
    vec![
        ".git".to_string(),
        ".docs".to_string(),
        "node_modules".to_string(),
        "__pycache__".to_string(),
        ".venv".to_string(),
        "venv".to_string(),
        "target".to_string(),
        "dist".to_string(),
        "build".to_string(),
        ".idea".to_string(),
        ".vscode".to_string(),
        ".next".to_string(),
        "out".to_string(),
        ".cache".to_string(),
        "*.pyc".to_string(),
        "*.pyo".to_string(),
        "*.so".to_string(),
        "*.dll".to_string(),
        "*.exe".to_string(),
    ]
}

fn default_supported_extensions() -> Vec<String> {
    vec![
        "py".to_string(),
        "js".to_string(),
        "ts".to_string(),
        "jsx".to_string(),
        "tsx".to_string(),
        "java".to_string(),
        "go".to_string(),
        "rs".to_string(),
        "c".to_string(),
        "cpp".to_string(),
        "h".to_string(),
        "hpp".to_string(),
        "cs".to_string(),
        "rb".to_string(),
        "php".to_string(),
        "swift".to_string(),
        "kt".to_string(),
        "scala".to_string(),
        "vue".to_string(),
        "svelte".to_string(),
    ]
}

fn default_max_file_size() -> u64 {
    1024 * 1024 // 1MB
}

fn default_enable_checkpoint() -> bool {
    true
}

fn default_concurrency() -> usize {
    3
}

impl Default for DocGenConfig {
    fn default() -> Self {
        Self {
            docs_suffix: default_docs_suffix(),
            dir_summary_name: default_dir_summary_name(),
            readme_name: default_readme_name(),
            api_doc_name: default_api_doc_name(),
            reading_guide_name: default_reading_guide_name(),
            ignore_patterns: default_ignore_patterns(),
            supported_extensions: default_supported_extensions(),
            max_file_size: default_max_file_size(),
            enable_checkpoint: default_enable_checkpoint(),
            concurrency: default_concurrency(),
        }
    }
}

/// WebSocket 进度消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsDocMessage {
    /// 进度更新
    Progress {
        progress: f32,
        current_file: Option<String>,
        stats: TaskStats,
    },
    /// 文件开始处理
    FileStarted { path: String },
    /// 文件处理完成
    FileCompleted { path: String },
    /// 目录开始处理
    DirStarted { path: String },
    /// 目录处理完成
    DirCompleted { path: String },
    /// 任务完成
    Completed { stats: TaskStats },
    /// 任务失败
    Error { message: String },
    /// 任务取消
    Cancelled,
}

/// 共享的任务状态（用于线程间通信）
pub type SharedDocTask = Arc<RwLock<DocTask>>;

// ============ 知识图谱相关类型 ============

/// LLM 提取的知识图谱节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmGraphNode {
    /// 节点ID，格式: `{type}::{file_path}::{name}` 或 `{type}::{file_path}::{class}::{method}`
    pub id: String,
    /// 显示标签
    pub label: String,
    /// 节点类型: class, function, method, interface, struct, enum, constant
    #[serde(rename = "type")]
    pub node_type: String,
    /// 代码行号（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

/// LLM 提取的知识图谱边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmGraphEdge {
    /// 源节点ID
    pub source: String,
    /// 目标节点ID
    pub target: String,
    /// 边类型: contains, imports, calls, inherits, implements
    #[serde(rename = "type")]
    pub edge_type: String,
}

/// 导入声明
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDeclaration {
    /// 导入的模块名
    pub module: String,
    /// 导入的具体项（可选）
    #[serde(default)]
    pub items: Vec<String>,
}

/// LLM 从响应中提取的原始图谱数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmGraphRawData {
    /// 节点列表
    #[serde(default)]
    pub nodes: Vec<LlmGraphNode>,
    /// 边列表
    #[serde(default)]
    pub edges: Vec<LlmGraphEdge>,
    /// 导入声明列表
    #[serde(default)]
    pub imports: Vec<ImportDeclaration>,
}

/// 单个文件的图谱数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileGraphData {
    /// 文件路径（相对路径）
    pub file_path: String,
    /// 文件节点ID
    pub file_id: String,
    /// 节点列表
    pub nodes: Vec<LlmGraphNode>,
    /// 边列表
    pub edges: Vec<LlmGraphEdge>,
    /// 导入声明列表
    pub imports: Vec<ImportDeclaration>,
}

impl FileGraphData {
    /// 创建新的文件图谱数据
    pub fn new(file_path: String, raw_data: LlmGraphRawData) -> Self {
        let file_id = format!("file::{}", file_path);
        Self {
            file_path,
            file_id,
            nodes: raw_data.nodes,
            edges: raw_data.edges,
            imports: raw_data.imports,
        }
    }
}

/// 单个目录的图谱数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirGraphData {
    /// 目录路径（相对路径）
    pub dir_path: String,
    /// 目录节点ID
    pub dir_id: String,
    /// 节点列表（包含子模块）
    pub nodes: Vec<LlmGraphNode>,
    /// 边列表（模块间关系）
    pub edges: Vec<LlmGraphEdge>,
    /// 导入声明列表
    pub imports: Vec<ImportDeclaration>,
}

impl DirGraphData {
    /// 创建新的目录图谱数据
    pub fn new(dir_path: String, raw_data: LlmGraphRawData) -> Self {
        let dir_id = if dir_path.is_empty() {
            "dir::".to_string()
        } else {
            format!("dir::{}", dir_path)
        };
        Self {
            dir_path,
            dir_id,
            nodes: raw_data.nodes,
            edges: raw_data.edges,
            imports: raw_data.imports,
        }
    }
}

/// 项目级聚合图谱
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectGraphData {
    /// 项目名称
    pub project_name: String,
    /// 包含的文件数量
    pub file_count: usize,
    /// 所有节点（包括文件节点）
    pub nodes: Vec<LlmGraphNode>,
    /// 所有边（包括跨文件依赖）
    pub edges: Vec<LlmGraphEdge>,
    /// 生成时间
    pub generated_at: String,
}

impl Default for LlmGraphRawData {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            imports: Vec::new(),
        }
    }
}
