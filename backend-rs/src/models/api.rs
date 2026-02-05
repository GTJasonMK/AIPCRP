//! REST API 请求/响应模型

use serde::{Deserialize, Serialize};

/// 建议问题请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestQuestionsRequest {
    pub project_path: Option<String>,
    pub current_file: Option<String>,
    pub file_tree_summary: Option<String>,
}

/// 建议问题响应
#[derive(Debug, Serialize)]
pub struct SuggestQuestionsResponse {
    pub questions: Vec<String>,
}
