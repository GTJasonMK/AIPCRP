//! Python 语言分析

use regex::Regex;
use once_cell::sync::Lazy;

use super::types::{GraphData, GraphEdge, GraphNode};

// 预编译正则表达式
static RE_CLASS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^class\s+(\w+)\s*(?:\(([^)]*)\))?:").unwrap()
});
static RE_FUNC: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\s*)def\s+(\w+)\s*\(").unwrap()
});

/// 分析 Python 模块
pub fn analyze_python_module(
    graph: &mut GraphData,
    file_id: &str,
    _content: &str,
    lines: &[&str],
    file_path: &str,
) {
    let mut current_class: Option<String> = None;
    let mut current_class_id: Option<String> = None;

    for (i, line) in lines.iter().enumerate() {
        let stripped = line.trim();

        // 类定义
        if let Some(caps) = RE_CLASS.captures(stripped) {
            let class_name = caps.get(1).unwrap().as_str();
            let bases = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let class_id = format!("{}::class::{}", file_id, class_name);

            current_class = Some(class_name.to_string());
            current_class_id = Some(class_id.clone());

            let node = GraphNode {
                id: class_id.clone(),
                label: class_name.to_string(),
                node_type: "class".to_string(),
                file_path: Some(file_path.to_string()),
                line_number: Some(i + 1),
                metadata: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("bases".to_string(), bases.to_string());
                    m
                },
            };
            graph.nodes.push(node);
            graph.edges.push(GraphEdge::contains(file_id, &class_id));

            // 继承关系
            if !bases.is_empty() {
                for base in bases.split(',') {
                    let base = base.trim();
                    if !base.is_empty() && base != "object" {
                        let base_id = format!("{}::class::{}", file_id, base);
                        graph.edges.push(GraphEdge::inherits(&class_id, &base_id));
                    }
                }
            }
            continue;
        }

        // 函数/方法定义
        if let Some(caps) = RE_FUNC.captures(stripped) {
            let _indent_str = caps.get(1).unwrap().as_str();
            let func_name = caps.get(2).unwrap().as_str();

            // 计算原始行的缩进
            let indent = line.len() - line.trim_start().len();

            if indent > 0 {
                if let Some(ref cls_id) = current_class_id {
                    // 方法
                    let func_id = format!("{}::method::{}", cls_id, func_name);
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("class".to_string(), current_class.clone().unwrap_or_default());
                    graph.nodes.push(GraphNode {
                        id: func_id.clone(),
                        label: func_name.to_string(),
                        node_type: "method".to_string(),
                        file_path: Some(file_path.to_string()),
                        line_number: Some(i + 1),
                        metadata,
                    });
                    graph.edges.push(GraphEdge::new(cls_id, &func_id, "contains", "has method"));
                }
            } else {
                // 顶层函数
                current_class = None;
                current_class_id = None;
                let func_id = format!("{}::func::{}", file_id, func_name);
                graph.nodes.push(GraphNode {
                    id: func_id.clone(),
                    label: func_name.to_string(),
                    node_type: "function".to_string(),
                    file_path: Some(file_path.to_string()),
                    line_number: Some(i + 1),
                    metadata: std::collections::HashMap::new(),
                });
                graph.edges.push(GraphEdge::contains(file_id, &func_id));
            }
            continue;
        }

        // 重置类上下文：遇到顶层非缩进的非注释代码
        if !stripped.is_empty() && !line.starts_with(char::is_whitespace) && !stripped.starts_with('#') {
            if !stripped.starts_with("class ") && !stripped.starts_with("def ") {
                current_class = None;
                current_class_id = None;
            }
        }
    }
}
