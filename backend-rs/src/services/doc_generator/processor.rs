//! 层级处理器
//!
//! 主调度器，负责协调文件和目录的处理顺序

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, Semaphore};
use tracing::{error, info, warn};
use futures::stream::{self, StreamExt};
use chrono::Local;

use super::checkpoint::CheckpointService;
use super::generator::{format_project_structure, DocumentGenerator};
use super::scanner::DirectoryScanner;
use super::types::{
    DirGraphData, DocGenConfig, DocTask, FileGraphData, FileNode, LlmGraphEdge, LlmGraphNode,
    NodeStatus, ProjectGraphData, SharedDocTask, TaskStatus, WsDocMessage,
};
use crate::llm::LlmClient;

/// 合并的节点任务类型（文件或目录）
#[derive(Clone)]
enum NodeTask {
    File { name: String, relative_path: String, path: PathBuf },
    Dir { name: String, relative_path: String, path: PathBuf },
}

/// 层级处理器
pub struct LevelProcessor {
    /// 文件树根节点（使用 Arc<RwLock> 支持并行更新）
    root: Arc<RwLock<FileNode>>,
    /// 断点服务（使用 Arc<RwLock> 支持并行访问）
    checkpoint: Arc<RwLock<CheckpointService>>,
    /// 文档生成器（使用 Arc 支持并行读取）
    doc_generator: Arc<DocumentGenerator>,
    /// LLM 客户端
    llm_client: Arc<LlmClient>,
    /// 模型名称
    model: String,
    /// 配置
    config: DocGenConfig,
    /// 进度广播通道
    progress_tx: broadcast::Sender<WsDocMessage>,
    /// 并行控制信号量
    semaphore: Arc<Semaphore>,
}

impl LevelProcessor {
    /// 创建新的层级处理器
    pub fn new(
        root: FileNode,
        checkpoint: CheckpointService,
        doc_generator: DocumentGenerator,
        llm_client: Arc<LlmClient>,
        model: String,
        config: DocGenConfig,
    ) -> (Self, broadcast::Receiver<WsDocMessage>) {
        let (progress_tx, progress_rx) = broadcast::channel(100);

        // 限制并行度（最小1，最大10）
        let concurrency = config.concurrency.clamp(1, 10);
        info!("Document generation concurrency: {}", concurrency);

        let processor = Self {
            root: Arc::new(RwLock::new(root)),
            checkpoint: Arc::new(RwLock::new(checkpoint)),
            doc_generator: Arc::new(doc_generator),
            llm_client,
            model,
            config,
            progress_tx,
            semaphore: Arc::new(Semaphore::new(concurrency)),
        };

        (processor, progress_rx)
    }

    /// 订阅进度消息
    pub fn subscribe(&self) -> broadcast::Receiver<WsDocMessage> {
        self.progress_tx.subscribe()
    }

    /// 处理所有层级
    ///
    /// 核心逻辑：按深度从深到浅处理，每一层同时处理该层的文件和目录（并发）
    /// 这样当处理某个目录时，它的所有子节点（文件+子目录）的文档都已完成
    pub async fn process_all_levels(&self, task: SharedDocTask) -> Result<(), ProcessorError> {
        // 更新任务状态
        {
            let mut t = task.write().await;
            t.start();
            let root = self.root.read().await;
            t.stats.total_files = root.file_count();
            t.stats.total_dirs = root.get_all_dirs().len();
        }

        // 按深度统一处理文件和目录
        info!("Starting level-by-level processing...");
        self.process_by_depth(&task).await?;

        // 生成最终文档
        info!("Generating final documents...");
        self.generate_final_docs(&task).await?;

        // 保存最终断点
        self.checkpoint.write().await.save_checkpoint().await.map_err(|e| {
            ProcessorError::CheckpointError(e.to_string())
        })?;

        // 更新任务状态为完成
        {
            let mut t = task.write().await;
            t.complete();
        }

        // 发送完成消息
        let stats = task.read().await.stats.clone();
        let _ = self.progress_tx.send(WsDocMessage::Completed { stats });

        Ok(())
    }

    /// 按深度处理所有节点（文件+目录统一处理）
    ///
    /// 处理顺序：
    /// 1. 收集所有节点并按深度分组
    /// 2. 从最深层开始，逐层向上处理
    /// 3. 每层内：先并发处理文件，再并发处理目录
    ///    （目录需要读取子节点文档，所以同层内目录要等文件完成）
    async fn process_by_depth(&self, task: &SharedDocTask) -> Result<(), ProcessorError> {
        // 收集所有节点信息
        #[derive(Clone)]
        struct NodeInfo {
            name: String,
            relative_path: String,
            path: PathBuf,
            depth: u32,
            is_file: bool,
        }

        let all_nodes: Vec<NodeInfo> = {
            let root = self.root.read().await;
            let mut nodes = Vec::new();

            // 收集所有文件
            for file in root.get_all_files() {
                nodes.push(NodeInfo {
                    name: file.name.clone(),
                    relative_path: file.relative_path.clone(),
                    path: file.path.clone(),
                    depth: file.depth,
                    is_file: true,
                });
            }

            // 收集所有目录
            for dir in root.get_all_dirs() {
                nodes.push(NodeInfo {
                    name: dir.name.clone(),
                    relative_path: dir.relative_path.clone(),
                    path: dir.path.clone(),
                    depth: dir.depth,
                    is_file: false,
                });
            }

            nodes
        };

        let total_nodes = all_nodes.len();
        if total_nodes == 0 {
            info!("No nodes to process");
            return Ok(());
        }

        // 按深度分组
        let mut depth_groups: std::collections::HashMap<u32, (Vec<NodeInfo>, Vec<NodeInfo>)> =
            std::collections::HashMap::new();

        for node in all_nodes {
            let entry = depth_groups.entry(node.depth).or_insert((Vec::new(), Vec::new()));
            if node.is_file {
                entry.0.push(node);  // 文件
            } else {
                entry.1.push(node);  // 目录
            }
        }

        // 获取所有深度并降序排列（最深的先处理）
        let mut depths: Vec<u32> = depth_groups.keys().cloned().collect();
        depths.sort_by(|a, b| b.cmp(a));

        info!("Processing {} nodes in {} depth levels, concurrency: {}",
              total_nodes, depths.len(), self.config.concurrency);

        let processed_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        // 按深度从深到浅处理
        for depth in depths {
            // 检查是否已取消
            if task.read().await.status == TaskStatus::Cancelled {
                return Err(ProcessorError::Cancelled);
            }

            let (files_at_depth, dirs_at_depth) = depth_groups.remove(&depth).unwrap_or_default();
            info!("Processing depth {}: {} files, {} directories",
                  depth, files_at_depth.len(), dirs_at_depth.len());

            // 将文件和目录合并成一个交错的任务列表
            // 这样可以确保文件和目录真正并发处理，而不是先处理完所有文件再处理目录
            let mut merged_tasks: Vec<NodeTask> = Vec::new();
            let mut file_iter = files_at_depth.into_iter();
            let mut dir_iter = dirs_at_depth.into_iter();

            // 交错合并文件和目录任务
            loop {
                let file = file_iter.next();
                let dir = dir_iter.next();

                if file.is_none() && dir.is_none() {
                    break;
                }

                if let Some(f) = file {
                    merged_tasks.push(NodeTask::File {
                        name: f.name,
                        relative_path: f.relative_path,
                        path: f.path,
                    });
                }
                if let Some(d) = dir {
                    merged_tasks.push(NodeTask::Dir {
                        name: d.name,
                        relative_path: d.relative_path,
                        path: d.path,
                    });
                }
            }

            // 使用单一流统一处理所有任务
            self.process_merged_batch(task, merged_tasks, &processed_count, total_nodes).await?;

            // 每层处理完保存断点
            let _ = self.checkpoint.write().await.save_checkpoint().await;
        }

        Ok(())
    }

    /// 并发处理合并后的文件和目录任务
    ///
    /// 将文件和目录放在同一个流中统一调度，确保真正的交错并发处理
    async fn process_merged_batch(
        &self,
        task: &SharedDocTask,
        tasks: Vec<NodeTask>,
        processed_count: &Arc<std::sync::atomic::AtomicUsize>,
        total_nodes: usize,
    ) -> Result<(), ProcessorError> {
        let task_stream = stream::iter(tasks.into_iter());

        task_stream
            .for_each_concurrent(self.config.concurrency, |node_task| {
                let task = task.clone();
                let semaphore = self.semaphore.clone();
                let checkpoint = self.checkpoint.clone();
                let doc_generator = self.doc_generator.clone();
                let llm_client = self.llm_client.clone();
                let model = self.model.clone();
                let progress_tx = self.progress_tx.clone();
                let root = self.root.clone();
                let processed_count = processed_count.clone();

                async move {
                    // 获取信号量许可
                    let _permit = semaphore.acquire().await.unwrap();

                    // 检查是否已取消或已失败（快速失败机制）
                    {
                        let t = task.read().await;
                        if t.status == TaskStatus::Cancelled || t.status == TaskStatus::Failed {
                            return;
                        }
                    }

                    match node_task {
                        NodeTask::File { name, relative_path, path } => {
                            Self::process_single_file(
                                &task, &checkpoint, &doc_generator, &llm_client, &model,
                                &progress_tx, &root, &processed_count, total_nodes,
                                name, relative_path, path,
                            ).await;
                        }
                        NodeTask::Dir { name, relative_path, path } => {
                            Self::process_single_dir(
                                &task, &checkpoint, &doc_generator, &llm_client, &model,
                                &progress_tx, &root, &processed_count, total_nodes,
                                name, relative_path, path,
                            ).await;
                        }
                    }
                }
            })
            .await;

        // 检查是否被取消或失败
        let task_guard = task.read().await;
        if task_guard.status == TaskStatus::Cancelled {
            return Err(ProcessorError::Cancelled);
        }
        if task_guard.status == TaskStatus::Failed {
            let error_msg = task_guard.error.clone().unwrap_or_else(|| "Unknown error".to_string());
            return Err(ProcessorError::GeneratorError(error_msg));
        }

        Ok(())
    }

    /// 处理单个文件
    async fn process_single_file(
        task: &SharedDocTask,
        checkpoint: &Arc<RwLock<CheckpointService>>,
        doc_generator: &Arc<DocumentGenerator>,
        llm_client: &Arc<LlmClient>,
        model: &str,
        progress_tx: &broadcast::Sender<WsDocMessage>,
        root: &Arc<RwLock<FileNode>>,
        processed_count: &Arc<std::sync::atomic::AtomicUsize>,
        total_nodes: usize,
        name: String,
        relative_path: String,
        path: PathBuf,
    ) {
        // 检查是否已完成（断点续传）- 验证文档文件实际存在
        if checkpoint.write().await.verify_file_completed(&relative_path).await {
            info!("Skipping completed file: {}", relative_path);
            let _ = progress_tx.send(WsDocMessage::FileCompleted {
                path: relative_path.clone(),
            });
            {
                let mut t = task.write().await;
                t.stats.processed_files += 1;
                t.stats.skipped_count += 1;
            }
            processed_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return;
        }

        // 发送文件开始处理消息
        let _ = progress_tx.send(WsDocMessage::FileStarted {
            path: relative_path.clone(),
        });

        // 更新当前处理文件
        {
            let mut t = task.write().await;
            t.current_file = Some(relative_path.clone());
        }

        // 发送进度消息
        let current = processed_count.load(std::sync::atomic::Ordering::Relaxed);
        let progress = (current as f32 / total_nodes as f32) * 90.0;
        let _ = progress_tx.send(WsDocMessage::Progress {
            progress,
            current_file: Some(relative_path.clone()),
            stats: task.read().await.stats.clone(),
        });

        info!("Analyzing file: {}", relative_path);

        // 构造 FileNode 用于分析
        let file_node = FileNode::new_file(name.clone(), path.clone(), relative_path.clone(), 0);

        // 分析文件（返回 FileAnalysisResult，包含文档和图谱数据）
        match doc_generator.analyze_file(&file_node, llm_client, model).await {
            Ok(analysis_result) => {
                // 保存文档
                match doc_generator.save_file_summary(&file_node, &analysis_result.doc_content).await {
                    Ok(doc_path) => {
                        // 更新断点
                        {
                            let mut cp = checkpoint.write().await;
                            cp.mark_file_completed(&relative_path, &doc_path.to_string_lossy());
                        }

                        // 更新节点状态
                        {
                            let mut root_guard = root.write().await;
                            update_node_status_recursive(
                                &mut root_guard,
                                &relative_path,
                                NodeStatus::Completed,
                                Some(doc_path.to_string_lossy().to_string()),
                                true,
                            );
                        }

                        // 保存图谱数据（如果有）
                        if let Some(graph_data) = &analysis_result.graph_data {
                            info!("保存图谱数据: {} ({} 节点, {} 边)",
                                relative_path,
                                graph_data.nodes.len(),
                                graph_data.edges.len()
                            );
                            if let Err(e) = doc_generator.save_file_graph(&file_node, graph_data).await {
                                warn!("Failed to save graph data for {}: {}", relative_path, e);
                            }
                        } else {
                            info!("文件 {} 未提取到图谱数据", relative_path);
                        }

                        // 发送完成消息
                        let _ = progress_tx.send(WsDocMessage::FileCompleted {
                            path: relative_path.clone(),
                        });

                        // 更新统计
                        {
                            let mut t = task.write().await;
                            t.stats.processed_files += 1;
                            t.update_progress(None);
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to save document {}: {}", relative_path, e);
                        error!("{}", error_msg);
                        {
                            let mut root_guard = root.write().await;
                            update_node_status_recursive(
                                &mut root_guard,
                                &relative_path,
                                NodeStatus::Failed,
                                None,
                                true,
                            );
                        }
                        // 设置任务为失败状态，触发快速失败
                        {
                            let mut t = task.write().await;
                            t.fail(error_msg.clone());
                        }
                        let _ = progress_tx.send(WsDocMessage::Error { message: error_msg });
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to analyze file {}: {}", relative_path, e);
                error!("{}", error_msg);
                {
                    let mut root_guard = root.write().await;
                    update_node_status_recursive(
                        &mut root_guard,
                        &relative_path,
                        NodeStatus::Failed,
                        None,
                        true,
                    );
                }
                // 设置任务为失败状态，触发快速失败
                {
                    let mut t = task.write().await;
                    t.fail(error_msg.clone());
                }
                let _ = progress_tx.send(WsDocMessage::Error { message: error_msg });
            }
        }

        processed_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// 处理单个目录
    async fn process_single_dir(
        task: &SharedDocTask,
        checkpoint: &Arc<RwLock<CheckpointService>>,
        doc_generator: &Arc<DocumentGenerator>,
        llm_client: &Arc<LlmClient>,
        model: &str,
        progress_tx: &broadcast::Sender<WsDocMessage>,
        root: &Arc<RwLock<FileNode>>,
        processed_count: &Arc<std::sync::atomic::AtomicUsize>,
        total_nodes: usize,
        name: String,
        relative_path: String,
        path: PathBuf,
    ) {
        // 检查是否已完成（断点续传）- 验证文档文件实际存在
        if checkpoint.write().await.verify_dir_completed(&relative_path).await {
            info!("Skipping completed directory: {}", relative_path);
            let _ = progress_tx.send(WsDocMessage::DirCompleted {
                path: relative_path.clone(),
            });
            {
                let mut t = task.write().await;
                t.stats.processed_dirs += 1;
                t.stats.skipped_count += 1;
            }
            processed_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return;
        }

        // 发送目录开始处理消息
        let _ = progress_tx.send(WsDocMessage::DirStarted {
            path: relative_path.clone(),
        });

        // 发送进度消息
        let current = processed_count.load(std::sync::atomic::Ordering::Relaxed);
        let progress = (current as f32 / total_nodes as f32) * 90.0;
        let _ = progress_tx.send(WsDocMessage::Progress {
            progress,
            current_file: Some(relative_path.clone()),
            stats: task.read().await.stats.clone(),
        });

        info!("Processing directory: {}", relative_path);

        // 读取子节点文档
        let sub_documents = {
            let root_guard = root.read().await;
            if let Some(dir_node) = find_node_recursive_ref(&root_guard, &relative_path) {
                doc_generator.read_child_summaries(dir_node).await.unwrap_or_default()
            } else {
                String::new()
            }
        };

        if sub_documents.is_empty() {
            warn!("Directory {} has no sub-documents, skipping", relative_path);
            {
                let mut root_guard = root.write().await;
                update_node_status_recursive(&mut root_guard, &relative_path, NodeStatus::Skipped, None, false);
            }
            task.write().await.stats.skipped_count += 1;
            processed_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return;
        }

        // 创建目录节点用于生成总结
        let dir_node = {
            let root_guard = root.read().await;
            find_node_recursive_ref(&root_guard, &relative_path)
                .cloned()
                .unwrap_or_else(|| FileNode::new_dir(name.clone(), path.clone(), relative_path.clone(), 0))
        };

        // 生成目录总结（同一次 LLM 调用中提取文档和图谱）
        match doc_generator.summarize_directory(&dir_node, &sub_documents, llm_client, model).await {
            Ok(analysis_result) => {
                match doc_generator.save_dir_summary(&dir_node, &analysis_result.doc_content).await {
                    Ok(doc_path) => {
                        {
                            let mut cp = checkpoint.write().await;
                            cp.mark_dir_completed(&relative_path, &doc_path.to_string_lossy());
                        }
                        {
                            let mut root_guard = root.write().await;
                            update_node_status_recursive(
                                &mut root_guard,
                                &relative_path,
                                NodeStatus::Completed,
                                Some(doc_path.to_string_lossy().to_string()),
                                false,
                            );
                        }

                        // 保存目录图谱数据（如果有）
                        if let Some(graph_data) = &analysis_result.graph_data {
                            info!("保存目录图谱数据: {} ({} 节点, {} 边)",
                                relative_path,
                                graph_data.nodes.len(),
                                graph_data.edges.len()
                            );
                            if let Err(e) = doc_generator.save_dir_graph(&dir_node, graph_data).await {
                                warn!("Failed to save graph data for directory {}: {}", relative_path, e);
                            }
                        } else {
                            info!("目录 {} 未提取到图谱数据", relative_path);
                        }

                        let _ = progress_tx.send(WsDocMessage::DirCompleted {
                            path: relative_path.clone(),
                        });

                        task.write().await.stats.processed_dirs += 1;
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to save directory document {}: {}", relative_path, e);
                        error!("{}", error_msg);
                        {
                            let mut root_guard = root.write().await;
                            update_node_status_recursive(&mut root_guard, &relative_path, NodeStatus::Failed, None, false);
                        }
                        // 设置任务为失败状态，触发快速失败
                        {
                            let mut t = task.write().await;
                            t.fail(error_msg.clone());
                        }
                        let _ = progress_tx.send(WsDocMessage::Error { message: error_msg });
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to generate directory summary {}: {}", relative_path, e);
                error!("{}", error_msg);
                {
                    let mut root_guard = root.write().await;
                    update_node_status_recursive(&mut root_guard, &relative_path, NodeStatus::Failed, None, false);
                }
                // 设置任务为失败状态，触发快速失败
                {
                    let mut t = task.write().await;
                    t.fail(error_msg.clone());
                }
                let _ = progress_tx.send(WsDocMessage::Error { message: error_msg });
            }
        }

        processed_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// 生成最终文档（README、阅读指南等）
    async fn generate_final_docs(&self, task: &SharedDocTask) -> Result<(), ProcessorError> {
        let (project_name, project_path, project_structure) = {
            let root = self.root.read().await;
            (
                root.name.clone(),
                root.path.to_string_lossy().to_string(),
                format_project_structure(&root, 0),
            )
        };

        // 读取所有文档
        let all_documents = self.read_all_documents().await;

        // 生成 README
        if !self.checkpoint.read().await.is_readme_completed() {
            info!("Generating README...");
            let _ = self.progress_tx.send(WsDocMessage::Progress {
                progress: 92.0,
                current_file: Some("README.md".to_string()),
                stats: task.read().await.stats.clone(),
            });

            let content = self
                .doc_generator
                .generate_readme(&project_name, &project_path, &all_documents, &self.llm_client, &self.model)
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to generate README: {}", e);
                    let _ = self.progress_tx.send(WsDocMessage::Error { message: error_msg.clone() });
                    ProcessorError::GeneratorError(error_msg)
                })?;

            self.doc_generator.save_readme(&project_name, &content).await.map_err(|e| {
                let error_msg = format!("Failed to save README: {}", e);
                let _ = self.progress_tx.send(WsDocMessage::Error { message: error_msg.clone() });
                ProcessorError::GeneratorError(error_msg)
            })?;
            self.checkpoint.write().await.mark_readme_completed();
        }

        // 生成阅读指南
        if !self.checkpoint.read().await.is_reading_guide_completed() {
            info!("Generating reading guide...");
            let _ = self.progress_tx.send(WsDocMessage::Progress {
                progress: 96.0,
                current_file: Some("READING_GUIDE.md".to_string()),
                stats: task.read().await.stats.clone(),
            });

            let content = self
                .doc_generator
                .generate_reading_guide(
                    &project_name,
                    &project_structure,
                    &all_documents,
                    &self.llm_client,
                    &self.model,
                )
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to generate reading guide: {}", e);
                    let _ = self.progress_tx.send(WsDocMessage::Error { message: error_msg.clone() });
                    ProcessorError::GeneratorError(error_msg)
                })?;

            self.doc_generator.save_reading_guide(&project_name, &content).await.map_err(|e| {
                let error_msg = format!("Failed to save reading guide: {}", e);
                let _ = self.progress_tx.send(WsDocMessage::Error { message: error_msg.clone() });
                ProcessorError::GeneratorError(error_msg)
            })?;
            self.checkpoint.write().await.mark_reading_guide_completed();
        }

        // 聚合项目级图谱
        if !self.checkpoint.read().await.is_project_graph_completed() {
            info!("Aggregating project graph...");
            let _ = self.progress_tx.send(WsDocMessage::Progress {
                progress: 98.0,
                current_file: Some("_project_graph.json".to_string()),
                stats: task.read().await.stats.clone(),
            });

            self.aggregate_project_graph(&project_name).await.map_err(|e| {
                let error_msg = format!("Failed to aggregate project graph: {}", e);
                let _ = self.progress_tx.send(WsDocMessage::Error { message: error_msg.clone() });
                e
            })?;
            self.checkpoint.write().await.mark_project_graph_completed();
        }

        // 保存断点
        let _ = self.checkpoint.write().await.save_checkpoint().await;

        Ok(())
    }

    /// 聚合项目级图谱
    ///
    /// 遍历所有 .graph.json 文件（包括文件图谱和目录图谱），
    /// 合并节点和边，生成 _project_graph.json
    async fn aggregate_project_graph(&self, project_name: &str) -> Result<(), ProcessorError> {
        use tokio::fs;

        let docs_root = self.doc_generator.docs_root();
        let mut all_nodes: Vec<LlmGraphNode> = Vec::new();
        let mut all_edges: Vec<LlmGraphEdge> = Vec::new();
        let mut file_count = 0;
        let mut dir_count = 0;

        // 递归收集所有 .graph.json 文件
        let graph_files = self.collect_graph_files(docs_root).await;
        info!("Found {} graph files to aggregate", graph_files.len());

        for graph_path in &graph_files {
            let file_name = graph_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            match fs::read_to_string(graph_path).await {
                Ok(content) => {
                    if file_name == "_dir.graph.json" {
                        // 目录图谱
                        match serde_json::from_str::<DirGraphData>(&content) {
                            Ok(graph_data) => {
                                // 添加目录节点
                                all_nodes.push(LlmGraphNode {
                                    id: graph_data.dir_id.clone(),
                                    label: graph_data.dir_path.split('/').last()
                                        .unwrap_or_else(|| if graph_data.dir_path.is_empty() { project_name } else { &graph_data.dir_path })
                                        .to_string(),
                                    node_type: "directory".to_string(),
                                    line: None,
                                });

                                // 添加目录内的节点
                                all_nodes.extend(graph_data.nodes.clone());

                                // 添加边
                                all_edges.extend(graph_data.edges.clone());

                                // 根据导入声明生成跨模块依赖边
                                for import in &graph_data.imports {
                                    let target_file_id = self.resolve_import_target(&import.module, &graph_data.dir_path);
                                    if let Some(target_id) = target_file_id {
                                        all_edges.push(LlmGraphEdge {
                                            source: graph_data.dir_id.clone(),
                                            target: target_id,
                                            edge_type: "imports".to_string(),
                                        });
                                    }
                                }

                                dir_count += 1;
                            }
                            Err(e) => {
                                warn!("解析目录图谱文件 {} 失败: {}", graph_path.display(), e);
                            }
                        }
                    } else {
                        // 文件图谱
                        match serde_json::from_str::<FileGraphData>(&content) {
                            Ok(graph_data) => {
                                // 添加文件节点
                                all_nodes.push(LlmGraphNode {
                                    id: graph_data.file_id.clone(),
                                    label: graph_data.file_path.split('/').last()
                                        .unwrap_or(&graph_data.file_path).to_string(),
                                    node_type: "file".to_string(),
                                    line: None,
                                });

                                // 添加文件内的节点
                                all_nodes.extend(graph_data.nodes.clone());

                                // 添加边
                                all_edges.extend(graph_data.edges.clone());

                                // 根据导入声明生成跨文件依赖边
                                for import in &graph_data.imports {
                                    let target_file_id = self.resolve_import_target(&import.module, &graph_data.file_path);
                                    if let Some(target_id) = target_file_id {
                                        all_edges.push(LlmGraphEdge {
                                            source: graph_data.file_id.clone(),
                                            target: target_id,
                                            edge_type: "imports".to_string(),
                                        });
                                    }
                                }

                                file_count += 1;
                            }
                            Err(e) => {
                                warn!("解析文件图谱 {} 失败: {}", graph_path.display(), e);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("读取图谱文件 {} 失败: {}", graph_path.display(), e);
                }
            }
        }

        // 从文件树生成目录包含关系边
        {
            let root = self.root.read().await;
            self.generate_structure_edges(&root, &mut all_nodes, &mut all_edges);
        }

        // 去重节点（根据 ID）
        let mut seen_ids = std::collections::HashSet::new();
        all_nodes.retain(|node| seen_ids.insert(node.id.clone()));

        // 去重边（根据 source + target + type）
        let mut seen_edges = std::collections::HashSet::new();
        all_edges.retain(|edge| {
            seen_edges.insert(format!("{}->{}:{}", edge.source, edge.target, edge.edge_type))
        });

        // 创建项目图谱
        let project_graph = ProjectGraphData {
            project_name: project_name.to_string(),
            file_count,
            nodes: all_nodes,
            edges: all_edges,
            generated_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        };

        // 保存项目图谱
        let project_graph_path = docs_root.join("_project_graph.json");
        let json_content = serde_json::to_string_pretty(&project_graph)
            .map_err(|e| ProcessorError::GeneratorError(format!("序列化项目图谱失败: {}", e)))?;

        fs::write(&project_graph_path, json_content)
            .await
            .map_err(|e| ProcessorError::GeneratorError(format!("保存项目图谱失败: {}", e)))?;

        info!("项目图谱已保存: {} ({} 节点, {} 边, {} 文件, {} 目录)",
            project_graph_path.display(),
            project_graph.nodes.len(),
            project_graph.edges.len(),
            file_count,
            dir_count
        );

        Ok(())
    }

    /// 从文件树结构生成目录包含关系
    ///
    /// 遍历文件树，为每个目录生成：
    /// - 目录节点（如果还没有）
    /// - 目录包含子节点的 contains 边
    fn generate_structure_edges(
        &self,
        node: &FileNode,
        nodes: &mut Vec<LlmGraphNode>,
        edges: &mut Vec<LlmGraphEdge>,
    ) {
        if node.is_file {
            return;
        }

        let dir_id = if node.relative_path.is_empty() {
            "dir::".to_string()
        } else {
            format!("dir::{}", node.relative_path)
        };

        // 确保目录节点存在
        nodes.push(LlmGraphNode {
            id: dir_id.clone(),
            label: node.name.clone(),
            node_type: "directory".to_string(),
            line: None,
        });

        // 为每个直接子节点生成包含关系边
        for child in &node.children {
            let child_id = if child.is_file {
                format!("file::{}", child.relative_path)
            } else {
                format!("dir::{}", child.relative_path)
            };

            edges.push(LlmGraphEdge {
                source: dir_id.clone(),
                target: child_id,
                edge_type: "contains".to_string(),
            });

            // 递归处理子目录
            if !child.is_file {
                self.generate_structure_edges(child, nodes, edges);
            }
        }
    }

    /// 递归收集所有 .graph.json 文件
    async fn collect_graph_files(&self, dir: &std::path::Path) -> Vec<PathBuf> {
        use tokio::fs;

        let mut graph_files = Vec::new();

        if let Ok(mut entries) = fs::read_dir(dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    // 递归扫描子目录
                    let sub_files = Box::pin(self.collect_graph_files(&path)).await;
                    graph_files.extend(sub_files);
                } else if path.is_file() {
                    // 检查是否是 .graph.json 文件
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.ends_with(".graph.json") {
                            graph_files.push(path);
                        }
                    }
                }
            }
        }

        graph_files
    }

    /// 尝试解析导入的目标文件 ID
    ///
    /// 根据导入路径尝试匹配项目中的文件
    fn resolve_import_target(&self, module: &str, _source_file: &str) -> Option<String> {
        // 简单实现：将模块路径转换为文件 ID
        // 实际项目中可能需要更复杂的解析逻辑

        // 如果是相对导入（以 . 或 .. 开头）
        if module.starts_with('.') {
            // 暂时返回 None，因为解析相对路径需要更多上下文
            return None;
        }

        // 对于绝对导入，尝试构建文件 ID
        // 这里只是一个简单的启发式方法
        let normalized = module.replace('.', "/");
        Some(format!("file::{}", normalized))
    }

    /// 读取所有文档内容
    async fn read_all_documents(&self) -> String {
        let root = self.root.read().await;
        let documents = self.collect_documents_recursive(&root).await;
        documents.join("\n\n---\n\n")
    }

    /// 递归收集文档内容（使用 Box::pin 解决递归异步问题）
    fn collect_documents_recursive<'a>(
        &'a self,
        node: &'a FileNode,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<String>> + Send + 'a>> {
        Box::pin(async move {
            let mut documents = Vec::new();

            if let Some(doc_path) = &node.doc_path {
                if let Ok(content) = self.doc_generator.read_document(std::path::Path::new(doc_path)).await {
                    documents.push(format!("### {}\n\n{}", node.relative_path, content));
                }
            }

            for child in &node.children {
                let child_docs = self.collect_documents_recursive(child).await;
                documents.extend(child_docs);
            }

            documents
        })
    }

    /// 根据路径读取子节点文档
    async fn read_child_documents_by_path(&self, relative_path: &str) -> String {
        if let Some(dir_node) = self.find_dir_node(relative_path).await {
            if let Ok(content) = self.doc_generator.read_child_summaries(&dir_node).await {
                return content;
            }
        }
        String::new()
    }

    /// 查找目录节点
    async fn find_dir_node(&self, relative_path: &str) -> Option<FileNode> {
        let root = self.root.read().await;
        self.find_node_recursive(&root, relative_path)
    }

    fn find_node_recursive(&self, node: &FileNode, relative_path: &str) -> Option<FileNode> {
        if node.relative_path == relative_path {
            return Some(node.clone());
        }

        for child in &node.children {
            if let Some(found) = self.find_node_recursive(child, relative_path) {
                return Some(found);
            }
        }

        None
    }
}

/// 递归查找节点引用（用于在持有读锁时查找节点）
fn find_node_recursive_ref<'a>(node: &'a FileNode, relative_path: &str) -> Option<&'a FileNode> {
    if node.relative_path == relative_path {
        return Some(node);
    }

    for child in &node.children {
        if let Some(found) = find_node_recursive_ref(child, relative_path) {
            return Some(found);
        }
    }

    None
}

/// 递归更新节点状态（独立函数，避免借用冲突）
fn update_node_status_recursive(
    node: &mut FileNode,
    relative_path: &str,
    status: NodeStatus,
    doc_path: Option<String>,
    is_file: bool,
) {
    if node.relative_path == relative_path && node.is_file == is_file {
        node.status = status;
        node.doc_path = doc_path;
        return;
    }

    for child in &mut node.children {
        update_node_status_recursive(child, relative_path, status.clone(), doc_path.clone(), is_file);
    }
}

/// 处理器错误类型
#[derive(Debug, thiserror::Error)]
pub enum ProcessorError {
    #[error("Task cancelled")]
    Cancelled,

    #[error("Checkpoint error: {0}")]
    CheckpointError(String),

    #[error("Generator error: {0}")]
    GeneratorError(String),

    #[error("LLM error: {0}")]
    LlmError(String),
}

/// 文档生成服务（主入口）
pub struct DocGenService {
    config: DocGenConfig,
}

impl DocGenService {
    /// 创建新的文档生成服务
    pub fn new(config: DocGenConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(DocGenConfig::default())
    }

    /// 启动文档生成任务
    pub async fn start_generation(
        &self,
        source_path: PathBuf,
        docs_path: Option<PathBuf>,
        llm_client: Arc<LlmClient>,
        model: String,
        resume: bool,
    ) -> Result<(SharedDocTask, broadcast::Receiver<WsDocMessage>), ProcessorError> {
        // 计算文档路径：默认放在项目根目录下的 .docs 目录
        let docs_path = docs_path.unwrap_or_else(|| {
            source_path.join(".docs")
        });

        // 创建任务
        let task_id = uuid::Uuid::new_v4().to_string();
        let task = Arc::new(RwLock::new(DocTask::new(
            task_id,
            source_path.clone(),
            docs_path.clone(),
        )));

        // 扫描目录
        let scanner = DirectoryScanner::new(self.config.clone());
        let root = scanner
            .scan(&source_path)
            .map_err(|e| ProcessorError::GeneratorError(e.to_string()))?;

        // 创建断点服务
        let mut checkpoint =
            CheckpointService::new(source_path.clone(), docs_path.clone(), self.config.clone());
        checkpoint
            .initialize()
            .await
            .map_err(|e| ProcessorError::CheckpointError(e.to_string()))?;

        // 如果启用断点续传，加载断点
        if resume {
            let _ = checkpoint.load_checkpoint().await;
            let _ = checkpoint.scan_existing_docs().await;
        }

        // 创建文档生成器
        let doc_generator = DocumentGenerator::new(docs_path, self.config.clone());

        // 创建处理器
        let (processor, progress_rx) = LevelProcessor::new(
            root,
            checkpoint,
            doc_generator,
            llm_client,
            model,
            self.config.clone(),
        );

        // 在后台运行处理
        let task_clone = Arc::clone(&task);
        tokio::spawn(async move {
            if let Err(e) = processor.process_all_levels(task_clone.clone()).await {
                error!("Document generation failed: {}", e);
                let mut t = task_clone.write().await;
                t.fail(e.to_string());
            }
        });

        Ok((task, progress_rx))
    }
}
