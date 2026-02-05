//! API 格式检测和 URL 构建工具

use serde::{Deserialize, Serialize};

/// API 格式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiFormat {
    /// OpenAI Chat Completions API
    OpenAi,
    /// Anthropic Messages API
    Anthropic,
}

/// 根据模型名称检测 API 格式
///
/// 规则：模型名包含 "claude" 则使用 Anthropic 格式，否则使用 OpenAI 格式
pub fn detect_api_format(model: &str) -> ApiFormat {
    if model.to_lowercase().contains("claude") {
        ApiFormat::Anthropic
    } else {
        ApiFormat::OpenAi
    }
}

/// 修复 base_url
///
/// - 移除末尾斜杠
/// - 修复双斜杠（保留协议部分）
pub fn fix_base_url(base_url: &str) -> String {
    let mut url = base_url.trim_end_matches('/').to_string();

    // 修复双斜杠（跳过协议部分）
    if let Some(pos) = url.find("://") {
        let (protocol, rest) = url.split_at(pos + 3);
        let fixed_rest = rest.replace("//", "/");
        url = format!("{}{}", protocol, fixed_rest);
    }

    url
}

/// 构建 OpenAI Chat Completions 端点
pub fn build_openai_endpoint(base_url: &str) -> String {
    let url = fix_base_url(base_url);

    if url.ends_with("/chat/completions") {
        url
    } else if url.ends_with("/v1") {
        format!("{}/chat/completions", url)
    } else {
        format!("{}/v1/chat/completions", url)
    }
}

/// 构建 Anthropic Messages 端点
pub fn build_anthropic_endpoint(base_url: &str) -> String {
    let url = fix_base_url(base_url);

    if url.ends_with("/messages") {
        url
    } else if url.ends_with("/v1") {
        format!("{}/messages", url)
    } else {
        format!("{}/v1/messages", url)
    }
}

/// 获取浏览器模拟请求头
pub fn get_browser_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"),
        ("Accept", "application/json, text/plain, */*"),
        ("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_api_format() {
        assert_eq!(detect_api_format("gpt-4o"), ApiFormat::OpenAi);
        assert_eq!(detect_api_format("deepseek-chat"), ApiFormat::OpenAi);
        assert_eq!(detect_api_format("claude-3-opus"), ApiFormat::Anthropic);
        assert_eq!(detect_api_format("Claude-3-Sonnet"), ApiFormat::Anthropic);
    }

    #[test]
    fn test_fix_base_url() {
        assert_eq!(fix_base_url("https://api.openai.com/"), "https://api.openai.com");
        assert_eq!(fix_base_url("https://api.openai.com//v1"), "https://api.openai.com/v1");
    }

    #[test]
    fn test_build_openai_endpoint() {
        assert_eq!(
            build_openai_endpoint("https://api.openai.com"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            build_openai_endpoint("https://api.openai.com/v1"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            build_openai_endpoint("https://api.openai.com/v1/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_build_anthropic_endpoint() {
        assert_eq!(
            build_anthropic_endpoint("https://api.anthropic.com"),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            build_anthropic_endpoint("https://api.anthropic.com/v1"),
            "https://api.anthropic.com/v1/messages"
        );
    }
}
