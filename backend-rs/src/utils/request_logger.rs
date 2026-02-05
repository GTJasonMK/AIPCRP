//! LLM 请求日志记录器
//!
//! 记录所有 LLM API 请求到 JSONL 文件，便于调试和分析。

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use uuid::Uuid;

/// 请求日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// 请求 ID
    pub request_id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// API 格式
    pub api_format: String,
    /// 端点 URL
    pub endpoint: String,
    /// 基础 URL
    pub base_url: String,
    /// API 密钥（脱敏）
    pub api_key_masked: String,
    /// 模型名称
    pub model: String,
    /// 消息数量
    pub messages_count: usize,
    /// 消息预览
    pub messages_preview: Vec<MessagePreview>,
    /// 温度参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// 最大 token 数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// 超时时间
    pub timeout: u64,
    /// 状态
    pub status: String,
    /// 持续时间（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// 响应长度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_length: Option<usize>,
    /// chunk 数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_count: Option<usize>,
    /// 响应预览
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_preview: Option<String>,
    /// 错误类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// HTTP 状态码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
}

/// 消息预览
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePreview {
    pub role: String,
    pub content_preview: String,
}

/// 请求日志记录器
pub struct RequestLogger {
    log_path: PathBuf,
    max_entries: usize,
    file: Mutex<Option<File>>,
}

impl RequestLogger {
    /// 创建新的日志记录器
    pub fn new(log_dir: Option<PathBuf>) -> Self {
        let log_dir = log_dir.unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."))
                .join("storage")
        });

        // 确保目录存在
        let _ = fs::create_dir_all(&log_dir);

        let log_path = log_dir.join("llm_requests.jsonl");

        Self {
            log_path,
            max_entries: 1000,
            file: Mutex::new(None),
        }
    }

    /// 生成请求 ID
    pub fn generate_request_id() -> String {
        Uuid::new_v4().to_string()[..8].to_string()
    }

    /// API 密钥脱敏
    pub fn mask_api_key(api_key: &str) -> String {
        if api_key.len() <= 8 {
            "*".repeat(api_key.len())
        } else {
            format!(
                "{}...{}",
                &api_key[..4],
                &api_key[api_key.len() - 4..]
            )
        }
    }

    /// 截断字符串
    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len])
        }
    }

    /// 创建消息预览
    pub fn create_message_previews(
        messages: &[(String, String)],
        max_messages: usize,
        max_content_len: usize,
    ) -> Vec<MessagePreview> {
        messages
            .iter()
            .take(max_messages)
            .map(|(role, content)| MessagePreview {
                role: role.clone(),
                content_preview: Self::truncate(content, max_content_len),
            })
            .collect()
    }

    /// 记录请求开始
    pub fn log_request(
        &self,
        request_id: &str,
        api_format: &str,
        endpoint: &str,
        model: &str,
        messages: &[(String, String)],
        temperature: Option<f64>,
        max_tokens: Option<u32>,
        timeout: u64,
        base_url: &str,
        api_key: &str,
    ) -> LogEntry {
        LogEntry {
            request_id: request_id.to_string(),
            timestamp: Utc::now(),
            api_format: api_format.to_string(),
            endpoint: endpoint.to_string(),
            base_url: base_url.to_string(),
            api_key_masked: Self::mask_api_key(api_key),
            model: model.to_string(),
            messages_count: messages.len(),
            messages_preview: Self::create_message_previews(messages, 3, 200),
            temperature,
            max_tokens,
            timeout,
            status: "pending".to_string(),
            duration_ms: None,
            response_length: None,
            chunk_count: None,
            response_preview: None,
            error_type: None,
            error_message: None,
            status_code: None,
        }
    }

    /// 记录成功
    pub fn log_success(
        &self,
        mut entry: LogEntry,
        start_time: std::time::Instant,
        response_length: usize,
        chunk_count: usize,
        response_preview: &str,
    ) {
        entry.status = "success".to_string();
        entry.duration_ms = Some(start_time.elapsed().as_millis() as u64);
        entry.response_length = Some(response_length);
        entry.chunk_count = Some(chunk_count);
        entry.response_preview = Some(Self::truncate(response_preview, 300));
        self.write_entry(&entry);
    }

    /// 记录错误
    pub fn log_error(
        &self,
        mut entry: LogEntry,
        start_time: std::time::Instant,
        error_type: &str,
        error_message: &str,
        status_code: Option<u16>,
    ) {
        entry.status = "error".to_string();
        entry.duration_ms = Some(start_time.elapsed().as_millis() as u64);
        entry.error_type = Some(error_type.to_string());
        entry.error_message = Some(Self::truncate(error_message, 500));
        entry.status_code = status_code;
        self.write_entry(&entry);
    }

    /// 写入日志条目
    fn write_entry(&self, entry: &LogEntry) {
        let mut file_guard = self.file.lock();

        // 懒加载文件
        if file_guard.is_none() {
            if let Ok(f) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log_path)
            {
                *file_guard = Some(f);
            }
        }

        if let Some(file) = file_guard.as_mut() {
            if let Ok(json) = serde_json::to_string(entry) {
                let _ = writeln!(file, "{}", json);
                let _ = file.flush();
            }
        }

        drop(file_guard);
        self.cleanup_if_needed();
    }

    /// 清理旧日志
    fn cleanup_if_needed(&self) {
        if let Ok(file) = File::open(&self.log_path) {
            let reader = BufReader::new(file);
            let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

            if lines.len() > self.max_entries {
                let keep_lines = &lines[lines.len() - self.max_entries..];
                if let Ok(mut file) = File::create(&self.log_path) {
                    for line in keep_lines {
                        let _ = writeln!(file, "{}", line);
                    }
                }
            }
        }
    }
}

impl Default for RequestLogger {
    fn default() -> Self {
        Self::new(None)
    }
}
