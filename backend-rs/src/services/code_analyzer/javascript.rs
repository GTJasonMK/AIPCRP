//! JavaScript / TypeScript 语言分析

use regex::Regex;
use once_cell::sync::Lazy;

use super::types::{GraphData, GraphEdge, GraphNode};

static RE_CLASS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:export\s+)?(?:default\s+)?class\s+(\w+)(?:\s+extends\s+(\w+))?").unwrap()
});
static RE_FUNC1: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:export\s+)?(?:default\s+)?(?:async\s+)?function\s+(\w+)").unwrap()
});
static RE_FUNC2: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\(?").unwrap()
});
static RE_FUNC3: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\(.*\)\s*=>").unwrap()
});
static RE_TYPE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:export\s+)?(?:interface|type)\s+(\w+)").unwrap()
});

/// 分析 JS/TS 模块
pub fn analyze_js_module(
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

            let mut metadata = std::collections::HashMap::new();
            if let Some(base) = base_class {
                metadata.insert("extends".to_string(), base.to_string());
            }

            graph.nodes.push(GraphNode {
                id: class_id.clone(),
                label: class_name.to_string(),
                node_type: "class".to_string(),
                file_path: Some(file_path.to_string()),
                line_number: Some(i + 1),
                metadata,
            });
            graph.edges.push(GraphEdge::contains(file_id, &class_id));

            if let Some(base) = base_class {
                let base_id = format!("{}::class::{}", file_id, base);
                graph.edges.push(GraphEdge::inherits(&class_id, &base_id));
            }
            continue;
        }

        // 函数定义（三种模式）
        let func_name = RE_FUNC1.captures(stripped)
            .or_else(|| RE_FUNC2.captures(stripped))
            .or_else(|| RE_FUNC3.captures(stripped))
            .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()));

        if let Some(name) = func_name {
            let func_id = format!("{}::func::{}", file_id, name);
            graph.nodes.push(GraphNode {
                id: func_id.clone(),
                label: name,
                node_type: "function".to_string(),
                file_path: Some(file_path.to_string()),
                line_number: Some(i + 1),
                metadata: std::collections::HashMap::new(),
            });
            graph.edges.push(GraphEdge::contains(file_id, &func_id));
            continue;
        }

        // 接口/类型定义（TypeScript）
        if let Some(caps) = RE_TYPE.captures(stripped) {
            let type_name = caps.get(1).unwrap().as_str();
            let type_id = format!("{}::type::{}", file_id, type_name);
            graph.nodes.push(GraphNode {
                id: type_id.clone(),
                label: type_name.to_string(),
                node_type: "interface".to_string(),
                file_path: Some(file_path.to_string()),
                line_number: Some(i + 1),
                metadata: std::collections::HashMap::new(),
            });
            graph.edges.push(GraphEdge::contains(file_id, &type_id));
        }
    }
}
