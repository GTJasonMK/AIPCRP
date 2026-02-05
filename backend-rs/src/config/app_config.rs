//! 应用配置管理
//!
//! 提供配置的加载、保存、更新功能，使用全局单例模式管理配置状态。

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::error::AppError;

/// 获取配置文件路径
fn get_config_path() -> PathBuf {
    // 配置文件位于可执行文件同级目录
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("config.json")
}

/// 应用配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// LLM API 密钥
    #[serde(default)]
    pub api_key: String,

    /// LLM API 基础 URL
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// 模型名称
    #[serde(default = "default_model")]
    pub model: String,

    /// 温度参数 (0.0 - 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f64,

    /// 最大 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_base_url() -> String {
    "https://api.openai.com".to_string()
}

fn default_model() -> String {
    "gpt-4o".to_string()
}

fn default_temperature() -> f64 {
    0.7
}

fn default_max_tokens() -> u32 {
    4096
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: default_base_url(),
            model: default_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

/// 全局配置单例
static CONFIG: Lazy<RwLock<AppConfig>> = Lazy::new(|| {
    RwLock::new(load_config_from_file().unwrap_or_default())
});

/// 从文件加载配置
fn load_config_from_file() -> Option<AppConfig> {
    let path = get_config_path();
    if path.exists() {
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

/// 保存配置到文件
fn save_config_to_file(config: &AppConfig) -> Result<(), AppError> {
    let path = get_config_path();
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| AppError::Config(format!("序列化配置失败: {}", e)))?;
    fs::write(&path, content)
        .map_err(|e| AppError::Config(format!("写入配置文件失败: {}", e)))?;
    Ok(())
}

/// 获取当前配置（克隆）
pub fn get_config() -> AppConfig {
    CONFIG.read().clone()
}

/// 更新配置
///
/// 接收一个闭包来修改配置，修改后自动保存到文件
pub fn update_config<F>(updater: F) -> Result<AppConfig, AppError>
where
    F: FnOnce(&mut AppConfig),
{
    let mut config = CONFIG.write();
    updater(&mut config);
    save_config_to_file(&config)?;
    Ok(config.clone())
}

/// 替换整个配置
pub fn set_config(new_config: AppConfig) -> Result<(), AppError> {
    save_config_to_file(&new_config)?;
    *CONFIG.write() = new_config;
    Ok(())
}

/// 重新从文件加载配置
pub fn reload_config() {
    if let Some(config) = load_config_from_file() {
        *CONFIG.write() = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.base_url, "https://api.openai.com");
        assert_eq!(config.model, "gpt-4o");
        assert!((config.temperature - 0.7).abs() < f64::EPSILON);
        assert_eq!(config.max_tokens, 4096);
    }
}
