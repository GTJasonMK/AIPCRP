//! 导入语句解析

use regex::Regex;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;

use super::types::ImportInfo;

// Python 导入
static RE_PY_IMPORT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^import\s+([\w.]+)").unwrap()
});
static RE_PY_FROM: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^from\s+([\w.]+)\s+import").unwrap()
});

// JS/TS 导入
static RE_JS_IMPORT1: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?:import|from)\s+['"]([^'"]+)['"]"#).unwrap()
});
static RE_JS_IMPORT2: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"import\s+.*?\s+from\s+['"]([^'"]+)['"]"#).unwrap()
});

// Java 导入
static RE_JAVA_IMPORT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^import\s+([\w.]+);").unwrap()
});

// Go 导入
static RE_GO_IMPORT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""([\w/.-]+)""#).unwrap()
});

/// 提取导入语句
pub fn extract_imports(content: &str, ext: &str, _current_file: &str) -> Vec<ImportInfo> {
    let mut imports = Vec::new();

    match ext {
        ".py" => {
            for line in content.lines() {
                let line = line.trim();
                // import foo.bar
                if let Some(caps) = RE_PY_IMPORT.captures(line) {
                    let imp = caps.get(1).unwrap().as_str();
                    let display = imp.rsplit('.').next().unwrap_or(imp);
                    imports.push(ImportInfo {
                        path: imp.to_string(),
                        display_name: display.to_string(),
                    });
                }
                // from foo.bar import ...
                if let Some(caps) = RE_PY_FROM.captures(line) {
                    let imp = caps.get(1).unwrap().as_str();
                    let display = imp.rsplit('.').next().unwrap_or(imp);
                    imports.push(ImportInfo {
                        path: imp.to_string(),
                        display_name: display.to_string(),
                    });
                }
            }
        }
        ".js" | ".jsx" | ".ts" | ".tsx" | ".vue" => {
            for line in content.lines() {
                // 只处理相对导入
                for re in [&*RE_JS_IMPORT1, &*RE_JS_IMPORT2] {
                    if let Some(caps) = re.captures(line) {
                        let imp = caps.get(1).unwrap().as_str();
                        if imp.starts_with('.') {
                            let display = Path::new(imp)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(imp);
                            imports.push(ImportInfo {
                                path: imp.to_string(),
                                display_name: display.to_string(),
                            });
                        }
                    }
                }
            }
        }
        ".java" => {
            for line in content.lines() {
                let line = line.trim();
                if let Some(caps) = RE_JAVA_IMPORT.captures(line) {
                    let imp = caps.get(1).unwrap().as_str();
                    let display = imp.rsplit('.').next().unwrap_or(imp);
                    imports.push(ImportInfo {
                        path: imp.to_string(),
                        display_name: display.to_string(),
                    });
                }
            }
        }
        ".go" => {
            for caps in RE_GO_IMPORT.captures_iter(content) {
                let imp = caps.get(1).unwrap().as_str();
                let display = imp.rsplit('/').next().unwrap_or(imp);
                imports.push(ImportInfo {
                    path: imp.to_string(),
                    display_name: display.to_string(),
                });
            }
        }
        _ => {}
    }

    imports
}

/// 解析导入路径到项目文件
pub fn resolve_import(
    import_path: &str,
    current_file: &str,
    file_map: &HashMap<String, bool>,
) -> Option<String> {
    let current_dir = Path::new(current_file)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // 相对导入 (./foo, ../foo)
    if import_path.starts_with('.') {
        let candidate = Path::new(&current_dir)
            .join(import_path)
            .to_string_lossy()
            .replace('\\', "/");

        // 尝试各种扩展名
        let extensions = ["", ".ts", ".tsx", ".js", ".jsx", ".py", "/index.ts", "/index.tsx", "/index.js"];
        for ext in extensions {
            let test = format!("{}{}", candidate, ext);
            // 规范化路径
            let normalized = normalize_path(&test);
            if file_map.contains_key(&normalized) {
                return Some(normalized);
            }
        }
        return None;
    }

    // Python 点导入 (app.utils.foo -> app/utils/foo.py)
    if import_path.contains('.') && !import_path.starts_with('.') {
        let candidate = import_path.replace('.', "/");
        for ext in [".py", "/__init__.py"] {
            let test = format!("{}{}", candidate, ext);
            if file_map.contains_key(&test) {
                return Some(test);
            }
        }
    }

    None
}

/// 规范化路径（简化版本）
fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => continue,
            ".." => { parts.pop(); }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}
