//! Go 语言分析

use regex::Regex;
use once_cell::sync::Lazy;

use super::types::{GraphData, GraphEdge, GraphNode};

static RE_STRUCT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^type\s+(\w+)\s+struct").unwrap()
});
static RE_INTERFACE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^type\s+(\w+)\s+interface").unwrap()
});
static RE_FUNC: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^func\s+(?:\(\w+\s+\*?\w+\)\s+)?(\w+)\s*\(").unwrap()
});

/// 分析 Go 模块
pub fn analyze_go_module(
    graph: &mut GraphData,
    file_id: &str,
    _content: &str,
    lines: &[&str],
    file_path: &str,
) {
    for (i, line) in lines.iter().enumerate() {
        let stripped = line.trim();

        // struct 定义
        if let Some(caps) = RE_STRUCT.captures(stripped) {
            let name = caps.get(1).unwrap().as_str();
            let node_id = format!("{}::struct::{}", file_id, name);
            graph.nodes.push(GraphNode {
                id: node_id.clone(),
                label: name.to_string(),
                node_type: "class".to_string(), // 用 class 类型以便前端统一处理
                file_path: Some(file_path.to_string()),
                line_number: Some(i + 1),
                metadata: std::collections::HashMap::new(),
            });
            graph.edges.push(GraphEdge::contains(file_id, &node_id));
            continue;
        }

        // interface 定义
        if let Some(caps) = RE_INTERFACE.captures(stripped) {
            let name = caps.get(1).unwrap().as_str();
            let node_id = format!("{}::interface::{}", file_id, name);
            graph.nodes.push(GraphNode {
                id: node_id.clone(),
                label: name.to_string(),
                node_type: "interface".to_string(),
                file_path: Some(file_path.to_string()),
                line_number: Some(i + 1),
                metadata: std::collections::HashMap::new(),
            });
            graph.edges.push(GraphEdge::contains(file_id, &node_id));
            continue;
        }

        // 函数定义
        if let Some(caps) = RE_FUNC.captures(stripped) {
            let name = caps.get(1).unwrap().as_str();
            let node_id = format!("{}::func::{}", file_id, name);
            graph.nodes.push(GraphNode {
                id: node_id.clone(),
                label: name.to_string(),
                node_type: "function".to_string(),
                file_path: Some(file_path.to_string()),
                line_number: Some(i + 1),
                metadata: std::collections::HashMap::new(),
            });
            graph.edges.push(GraphEdge::contains(file_id, &node_id));
        }
    }
}
