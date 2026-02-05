//! 代码分析知识图谱类型定义

use serde::Serialize;
use std::collections::HashMap;

/// 支持分析的文件扩展名
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    ".py", ".js", ".jsx", ".ts", ".tsx", ".java", ".go",
    ".c", ".cpp", ".h", ".hpp", ".cs", ".rb", ".rs", ".vue",
];

/// 需要跳过的目录
pub const IGNORED_DIRS: &[&str] = &[
    ".git", "node_modules", "__pycache__", ".venv", "venv",
    "dist", "build", ".idea", ".vscode", ".next", "out",
    ".cache", "target", ".tox", "egg-info",
];

/// 图谱节点
#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number: Option<usize>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl GraphNode {
    pub fn file(id: impl Into<String>, label: impl Into<String>, file_path: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            node_type: "file".to_string(),
            file_path: Some(file_path.into()),
            line_number: None,
            metadata: HashMap::new(),
        }
    }

    pub fn directory(id: impl Into<String>, label: impl Into<String>, path: impl Into<String>) -> Self {
        let path_str = path.into();
        let mut metadata = HashMap::new();
        metadata.insert("full_path".to_string(), path_str.clone());
        Self {
            id: id.into(),
            label: label.into(),
            node_type: "directory".to_string(),
            file_path: Some(path_str),
            line_number: None,
            metadata,
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// 图谱边
#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub edge_type: String,
    #[serde(default)]
    pub label: String,
}

impl GraphEdge {
    pub fn new(source: impl Into<String>, target: impl Into<String>, edge_type: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            edge_type: edge_type.into(),
            label: label.into(),
        }
    }

    pub fn contains(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self::new(source, target, "contains", "defines")
    }

    pub fn imports(source: impl Into<String>, target: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(source, target, "imports", label)
    }

    pub fn inherits(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self::new(source, target, "inherits", "extends")
    }
}

/// 完整图谱数据
#[derive(Debug, Clone, Serialize, Default)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// 导入信息
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// 导入路径
    pub path: String,
    /// 显示名称
    pub display_name: String,
}
