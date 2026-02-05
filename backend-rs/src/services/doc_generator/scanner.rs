//! 目录扫描器
//!
//! 扫描源码目录，构建文件树结构

use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use super::types::{DocGenConfig, FileNode};

/// 目录扫描器
pub struct DirectoryScanner {
    config: DocGenConfig,
    /// 编译后的忽略模式（glob patterns）
    ignore_patterns: Vec<glob::Pattern>,
}

impl DirectoryScanner {
    /// 创建新的目录扫描器
    pub fn new(config: DocGenConfig) -> Self {
        let ignore_patterns = config
            .ignore_patterns
            .iter()
            .filter_map(|p| {
                match glob::Pattern::new(p) {
                    Ok(pattern) => Some(pattern),
                    Err(e) => {
                        warn!("Invalid ignore pattern '{}': {}", p, e);
                        None
                    }
                }
            })
            .collect();

        Self {
            config,
            ignore_patterns,
        }
    }

    /// 扫描目录，构建文件树
    pub fn scan(&self, root_path: &Path) -> Result<FileNode, ScanError> {
        if !root_path.exists() {
            return Err(ScanError::PathNotFound(root_path.to_path_buf()));
        }

        if !root_path.is_dir() {
            return Err(ScanError::NotADirectory(root_path.to_path_buf()));
        }

        info!("Starting directory scan: {}", root_path.display());
        let root = self.scan_dir(root_path, root_path, 0)?;
        info!(
            "Scan completed: {} files, {} directories",
            root.file_count(),
            root.get_all_dirs().len()
        );

        Ok(root)
    }

    /// 递归扫描目录
    fn scan_dir(
        &self,
        path: &Path,
        root_path: &Path,
        depth: u32,
    ) -> Result<FileNode, ScanError> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let relative_path = path
            .strip_prefix(root_path)
            .map(|p| p.to_string_lossy().to_string().replace('\\', "/"))
            .unwrap_or_default();

        let mut node = FileNode::new_dir(name, path.to_path_buf(), relative_path, depth);

        // 读取目录内容
        let entries = fs::read_dir(path).map_err(|e| ScanError::IoError(path.to_path_buf(), e))?;

        let mut children = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| ScanError::IoError(path.to_path_buf(), e))?;
            let entry_path = entry.path();
            let entry_name = entry.file_name().to_string_lossy().to_string();

            // 检查是否应该忽略
            if self.should_ignore(&entry_path, &entry_name) {
                debug!("Ignoring: {}", entry_path.display());
                continue;
            }

            if entry_path.is_dir() {
                // 递归扫描子目录
                match self.scan_dir(&entry_path, root_path, depth + 1) {
                    Ok(child) => {
                        // 只添加非空目录或包含支持文件的目录
                        if !child.children.is_empty() {
                            children.push(child);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to scan subdirectory {}: {}", entry_path.display(), e);
                    }
                }
            } else if entry_path.is_file() {
                // 检查是否是支持的文件类型
                if self.is_supported_file(&entry_path) {
                    let child_relative = entry_path
                        .strip_prefix(root_path)
                        .map(|p| p.to_string_lossy().to_string().replace('\\', "/"))
                        .unwrap_or_default();

                    let mut file_node = FileNode::new_file(
                        entry_name,
                        entry_path.clone(),
                        child_relative,
                        depth + 1,
                    );

                    // 获取文件大小
                    if let Ok(metadata) = fs::metadata(&entry_path) {
                        file_node.size = Some(metadata.len());

                        // 跳过过大的文件
                        if metadata.len() > self.config.max_file_size {
                            debug!(
                                "Skipping oversized file: {} ({} bytes)",
                                entry_path.display(),
                                metadata.len()
                            );
                            continue;
                        }
                    }

                    children.push(file_node);
                }
            }
        }

        // 排序：目录在前，文件在后，按名称排序
        children.sort_by(|a, b| {
            match (a.is_file, b.is_file) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        node.children = children;
        Ok(node)
    }

    /// 检查是否应该忽略该路径
    fn should_ignore(&self, path: &Path, name: &str) -> bool {
        // 忽略隐藏文件/目录（以 . 开头）
        if name.starts_with('.') {
            return true;
        }

        // 检查是否匹配忽略模式
        for pattern in &self.ignore_patterns {
            // 检查名称匹配
            if pattern.matches(name) {
                return true;
            }

            // 检查路径匹配
            if let Some(path_str) = path.to_str() {
                if pattern.matches(path_str) {
                    return true;
                }
            }
        }

        // 检查是否是文档目录（避免扫描已生成的文档）
        if name.ends_with(&self.config.docs_suffix) {
            return true;
        }

        false
    }

    /// 检查是否是支持的文件类型
    fn is_supported_file(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            self.config.supported_extensions.contains(&ext_str)
        } else {
            false
        }
    }
}

/// 扫描错误类型
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("路径不存在: {0}")]
    PathNotFound(PathBuf),

    #[error("路径不是目录: {0}")]
    NotADirectory(PathBuf),

    #[error("IO错误 ({0}): {1}")]
    IoError(PathBuf, #[source] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();

        // 创建测试文件结构
        let src_dir = dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let mut main_file = File::create(src_dir.join("main.py")).unwrap();
        main_file.write_all(b"print('hello')").unwrap();

        let utils_dir = src_dir.join("utils");
        fs::create_dir(&utils_dir).unwrap();

        let mut helper_file = File::create(utils_dir.join("helper.py")).unwrap();
        helper_file.write_all(b"def helper(): pass").unwrap();

        // 创建应该被忽略的目录
        fs::create_dir(dir.path().join("node_modules")).unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();

        dir
    }

    #[test]
    fn test_scan_directory() {
        let test_dir = create_test_dir();
        let scanner = DirectoryScanner::new(DocGenConfig::default());

        let root = scanner.scan(test_dir.path()).unwrap();

        // 验证根节点
        assert!(!root.is_file);
        assert_eq!(root.depth, 0);

        // 验证文件数量（应该只有 main.py 和 helper.py）
        assert_eq!(root.file_count(), 2);

        // 验证忽略了 node_modules 和 .git
        let all_names: Vec<_> = root.children.iter().map(|c| c.name.as_str()).collect();
        assert!(!all_names.contains(&"node_modules"));
        assert!(!all_names.contains(&".git"));
    }

    #[test]
    fn test_should_ignore() {
        let scanner = DirectoryScanner::new(DocGenConfig::default());

        // 测试忽略隐藏文件
        assert!(scanner.should_ignore(Path::new(".gitignore"), ".gitignore"));

        // 测试忽略 node_modules
        assert!(scanner.should_ignore(Path::new("node_modules"), "node_modules"));

        // 测试不忽略正常文件
        assert!(!scanner.should_ignore(Path::new("main.py"), "main.py"));
    }

    #[test]
    fn test_is_supported_file() {
        let scanner = DirectoryScanner::new(DocGenConfig::default());

        assert!(scanner.is_supported_file(Path::new("main.py")));
        assert!(scanner.is_supported_file(Path::new("app.ts")));
        assert!(scanner.is_supported_file(Path::new("lib.rs")));
        assert!(!scanner.is_supported_file(Path::new("data.json")));
        assert!(!scanner.is_supported_file(Path::new("README.md")));
    }
}
