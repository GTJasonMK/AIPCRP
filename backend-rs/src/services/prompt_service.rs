//! Prompt 构建服务
//!
//! 负责构建 LLM 聊天消息和生成建议问题

use crate::llm::ChatMessage;

/// 系统提示词
const SYSTEM_PROMPT: &str = r#"You are an expert code reviewer and programming assistant. Your role is to:
1. Analyze code structure and identify potential issues
2. Explain code functionality clearly and concisely
3. Suggest improvements and best practices
4. Answer questions about programming concepts and techniques

Always provide accurate, helpful responses. When reviewing code, consider:
- Code quality and readability
- Performance implications
- Security concerns
- Best practices and design patterns

Respond in the same language as the user's question."#;

/// 最大文件内容长度
const MAX_CONTENT_LENGTH: usize = 8000;

/// Prompt 服务
pub struct PromptService;

impl PromptService {
    /// 创建新的 Prompt 服务
    pub fn new() -> Self {
        Self
    }

    /// 构建聊天消息列表
    pub fn build_chat_messages(
        &self,
        user_query: &str,
        project_path: Option<&str>,
        current_file: Option<&str>,
        current_file_content: Option<&str>,
        selected_code: Option<&str>,
        file_tree_summary: Option<&str>,
    ) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        // 系统消息
        messages.push(ChatMessage::system(SYSTEM_PROMPT));

        // 构建上下文消息
        let mut context_parts = Vec::new();

        if let Some(path) = project_path {
            if !path.is_empty() {
                context_parts.push(format!("Project path: {}", path));
            }
        }

        if let Some(tree) = file_tree_summary {
            if !tree.is_empty() {
                context_parts.push(format!("Project structure:\n```\n{}\n```", tree));
            }
        }

        if let Some(file) = current_file {
            if !file.is_empty() {
                context_parts.push(format!("Current file: {}", file));
            }
        }

        if let Some(content) = current_file_content {
            if !content.is_empty() {
                let truncated = Self::truncate_content(content, MAX_CONTENT_LENGTH);
                context_parts.push(format!("Current file content:\n```\n{}\n```", truncated));
            }
        }

        if let Some(code) = selected_code {
            if !code.is_empty() {
                context_parts.push(format!("Selected code:\n```\n{}\n```", code));
            }
        }

        // 如果有上下文，添加上下文消息
        if !context_parts.is_empty() {
            let context_message = format!("Current context:\n\n{}", context_parts.join("\n\n"));
            messages.push(ChatMessage::system(context_message));
        }

        // 用户消息
        messages.push(ChatMessage::user(user_query));

        messages
    }

    /// 生成建议问题
    pub fn generate_suggested_questions(
        &self,
        _project_path: Option<&str>,
        current_file: Option<&str>,
        _file_tree_summary: Option<&str>,
    ) -> Vec<String> {
        let mut questions = vec![
            "What is the overall architecture of this project?".to_string(),
            "What are the main technologies and frameworks used?".to_string(),
            "What improvements can be made?".to_string(),
        ];

        // 如果有当前文件，添加文件相关问题
        if let Some(file) = current_file {
            if !file.is_empty() {
                let file_name = Self::extract_file_name(file);
                questions.push(format!("Please explain the purpose of {}", file_name));
                questions.push(format!("What are potential issues in {}?", file_name));
            }
        }

        // 最多返回 5 个问题
        questions.truncate(5);
        questions
    }

    /// 截断内容
    fn truncate_content(content: &str, max_len: usize) -> String {
        if content.len() <= max_len {
            content.to_string()
        } else {
            format!("{}... (content truncated)", &content[..max_len])
        }
    }

    /// 提取文件名
    fn extract_file_name(path: &str) -> &str {
        path.rsplit(['/', '\\']).next().unwrap_or(path)
    }
}

impl Default for PromptService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_file_name() {
        assert_eq!(PromptService::extract_file_name("src/main.rs"), "main.rs");
        assert_eq!(PromptService::extract_file_name("src\\main.rs"), "main.rs");
        assert_eq!(PromptService::extract_file_name("main.rs"), "main.rs");
    }

    #[test]
    fn test_generate_suggested_questions() {
        let service = PromptService::new();

        let questions = service.generate_suggested_questions(None, None, None);
        assert_eq!(questions.len(), 3);

        let questions = service.generate_suggested_questions(None, Some("main.rs"), None);
        assert_eq!(questions.len(), 5);
    }
}
