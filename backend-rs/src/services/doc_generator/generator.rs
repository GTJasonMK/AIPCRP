//! 文档生成器
//!
//! 负责调用 LLM 生成文档并保存到文件

use chrono::Local;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, warn};

use super::prompts;
use super::types::{DirGraphData, DocGenConfig, FileGraphData, FileNode, LlmGraphRawData};
use crate::llm::{ChatMessage, ChatOptions, CollectMode, LlmClient};

/// 文件分析结果：包含文档内容和可选的图谱数据
pub struct FileAnalysisResult {
    /// 文档内容（不含图谱数据标记）
    pub doc_content: String,
    /// 图谱数据（如果解析成功）
    pub graph_data: Option<FileGraphData>,
}

/// 目录分析结果：包含文档内容和可选的图谱数据
pub struct DirAnalysisResult {
    /// 文档内容（不含图谱数据标记）
    pub doc_content: String,
    /// 图谱数据（如果解析成功）
    pub graph_data: Option<DirGraphData>,
}

/// 文档生成器
pub struct DocumentGenerator {
    /// 文档根目录
    docs_root: PathBuf,
    /// 配置
    config: DocGenConfig,
}

impl DocumentGenerator {
    /// 创建新的文档生成器
    pub fn new(docs_root: PathBuf, config: DocGenConfig) -> Self {
        Self { docs_root, config }
    }

    /// 获取文件的文档路径
    ///
    /// 例如: src/utils/helper.py -> docs_root/src/utils/helper.py.md
    pub fn get_file_doc_path(&self, node: &FileNode) -> PathBuf {
        let doc_name = format!("{}.md", node.name);
        let parent = Path::new(&node.relative_path).parent();

        match parent {
            Some(p) if !p.as_os_str().is_empty() => self.docs_root.join(p).join(doc_name),
            _ => self.docs_root.join(doc_name),
        }
    }

    /// 获取目录的文档路径
    ///
    /// 例如: src/utils -> docs_root/src/utils/_dir_summary.md
    pub fn get_dir_doc_path(&self, node: &FileNode) -> PathBuf {
        if node.relative_path.is_empty() {
            // 根目录
            self.docs_root.join(&self.config.dir_summary_name)
        } else {
            self.docs_root
                .join(&node.relative_path)
                .join(&self.config.dir_summary_name)
        }
    }

    /// 获取节点的文档路径
    pub fn get_doc_path(&self, node: &FileNode) -> PathBuf {
        if node.is_file {
            self.get_file_doc_path(node)
        } else {
            self.get_dir_doc_path(node)
        }
    }

    /// 分析代码文件并生成文档（包含知识图谱数据提取）
    pub async fn analyze_file(
        &self,
        node: &FileNode,
        llm_client: &LlmClient,
        model: &str,
    ) -> Result<FileAnalysisResult, GeneratorError> {
        // 读取文件内容
        let content = fs::read_to_string(&node.path)
            .await
            .map_err(|e| GeneratorError::IoError(node.path.clone(), e))?;

        // 构建 prompt
        let prompt = prompts::format_code_analysis_prompt(&node.relative_path, &content);

        // 调用 LLM
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        let options = ChatOptions {
            temperature: Some(0.3),
            max_tokens: Some(8192), // 代码分析需要较大的 token 限制
            ..Default::default()
        };

        let result = llm_client
            .stream_and_collect(messages, model, options, CollectMode::ContentOnly)
            .await
            .map_err(|e| GeneratorError::LlmError(e.to_string()))?;

        // 解析响应，分离文档内容和图谱数据
        let (doc_content, raw_graph) = self.parse_llm_response_raw(&result.content, &node.relative_path);
        let graph_data = raw_graph.map(|raw| FileGraphData::new(node.relative_path.clone(), raw));

        Ok(FileAnalysisResult {
            doc_content,
            graph_data,
        })
    }

    /// 解析 LLM 响应，分离文档内容和原始图谱数据
    ///
    /// 查找 `<!-- GRAPH_DATA_START -->` 和 `<!-- GRAPH_DATA_END -->` 之间的 JSON 数据
    fn parse_llm_response_raw(&self, response: &str, path: &str) -> (String, Option<LlmGraphRawData>) {
        const GRAPH_START: &str = "<!-- GRAPH_DATA_START -->";
        const GRAPH_END: &str = "<!-- GRAPH_DATA_END -->";

        // 查找图谱数据标记
        let start_pos = response.find(GRAPH_START);
        let end_pos = response.find(GRAPH_END);

        // 调试日志：显示是否找到图谱标记
        if start_pos.is_none() || end_pos.is_none() {
            info!("[{}] LLM 响应中未找到图谱数据标记 (GRAPH_DATA_START: {}, GRAPH_DATA_END: {})",
                path,
                start_pos.is_some(),
                end_pos.is_some()
            );
        }

        match (start_pos, end_pos) {
            (Some(start), Some(end)) if start < end => {
                // 提取文档内容（去除图谱数据部分）
                let doc_content = format!(
                    "{}{}",
                    response[..start].trim_end(),
                    response[end + GRAPH_END.len()..].trim_start()
                );

                // 提取图谱 JSON
                let graph_section = &response[start + GRAPH_START.len()..end];

                // 在图谱部分中查找 JSON（可能被 ```json 包裹）
                let json_str = self.extract_json_from_section(graph_section);

                match json_str {
                    Some(json) => {
                        match serde_json::from_str::<LlmGraphRawData>(&json) {
                            Ok(raw_data) => {
                                info!("成功解析 {} 的知识图谱: {} 节点, {} 边",
                                    path, raw_data.nodes.len(), raw_data.edges.len());
                                (doc_content, Some(raw_data))
                            }
                            Err(e) => {
                                warn!("解析 {} 的图谱 JSON 失败: {}", path, e);
                                (response.to_string(), None)
                            }
                        }
                    }
                    None => {
                        warn!("{} 的图谱标记中未找到有效 JSON", path);
                        (response.to_string(), None)
                    }
                }
            }
            _ => {
                // 没有找到图谱数据标记，返回原始响应
                debug!("{} 的响应中未找到图谱数据标记", path);
                (response.to_string(), None)
            }
        }
    }

    /// 从图谱部分提取 JSON 字符串
    ///
    /// 支持以下格式：
    /// 1. 直接的 JSON: `{ ... }`
    /// 2. 被 markdown 代码块包裹: ` ```json { ... } ``` `
    fn extract_json_from_section(&self, section: &str) -> Option<String> {
        let trimmed = section.trim();

        // 尝试查找 ```json ... ``` 格式
        if let Some(start) = trimmed.find("```json") {
            let after_marker = &trimmed[start + 7..];
            if let Some(end) = after_marker.find("```") {
                let json = after_marker[..end].trim();
                return Some(json.to_string());
            }
        }

        // 尝试查找 ``` ... ``` 格式（没有 json 标记）
        if let Some(start) = trimmed.find("```") {
            let after_marker = &trimmed[start + 3..];
            // 跳过可能的语言标识符行
            let json_start = after_marker.find('{').unwrap_or(0);
            if let Some(end) = after_marker.rfind("```") {
                let json = after_marker[json_start..end].trim();
                return Some(json.to_string());
            }
        }

        // 尝试直接找到 JSON 对象
        if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                let json = &trimmed[start..=end];
                return Some(json.to_string());
            }
        }

        None
    }

    /// 获取文件的图谱数据路径
    ///
    /// 例如: src/utils/helper.py -> docs_root/src/utils/helper.py.graph.json
    pub fn get_file_graph_path(&self, node: &FileNode) -> PathBuf {
        let graph_name = format!("{}.graph.json", node.name);
        let parent = Path::new(&node.relative_path).parent();

        match parent {
            Some(p) if !p.as_os_str().is_empty() => self.docs_root.join(p).join(graph_name),
            _ => self.docs_root.join(graph_name),
        }
    }

    /// 保存文件图谱数据
    pub async fn save_file_graph(
        &self,
        node: &FileNode,
        graph_data: &FileGraphData,
    ) -> Result<PathBuf, GeneratorError> {
        let graph_path = self.get_file_graph_path(node);

        // 确保父目录存在
        if let Some(parent) = graph_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| GeneratorError::IoError(parent.to_path_buf(), e))?;
        }

        // 序列化并保存
        let json_content = serde_json::to_string_pretty(graph_data)
            .map_err(|e| GeneratorError::LlmError(format!("序列化图谱数据失败: {}", e)))?;

        fs::write(&graph_path, json_content)
            .await
            .map_err(|e| GeneratorError::IoError(graph_path.clone(), e))?;

        debug!("文件图谱已保存: {}", graph_path.display());
        Ok(graph_path)
    }

    /// 保存文件分析文档
    pub async fn save_file_summary(
        &self,
        node: &FileNode,
        summary: &str,
    ) -> Result<PathBuf, GeneratorError> {
        let doc_path = self.get_file_doc_path(node);
        let content = self.format_file_doc(node, summary);
        self.save_document(&doc_path, &content).await?;
        debug!("File summary saved: {}", doc_path.display());
        Ok(doc_path)
    }

    /// 生成目录总结（包含知识图谱数据提取）
    ///
    /// 在同一次 LLM 调用中同时生成目录文档和提取图谱数据
    pub async fn summarize_directory(
        &self,
        node: &FileNode,
        sub_documents: &str,
        llm_client: &LlmClient,
        model: &str,
    ) -> Result<DirAnalysisResult, GeneratorError> {
        let prompt = prompts::format_directory_summary_prompt(
            &node.name,
            &node.relative_path,
            sub_documents,
        );

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        let options = ChatOptions {
            temperature: Some(0.3),
            max_tokens: Some(8192),
            ..Default::default()
        };

        let result = llm_client
            .stream_and_collect(messages, model, options, CollectMode::ContentOnly)
            .await
            .map_err(|e| GeneratorError::LlmError(e.to_string()))?;

        // 解析响应，分离文档内容和图谱数据
        let (doc_content, raw_graph) = self.parse_llm_response_raw(&result.content, &node.relative_path);
        let graph_data = raw_graph.map(|raw| DirGraphData::new(node.relative_path.clone(), raw));

        Ok(DirAnalysisResult {
            doc_content,
            graph_data,
        })
    }

    /// 保存目录总结文档
    pub async fn save_dir_summary(
        &self,
        node: &FileNode,
        summary: &str,
    ) -> Result<PathBuf, GeneratorError> {
        let doc_path = self.get_dir_doc_path(node);
        let content = self.format_dir_doc(node, summary);
        self.save_document(&doc_path, &content).await?;
        debug!("Directory summary saved: {}", doc_path.display());
        Ok(doc_path)
    }

    /// 获取目录的图谱数据路径
    ///
    /// 例如: src/utils -> docs_root/src/utils/_dir.graph.json
    /// 根目录 -> docs_root/_dir.graph.json
    pub fn get_dir_graph_path(&self, node: &FileNode) -> PathBuf {
        if node.relative_path.is_empty() {
            self.docs_root.join("_dir.graph.json")
        } else {
            self.docs_root
                .join(&node.relative_path)
                .join("_dir.graph.json")
        }
    }

    /// 保存目录图谱数据
    pub async fn save_dir_graph(
        &self,
        node: &FileNode,
        graph_data: &DirGraphData,
    ) -> Result<PathBuf, GeneratorError> {
        let graph_path = self.get_dir_graph_path(node);

        // 确保父目录存在
        if let Some(parent) = graph_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| GeneratorError::IoError(parent.to_path_buf(), e))?;
        }

        // 序列化并保存
        let json_content = serde_json::to_string_pretty(graph_data)
            .map_err(|e| GeneratorError::LlmError(format!("序列化目录图谱数据失败: {}", e)))?;

        fs::write(&graph_path, json_content)
            .await
            .map_err(|e| GeneratorError::IoError(graph_path.clone(), e))?;

        debug!("目录图谱已保存: {}", graph_path.display());
        Ok(graph_path)
    }

    /// 生成 README
    pub async fn generate_readme(
        &self,
        project_name: &str,
        project_path: &str,
        all_documents: &str,
        llm_client: &LlmClient,
        model: &str,
    ) -> Result<String, GeneratorError> {
        let prompt =
            prompts::format_readme_prompt(project_name, project_path, all_documents);

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        let options = ChatOptions {
            temperature: Some(0.3),
            max_tokens: Some(16384), // README 需要更大的 token 限制
            ..Default::default()
        };

        let result = llm_client
            .stream_and_collect(messages, model, options, CollectMode::ContentOnly)
            .await
            .map_err(|e| GeneratorError::LlmError(e.to_string()))?;

        Ok(result.content)
    }

    /// 保存 README
    pub async fn save_readme(
        &self,
        project_name: &str,
        content: &str,
    ) -> Result<PathBuf, GeneratorError> {
        let doc_path = self.docs_root.join(&self.config.readme_name);
        let formatted = self.format_readme(project_name, content);
        self.save_document(&doc_path, &formatted).await?;
        info!("README saved: {}", doc_path.display());
        Ok(doc_path)
    }

    /// 生成阅读指南
    pub async fn generate_reading_guide(
        &self,
        project_name: &str,
        project_structure: &str,
        all_documents: &str,
        llm_client: &LlmClient,
        model: &str,
    ) -> Result<String, GeneratorError> {
        let prompt = prompts::format_reading_guide_prompt(
            project_name,
            project_structure,
            all_documents,
        );

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        let options = ChatOptions {
            temperature: Some(0.3),
            max_tokens: Some(16384),
            ..Default::default()
        };

        let result = llm_client
            .stream_and_collect(messages, model, options, CollectMode::ContentOnly)
            .await
            .map_err(|e| GeneratorError::LlmError(e.to_string()))?;

        Ok(result.content)
    }

    /// 保存阅读指南
    pub async fn save_reading_guide(
        &self,
        project_name: &str,
        content: &str,
    ) -> Result<PathBuf, GeneratorError> {
        let doc_path = self.docs_root.join(&self.config.reading_guide_name);
        let formatted = self.format_reading_guide(project_name, content);
        self.save_document(&doc_path, &formatted).await?;
        info!("Reading guide saved: {}", doc_path.display());
        Ok(doc_path)
    }

    /// 读取文档内容
    pub async fn read_document(&self, doc_path: &Path) -> Result<String, GeneratorError> {
        fs::read_to_string(doc_path)
            .await
            .map_err(|e| GeneratorError::IoError(doc_path.to_path_buf(), e))
    }

    /// 读取子节点的所有文档并合并
    pub async fn read_child_summaries(&self, node: &FileNode) -> Result<String, GeneratorError> {
        let mut summaries = Vec::new();

        for child in &node.children {
            if let Some(doc_path) = &child.doc_path {
                match self.read_document(Path::new(doc_path)).await {
                    Ok(content) => {
                        summaries.push(format!("### {}\n\n{}", child.name, content));
                    }
                    Err(e) => {
                        error!("Failed to read child node document {}: {}", doc_path, e);
                    }
                }
            }
        }

        Ok(summaries.join("\n\n---\n\n"))
    }

    /// 保存文档到文件
    async fn save_document(&self, path: &Path, content: &str) -> Result<(), GeneratorError> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| GeneratorError::IoError(parent.to_path_buf(), e))?;
        }

        // 写入文件
        let mut file = fs::File::create(path)
            .await
            .map_err(|e| GeneratorError::IoError(path.to_path_buf(), e))?;

        file.write_all(content.as_bytes())
            .await
            .map_err(|e| GeneratorError::IoError(path.to_path_buf(), e))?;

        Ok(())
    }

    /// 格式化文件文档
    fn format_file_doc(&self, node: &FileNode, summary: &str) -> String {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S");
        format!(
            r#"# 文件分析: {}

**源文件**: `{}`
**生成时间**: {}

---

{}
"#,
            node.name, node.relative_path, now, summary
        )
    }

    /// 格式化目录文档
    fn format_dir_doc(&self, node: &FileNode, summary: &str) -> String {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S");
        let path_display = if node.relative_path.is_empty() {
            &node.name
        } else {
            &node.relative_path
        };

        format!(
            r#"# 目录分析: {}

**目录路径**: `{}`
**子文件数**: {}
**子目录数**: {}
**生成时间**: {}

---

{}
"#,
            node.name,
            path_display,
            node.file_count(),
            node.dir_count(),
            now,
            summary
        )
    }

    /// 格式化 README
    fn format_readme(&self, _project_name: &str, summary: &str) -> String {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S");
        format!(
            r#"{}

---

*本文档由 CodeSummaryAgent (Rust) 自动生成*
*生成时间: {}*
"#,
            summary, now
        )
    }

    /// 格式化阅读指南
    fn format_reading_guide(&self, project_name: &str, content: &str) -> String {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S");
        format!(
            r#"# {} - 文档阅读顺序指南

> 本指南帮助你按照合理的顺序阅读项目文档，快速理解项目结构和核心逻辑。

---

{}

---

*本文档由 CodeSummaryAgent (Rust) 自动生成*
*生成时间: {}*
"#,
            project_name, content, now
        )
    }

    /// 获取文档根目录
    pub fn docs_root(&self) -> &Path {
        &self.docs_root
    }
}

/// 生成器错误类型
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error("IO错误 ({0}): {1}")]
    IoError(PathBuf, #[source] std::io::Error),

    #[error("LLM调用错误: {0}")]
    LlmError(String),
}

/// 生成项目结构字符串（用于 Prompt）
pub fn format_project_structure(root: &FileNode, indent: usize) -> String {
    let mut result = String::new();
    let prefix = "  ".repeat(indent);

    if root.is_file {
        result.push_str(&format!("{}{}\n", prefix, root.name));
    } else {
        if indent > 0 {
            result.push_str(&format!("{}{}/\n", prefix, root.name));
        }
        for child in &root.children {
            result.push_str(&format_project_structure(child, indent + 1));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_doc_path() {
        let generator = DocumentGenerator::new(
            PathBuf::from("/docs"),
            DocGenConfig::default(),
        );

        let node = FileNode::new_file(
            "main.py".to_string(),
            PathBuf::from("/src/main.py"),
            "src/main.py".to_string(),
            1,
        );

        let doc_path = generator.get_file_doc_path(&node);
        assert_eq!(doc_path, PathBuf::from("/docs/src/main.py.md"));
    }

    #[test]
    fn test_get_dir_doc_path() {
        let generator = DocumentGenerator::new(
            PathBuf::from("/docs"),
            DocGenConfig::default(),
        );

        let node = FileNode::new_dir(
            "utils".to_string(),
            PathBuf::from("/src/utils"),
            "src/utils".to_string(),
            1,
        );

        let doc_path = generator.get_dir_doc_path(&node);
        assert_eq!(doc_path, PathBuf::from("/docs/src/utils/_dir_summary.md"));
    }

    #[test]
    fn test_format_project_structure() {
        let mut root = FileNode::new_dir(
            "project".to_string(),
            PathBuf::from("/project"),
            "".to_string(),
            0,
        );

        root.children.push(FileNode::new_file(
            "main.py".to_string(),
            PathBuf::from("/project/main.py"),
            "main.py".to_string(),
            1,
        ));

        let structure = format_project_structure(&root, 0);
        assert!(structure.contains("main.py"));
    }
}
