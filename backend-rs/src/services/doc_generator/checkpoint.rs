//! 断点续传服务
//!
//! 管理文档生成的断点，支持中断后继续

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info};

use super::types::{DocGenConfig, FileNode, NodeStatus};

/// 断点数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointData {
    /// 已完成的文件路径集合
    pub completed_files: HashSet<String>,
    /// 已完成的目录路径集合
    pub completed_dirs: HashSet<String>,
    /// README 是否已完成
    pub readme_completed: bool,
    /// 阅读指南是否已完成
    pub reading_guide_completed: bool,
    /// API 文档是否已完成
    pub api_doc_completed: bool,
    /// 项目图谱是否已完成
    #[serde(default)]
    pub project_graph_completed: bool,
}

/// 断点续传服务
pub struct CheckpointService {
    /// 源码根目录
    source_root: PathBuf,
    /// 文档根目录
    docs_root: PathBuf,
    /// 配置
    config: DocGenConfig,
    /// 断点数据
    data: CheckpointData,
    /// 断点文件路径
    checkpoint_file: PathBuf,
    /// 文档路径映射（相对路径 -> 文档路径）
    doc_path_map: std::collections::HashMap<String, String>,
}

impl CheckpointService {
    /// 创建新的断点服务
    pub fn new(source_root: PathBuf, docs_root: PathBuf, config: DocGenConfig) -> Self {
        let checkpoint_file = docs_root.join(".checkpoint.json");

        Self {
            source_root,
            docs_root,
            config,
            data: CheckpointData::default(),
            checkpoint_file,
            doc_path_map: std::collections::HashMap::new(),
        }
    }

    /// 初始化断点服务
    pub async fn initialize(&mut self) -> Result<(), CheckpointError> {
        // 确保文档目录存在
        fs::create_dir_all(&self.docs_root)
            .await
            .map_err(|e| CheckpointError::IoError(self.docs_root.clone(), e))?;

        Ok(())
    }

    /// 加载断点文件
    pub async fn load_checkpoint(&mut self) -> Result<bool, CheckpointError> {
        if !self.checkpoint_file.exists() {
            debug!("Checkpoint file does not exist: {}", self.checkpoint_file.display());
            return Ok(false);
        }

        let content = fs::read_to_string(&self.checkpoint_file)
            .await
            .map_err(|e| CheckpointError::IoError(self.checkpoint_file.clone(), e))?;

        self.data = serde_json::from_str(&content)
            .map_err(|e| CheckpointError::ParseError(e.to_string()))?;

        info!(
            "Checkpoint loaded: {} files, {} directories",
            self.data.completed_files.len(),
            self.data.completed_dirs.len()
        );

        Ok(true)
    }

    /// 保存断点文件
    pub async fn save_checkpoint(&self) -> Result<(), CheckpointError> {
        let content = serde_json::to_string_pretty(&self.data)
            .map_err(|e| CheckpointError::SerializeError(e.to_string()))?;

        fs::write(&self.checkpoint_file, content)
            .await
            .map_err(|e| CheckpointError::IoError(self.checkpoint_file.clone(), e))?;

        debug!("Checkpoint saved");
        Ok(())
    }

    /// 扫描已存在的文档
    pub async fn scan_existing_docs(&mut self) -> Result<(), CheckpointError> {
        if !self.docs_root.exists() {
            return Ok(());
        }

        self.scan_docs_recursive(&self.docs_root.clone(), "").await?;

        info!(
            "Scanned {} existing documents",
            self.doc_path_map.len()
        );

        Ok(())
    }

    /// 递归扫描文档目录
    async fn scan_docs_recursive(
        &mut self,
        path: &Path,
        relative: &str,
    ) -> Result<(), CheckpointError> {
        let mut entries = fs::read_dir(path)
            .await
            .map_err(|e| CheckpointError::IoError(path.to_path_buf(), e))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| CheckpointError::IoError(path.to_path_buf(), e))?
        {
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // 跳过断点文件
            if name == ".checkpoint.json" {
                continue;
            }

            let entry_relative = if relative.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", relative, name)
            };

            if entry_path.is_dir() {
                // 递归扫描子目录
                Box::pin(self.scan_docs_recursive(&entry_path, &entry_relative)).await?;
            } else if entry_path.is_file() && name.ends_with(".md") {
                // 记录文档文件
                if name == self.config.dir_summary_name {
                    // 目录总结文档
                    let source_relative = if relative.is_empty() {
                        "".to_string()
                    } else {
                        relative.to_string()
                    };
                    self.doc_path_map.insert(
                        format!("dir:{}", source_relative),
                        entry_path.to_string_lossy().to_string(),
                    );
                    self.data.completed_dirs.insert(source_relative);
                } else if name != self.config.readme_name
                    && name != self.config.reading_guide_name
                    && name != self.config.api_doc_name
                {
                    // 文件文档（去掉 .md 后缀得到源文件名）
                    let source_name = name.strip_suffix(".md").unwrap_or(&name);
                    let source_relative = if relative.is_empty() {
                        source_name.to_string()
                    } else {
                        format!("{}/{}", relative, source_name)
                    };
                    self.doc_path_map.insert(
                        format!("file:{}", source_relative),
                        entry_path.to_string_lossy().to_string(),
                    );
                    self.data.completed_files.insert(source_relative);
                }
            }
        }

        Ok(())
    }

    /// 更新节点状态（根据断点恢复）
    pub fn update_node_status(&self, root: &mut FileNode) -> usize {
        let mut restored = 0;
        self.update_node_recursive(root, &mut restored);
        restored
    }

    fn update_node_recursive(&self, node: &mut FileNode, restored: &mut usize) {
        if node.is_file {
            // 检查文件是否已完成
            if self.data.completed_files.contains(&node.relative_path) {
                node.status = NodeStatus::Completed;
                // 恢复文档路径
                if let Some(doc_path) = self.doc_path_map.get(&format!("file:{}", node.relative_path)) {
                    node.doc_path = Some(doc_path.clone());
                }
                *restored += 1;
            }
        } else {
            // 先递归处理子节点
            for child in &mut node.children {
                self.update_node_recursive(child, restored);
            }

            // 检查目录是否已完成
            if self.data.completed_dirs.contains(&node.relative_path) {
                node.status = NodeStatus::Completed;
                // 恢复文档路径
                if let Some(doc_path) = self.doc_path_map.get(&format!("dir:{}", node.relative_path)) {
                    node.doc_path = Some(doc_path.clone());
                }
                *restored += 1;
            }
        }
    }

    /// 标记文件完成
    pub fn mark_file_completed(&mut self, relative_path: &str, doc_path: &str) {
        self.data.completed_files.insert(relative_path.to_string());
        self.doc_path_map.insert(
            format!("file:{}", relative_path),
            doc_path.to_string(),
        );
    }

    /// 标记目录完成
    pub fn mark_dir_completed(&mut self, relative_path: &str, doc_path: &str) {
        self.data.completed_dirs.insert(relative_path.to_string());
        self.doc_path_map.insert(
            format!("dir:{}", relative_path),
            doc_path.to_string(),
        );
    }

    /// 标记 README 完成
    pub fn mark_readme_completed(&mut self) {
        self.data.readme_completed = true;
    }

    /// 标记阅读指南完成
    pub fn mark_reading_guide_completed(&mut self) {
        self.data.reading_guide_completed = true;
    }

    /// 标记 API 文档完成
    pub fn mark_api_doc_completed(&mut self) {
        self.data.api_doc_completed = true;
    }

    /// 标记项目图谱完成
    pub fn mark_project_graph_completed(&mut self) {
        self.data.project_graph_completed = true;
    }

    /// 检查文件是否已完成
    pub fn is_file_completed(&self, relative_path: &str) -> bool {
        self.data.completed_files.contains(relative_path)
    }

    /// 检查目录是否已完成
    pub fn is_dir_completed(&self, relative_path: &str) -> bool {
        self.data.completed_dirs.contains(relative_path)
    }

    /// 检查 README 是否已完成
    pub fn is_readme_completed(&self) -> bool {
        self.data.readme_completed
    }

    /// 检查阅读指南是否已完成
    pub fn is_reading_guide_completed(&self) -> bool {
        self.data.reading_guide_completed
    }

    /// 检查 API 文档是否已完成
    pub fn is_api_doc_completed(&self) -> bool {
        self.data.api_doc_completed
    }

    /// 检查项目图谱是否已完成
    pub fn is_project_graph_completed(&self) -> bool {
        self.data.project_graph_completed
    }

    /// 获取文档路径
    pub fn get_doc_path(&self, key: &str) -> Option<&String> {
        self.doc_path_map.get(key)
    }

    /// 清除断点
    pub async fn clear(&mut self) -> Result<(), CheckpointError> {
        self.data = CheckpointData::default();
        self.doc_path_map.clear();

        if self.checkpoint_file.exists() {
            fs::remove_file(&self.checkpoint_file)
                .await
                .map_err(|e| CheckpointError::IoError(self.checkpoint_file.clone(), e))?;
        }

        info!("Checkpoint cleared");
        Ok(())
    }
}

/// 断点服务错误类型
#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    #[error("IO错误 ({0}): {1}")]
    IoError(PathBuf, #[source] std::io::Error),

    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("序列化错误: {0}")]
    SerializeError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_checkpoint_save_load() {
        let dir = TempDir::new().unwrap();
        let source_root = dir.path().join("source");
        let docs_root = dir.path().join("docs");

        fs::create_dir_all(&source_root).await.unwrap();
        fs::create_dir_all(&docs_root).await.unwrap();

        let mut service = CheckpointService::new(
            source_root,
            docs_root,
            DocGenConfig::default(),
        );

        service.initialize().await.unwrap();

        // 标记一些完成
        service.mark_file_completed("main.py", "/docs/main.py.md");
        service.mark_dir_completed("src", "/docs/src/_dir_summary.md");

        // 保存
        service.save_checkpoint().await.unwrap();

        // 创建新实例并加载
        let mut service2 = CheckpointService::new(
            dir.path().join("source"),
            dir.path().join("docs"),
            DocGenConfig::default(),
        );

        let loaded = service2.load_checkpoint().await.unwrap();
        assert!(loaded);
        assert!(service2.is_file_completed("main.py"));
        assert!(service2.is_dir_completed("src"));
    }
}
