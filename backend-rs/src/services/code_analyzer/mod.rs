//! 代码分析器主模块
//!
//! 分析源代码以生成知识图谱

mod generic;
mod go;
mod imports;
mod java;
mod javascript;
mod python;
pub mod types;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use types::{GraphData, GraphEdge, GraphNode, IGNORED_DIRS, SUPPORTED_EXTENSIONS};

/// 代码分析器
pub struct CodeAnalyzer {
    project_path: PathBuf,
}

impl CodeAnalyzer {
    /// 创建新的代码分析器
    pub fn new(project_path: impl Into<PathBuf>) -> Self {
        Self {
            project_path: project_path.into(),
        }
    }

    /// 生成项目级概览图谱（文件/模块依赖）
    pub fn analyze_project(&self) -> GraphData {
        let mut graph = GraphData::default();
        let mut file_map: HashMap<String, bool> = HashMap::new();

        // 收集所有源文件
        let source_files = self.collect_source_files();

        // 创建文件节点
        for file_path in &source_files {
            let rel_path = self.relative_path(file_path);
            let node_id = self.path_to_id(&rel_path);
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let ext_with_dot = format!(".{}", ext);

            let node = GraphNode::file(&node_id, file_path.file_name().unwrap().to_string_lossy(), &rel_path)
                .with_metadata("extension", &ext_with_dot)
                .with_metadata("directory", file_path.parent().map(|p| self.relative_path(p)).unwrap_or_default())
                .with_metadata("language", Self::ext_to_language(&ext_with_dot));

            graph.nodes.push(node);
            file_map.insert(rel_path.clone(), true);
        }

        // 分析导入关系
        for file_path in &source_files {
            let rel_path = self.relative_path(file_path);
            let source_id = self.path_to_id(&rel_path);
            let ext = format!(".{}", file_path.extension().and_then(|e| e.to_str()).unwrap_or(""));

            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let import_infos = imports::extract_imports(&content, &ext, &rel_path);
            for imp in import_infos {
                if let Some(resolved) = imports::resolve_import(&imp.path, &rel_path, &file_map) {
                    let target_id = self.path_to_id(&resolved);
                    graph.edges.push(GraphEdge::imports(&source_id, &target_id, &imp.display_name));
                }
            }
        }

        // 添加目录分组
        self.add_directory_groups(&mut graph, &source_files);

        graph
    }

    /// 生成模块级详细图谱
    pub fn analyze_module(&self, file_path: &str) -> GraphData {
        let mut graph = GraphData::default();
        let full_path = self.project_path.join(file_path);

        if !full_path.is_file() {
            return graph;
        }

        let content = match fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => return graph,
        };

        let ext = full_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let ext_with_dot = format!(".{}", ext);
        let lines: Vec<&str> = content.lines().collect();

        // 文件根节点
        let file_id = self.path_to_id(file_path);
        graph.nodes.push(GraphNode::file(
            &file_id,
            full_path.file_name().unwrap().to_string_lossy(),
            file_path,
        ));

        // 根据语言分发
        match ext_with_dot.as_str() {
            ".py" => python::analyze_python_module(&mut graph, &file_id, &content, &lines, file_path),
            ".js" | ".jsx" | ".ts" | ".tsx" | ".vue" => {
                javascript::analyze_js_module(&mut graph, &file_id, &content, &lines, file_path)
            }
            ".java" => java::analyze_java_module(&mut graph, &file_id, &content, &lines, file_path),
            ".go" => go::analyze_go_module(&mut graph, &file_id, &content, &lines, file_path),
            _ => generic::analyze_generic_module(&mut graph, &file_id, &content, &lines, file_path),
        }

        graph
    }

    /// 收集所有源文件
    fn collect_source_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.project_path)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !IGNORED_DIRS.contains(&name.as_ref())
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                    let ext_with_dot = format!(".{}", ext);
                    if SUPPORTED_EXTENSIONS.contains(&ext_with_dot.as_str()) {
                        files.push(entry.into_path());
                    }
                }
            }
        }

        files.sort();
        files
    }

    /// 添加目录分组信息
    fn add_directory_groups(&self, graph: &mut GraphData, files: &[PathBuf]) {
        let mut dirs: HashSet<String> = HashSet::new();

        for f in files {
            let rel = self.relative_path(f);
            if let Some(dir) = Path::new(&rel).parent() {
                let dir_str = dir.to_string_lossy().to_string();
                if !dir_str.is_empty() && dir_str != "." {
                    dirs.insert(dir_str);
                }
            }
        }

        for d in dirs {
            let dir_name = Path::new(&d)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&d);
            graph.nodes.push(GraphNode::directory(
                format!("dir::{}", d),
                dir_name,
                &d,
            ));
        }
    }

    /// 获取相对路径
    fn relative_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.project_path)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }

    /// 路径转节点 ID
    fn path_to_id(&self, path: &str) -> String {
        format!("file::{}", path.replace('\\', "/").replace(' ', "_"))
    }

    /// 扩展名转语言名
    fn ext_to_language(ext: &str) -> &'static str {
        match ext {
            ".py" => "Python",
            ".js" => "JavaScript",
            ".jsx" => "React",
            ".ts" => "TypeScript",
            ".tsx" => "React TypeScript",
            ".java" => "Java",
            ".go" => "Go",
            ".rs" => "Rust",
            ".c" => "C",
            ".cpp" => "C++",
            ".h" => "C Header",
            ".hpp" => "C++ Header",
            ".cs" => "C#",
            ".rb" => "Ruby",
            ".vue" => "Vue",
            _ => "Unknown",
        }
    }
}
