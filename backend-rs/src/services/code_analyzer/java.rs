//! Java 语言分析

use regex::Regex;
use once_cell::sync::Lazy;

use super::types::{GraphData, GraphEdge, GraphNode};

static RE_CLASS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:public|private|protected)?\s*(?:static\s+)?(?:abstract\s+)?class\s+(\w+)(?:\s+extends\s+(\w+))?").unwrap()
});
static RE_METHOD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s+(?:public|private|protected)?\s*(?:static\s+)?(?:\w+\s+)(\w+)\s*\(").unwrap()
});

/// 关键字列表，不应当作方法名
const JAVA_KEYWORDS: &[&str] = &["if", "for", "while", "switch", "catch", "return", "new"];

/// 分析 Java 模块
pub fn analyze_java_module(
    graph: &mut GraphData,
    file_id: &str,
    _content: &str,
    lines: &[&str],
    file_path: &str,
) {
    for (i, line) in lines.iter().enumerate() {
        let stripped = line.trim();

        // 类定义
        if let Some(caps) = RE_CLASS.captures(stripped) {
            let class_name = caps.get(1).unwrap().as_str();
            let base_class = caps.get(2).map(|m| m.as_str());
            let class_id = format!("{}::class::{}", file_id, class_name);

            graph.nodes.push(GraphNode {
                id: class_id.clone(),
                label: class_name.to_string(),
                node_type: "class".to_string(),
                file_path: Some(file_path.to_string()),
                line_number: Some(i + 1),
                metadata: std::collections::HashMap::new(),
            });
            graph.edges.push(GraphEdge::contains(file_id, &class_id));

            if let Some(base) = base_class {
                let base_id = format!("{}::class::{}", file_id, base);
                graph.edges.push(GraphEdge::inherits(&class_id, &base_id));
            }
            continue;
        }

        // 方法定义
        if let Some(caps) = RE_METHOD.captures(line) {
            let method_name = caps.get(1).unwrap().as_str();
            if !JAVA_KEYWORDS.contains(&method_name) {
                let func_id = format!("{}::func::{}", file_id, method_name);
                graph.nodes.push(GraphNode {
                    id: func_id.clone(),
                    label: method_name.to_string(),
                    node_type: "method".to_string(),
                    file_path: Some(file_path.to_string()),
                    line_number: Some(i + 1),
                    metadata: std::collections::HashMap::new(),
                });
            }
        }
    }
}
