//! 通用语言分析（回退方案）

use regex::Regex;
use once_cell::sync::Lazy;

use super::types::{GraphData, GraphEdge, GraphNode};

static RE_FUNC: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:(?:pub|public|private|protected|static|async|export|def|fn|func)\s+)*(?:function\s+)?(\w+)\s*\(").unwrap()
});

const GENERIC_KEYWORDS: &[&str] = &["if", "for", "while", "switch", "catch", "return", "new", "else"];

/// 分析未知语言的模块（回退）
pub fn analyze_generic_module(
    graph: &mut GraphData,
    file_id: &str,
    _content: &str,
    lines: &[&str],
    file_path: &str,
) {
    for (i, line) in lines.iter().enumerate() {
        let stripped = line.trim();

        if let Some(caps) = RE_FUNC.captures(stripped) {
            let name = caps.get(1).unwrap().as_str();
            if !GENERIC_KEYWORDS.contains(&name) {
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
}
