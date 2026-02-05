//! LLM Prompt 模板
//!
//! 定义代码分析、目录总结、README生成等 Prompt 模板

/// 代码文件分析 Prompt
pub const CODE_ANALYSIS_PROMPT: &str = r#"请分析以下代码文件，生成详细的技术文档。

文件路径: {file_path}

代码内容:
```
{code_content}
```

请提供以下内容：
1. 文件概述：简要描述这个文件的主要功能和用途
2. 主要组件：列出文件中的类、函数、常量等主要组件
3. 依赖关系：列出该文件依赖的其他模块
4. 关键逻辑：解释核心算法或业务逻辑
5. 使用示例：如果适用，提供简单的使用示例

6. API接口识别（重要）：
   请**仔细检查**代码，判断此文件是否包含API接口定义（HTTP端点、路由、RPC接口、WebSocket等）。

   **识别API接口的特征**：
   - Flask: @app.route, @blueprint.route, @bp.route
   - FastAPI: @app.get, @app.post, @router.get, @router.post 等
   - Express: app.get, app.post, router.get, router.post 等
   - Django: path(), re_path(), urlpatterns
   - Spring: @GetMapping, @PostMapping, @RequestMapping, @DeleteMapping, @PutMapping
   - Gin (Go): r.GET, r.POST, router.Handle 等
   - Axum (Rust): .route(), Router::new()
   - 其他框架的路由/端点装饰器或注册函数

   **如果包含API接口**，请在文档末尾添加以下格式的标记（确保每个接口都列出，不要遗漏）：

   <!-- API_START -->
   包含API接口: 是
   接口列表:
   - [GET] /api/users - 获取用户列表
   - [POST] /api/users - 创建新用户
   - [DELETE] /api/users/{{id}} - 删除指定用户
   <!-- API_END -->

   **如果不包含API接口**，请添加：
   <!-- API_START -->
   包含API接口: 否
   <!-- API_END -->

   **注意**：
   - 只列出代码中明确定义的接口，不要推测或编造
   - 路径中的动态参数用 {{param}} 格式表示
   - 确保不遗漏任何接口

7. 知识图谱数据提取（重要）：
   请仔细分析代码结构，提取以下信息并以JSON格式输出：

   **需要提取的节点类型**：
   - class: 类定义
   - function: 独立函数
   - method: 类方法
   - interface: 接口定义（TypeScript/Java/Go等）
   - struct: 结构体定义（Rust/Go/C等）
   - enum: 枚举定义
   - constant: 常量定义

   **需要提取的关系类型**：
   - contains: 包含关系（文件包含类，类包含方法）
   - imports: 导入关系
   - calls: 调用关系（函数调用其他函数）
   - inherits: 继承关系
   - implements: 实现关系

   请在文档末尾添加以下格式的知识图谱数据：

   <!-- GRAPH_DATA_START -->
   ```json
   {{
     "nodes": [
       {{"id": "class::{file_path}::ClassName", "label": "ClassName", "type": "class", "line": 10}},
       {{"id": "function::{file_path}::func_name", "label": "func_name", "type": "function", "line": 25}},
       {{"id": "method::{file_path}::ClassName::method_name", "label": "method_name", "type": "method", "line": 15}}
     ],
     "edges": [
       {{"source": "file::{file_path}", "target": "class::{file_path}::ClassName", "type": "contains"}},
       {{"source": "class::{file_path}::ClassName", "target": "method::{file_path}::ClassName::method_name", "type": "contains"}},
       {{"source": "function::{file_path}::func_name", "target": "function::{file_path}::other_func", "type": "calls"}}
     ],
     "imports": [
       {{"module": "os", "items": ["path", "getcwd"]}},
       {{"module": "./utils", "items": ["helper"]}}
     ]
   }}
   ```
   <!-- GRAPH_DATA_END -->

   **图谱提取规则**：
   - id格式: `{{type}}::{{file_path}}::{{name}}` 或 `{{type}}::{{file_path}}::{{class}}::{{method}}`
   - 使用代码中的实际文件路径替换 {{file_path}}
   - line 是代码行号，如果无法确定可以省略
   - 只提取代码中明确存在的元素，不要推测
   - imports 列出所有导入语句

请用中文回答，保持专业和简洁。
"#;

/// 目录总结 Prompt
pub const DIRECTORY_SUMMARY_PROMPT: &str = r#"请根据以下子模块的文档，生成该目录的总结文档。

目录名称: {dir_name}
目录路径: {dir_path}

子模块文档:
{sub_documents}

请提供以下内容：
1. 目录概述：这个目录的整体功能和职责
2. 模块关系：子模块之间的关系和依赖
3. 核心功能：该目录提供的主要功能
4. 设计模式：如果有明显的设计模式，请指出

5. 知识图谱数据提取（重要）：
   请根据子模块文档分析模块间的关系，提取以下信息并以JSON格式输出：

   **需要提取的节点类型**：
   - module: 子模块（文件或子目录）
   - class: 主要的类定义
   - function: 主要的公开函数
   - interface: 接口定义

   **需要提取的关系类型**：
   - contains: 目录包含子模块
   - imports: 模块间导入关系
   - calls: 模块间调用关系
   - depends: 模块间依赖关系

   请在文档末尾添加以下格式的知识图谱数据：

   <!-- GRAPH_DATA_START -->
   ```json
   {{
     "nodes": [
       {{"id": "module::{dir_path}::sub_module_name", "label": "sub_module_name", "type": "module", "line": null}},
       {{"id": "class::{dir_path}::ClassName", "label": "ClassName", "type": "class", "line": null}}
     ],
     "edges": [
       {{"source": "dir::{dir_path}", "target": "module::{dir_path}::sub_module", "type": "contains"}},
       {{"source": "module::{dir_path}::moduleA", "target": "module::{dir_path}::moduleB", "type": "imports"}}
     ],
     "imports": [
       {{"module": "sub_module_name", "items": ["exported_item"]}}
     ]
   }}
   ```
   <!-- GRAPH_DATA_END -->

   **图谱提取规则**：
   - 只提取子模块文档中明确提到的模块和关系
   - id格式: `{{type}}::{{dir_path}}::{{name}}`
   - 重点关注模块间的依赖和调用关系
   - 不要推测或编造不存在的关系

请用中文回答，保持专业和简洁。
"#;

/// README 生成 Prompt
pub const README_PROMPT: &str = r#"请根据以下所有模块的文档，生成项目的README文档。

项目名称: {project_name}
项目路径: {project_path}

所有模块文档:
{all_documents}

请生成一份完整、实用的README文档，让用户能够快速上手使用该项目。

## 必须包含的内容

### 1. 项目简介
- 项目名称和一句话描述
- 项目解决什么问题
- 主要特性列表

### 2. 快速开始

#### 2.1 环境要求
根据代码分析推断需要的环境：
- Python/Node.js/Rust/其他运行时版本
- 操作系统要求（如果有）
- 其他依赖（数据库、Redis等，如果有）

#### 2.2 安装步骤
```bash
# 克隆项目
git clone <repository_url>
cd {project_name}

# 安装依赖（根据项目类型推断）
pip install -r requirements.txt  # Python项目
npm install  # Node.js项目
cargo build  # Rust项目
```

#### 2.3 配置说明
- 列出需要配置的环境变量或配置文件
- 提供配置示例
- 说明每个配置项的作用

#### 2.4 运行项目
```bash
# 提供具体的启动命令
```

### 3. 使用方法

#### 3.1 命令行使用（如果是CLI工具）
```bash
# 基本用法
# 常用示例
```

#### 3.2 作为库使用（如果可以作为模块导入）
```
# 使用示例
```

#### 3.3 API接口（如果是Web服务）
列出主要的API端点和用法

### 4. 项目结构
```
{project_name}/
├── src/           # 源代码目录
│   ├── core/      # 核心模块
│   └── utils/     # 工具函数
├── config.yaml    # 配置文件
└── main.py        # 程序入口
```

### 5. 核心功能说明
简要描述各个核心模块的功能

### 6. 配置参数详解
以表格形式列出所有配置项：

| 参数名 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| xxx    | str  | ""     | 描述 |

### 7. 常见问题（FAQ）
根据项目特点，预测可能的问题并提供解答

## 注意事项
- 所有代码块都要标注语言类型
- 配置示例要完整可用
- 命令要可以直接复制执行
- 如果某些信息无法从代码中推断，用 `<待补充>` 标记

请用中文回答，格式清晰，适合作为项目文档。
"#;

/// 阅读顺序指南 Prompt
pub const READING_GUIDE_PROMPT: &str = r#"请根据以下项目文档，生成一份项目文档阅读顺序指南。

项目名称: {project_name}

项目结构:
{project_structure}

所有模块文档:
{all_documents}

请生成一份详细的阅读顺序指南，帮助新人系统性地理解整个项目。

## 核心要求：生成明确的阅读顺序链条

你必须生成一个**完整的阅读顺序链条**，用箭头连接所有重要文件，形成一条清晰的阅读路径。

### 格式要求

1. **阅读顺序链条**（必须包含）

   用以下格式展示完整的阅读路径：
   ```
   推荐阅读顺序：

   config.py -> main.py -> core/analyzer.py -> services/scanner.py -> ...
   ```

   这个链条必须：
   - 包含项目中所有重要的文件
   - 按照从基础到高级的顺序排列
   - 用 `->` 箭头连接每个文件

2. **每一步的阅读理由**（必须包含）

   对链条中的每一个"箭头"解释原因，说明为什么按这个顺序阅读：

   ```
   第1步：config.py
      为什么先读：了解项目的配置结构，这是理解后续模块的基础

   第2步：main.py（在 config.py 之后）
      为什么这个顺序：config.py 定义了配置，main.py 是程序入口，会加载和使用这些配置

   第3步：core/analyzer.py（在 main.py 之后）
      为什么这个顺序：main.py 调用了 analyzer，理解入口后再看核心逻辑
   ```

3. **阅读链条设计原则**

   按以下优先级设计阅读顺序：
   - 先读被依赖的模块，后读依赖它的模块
   - 先读配置和模型定义，后读业务逻辑
   - 先读入口文件，后读核心实现
   - 先读基础工具，后读高级功能
   - 考虑认知负荷：简单模块在前，复杂模块在后

4. **模块分层概览**

   将文件按层次分类，帮助理解架构：
   - 入口层：程序启动入口
   - 配置层：配置和常量定义
   - 模型层：数据结构定义
   - 服务层：业务逻辑实现
   - 工具层：辅助工具函数

5. **快速阅读路径**（可选）

   如果读者时间有限，提供一个精简版的阅读路径：
   ```
   快速理解路径（5个核心文件）：
   config.py -> main.py -> core/analyzer.py -> 完成！
   ```

请用中文回答，格式清晰，使用Markdown格式。确保阅读链条是连贯的、有逻辑的。
"#;

/// API 接口提取 Prompt（第一阶段）
pub const API_EXTRACT_PROMPT: &str = r#"请从以下代码文件分析文档中**精确提取**所有API接口信息。

文件路径: {file_path}

文件分析文档:
{file_doc}

## 严格要求

1. **只提取明确存在的接口**：只输出文档中明确提到的接口，禁止推测或编造
2. **必须标注认证要求**：每个接口必须明确标注是否需要认证
3. **保持信息原貌**：接口路径、方法必须与文档描述完全一致

## 认证判断规则

根据以下特征判断接口是否需要认证：
- 使用了 `@require_auth`、`@require_admin`、`@login_required` 等装饰器 -> 需要认证
- 使用了 `Depends(require_api_auth)`、`Depends(get_current_user)` 等依赖 -> 需要认证
- 路由定义中明确提到 "无需认证"、"公开接口" -> 无需认证
- 登录接口（如 `/login`）本身 -> 无需认证
- 健康检查、静态资源接口 -> 通常无需认证
- 如果文档未明确说明，标注为"未明确"

## 输出格式（严格按此格式）

如果文件包含API接口，按以下格式输出：

### {file_path} 的接口列表

| 序号 | 方法 | 路径 | 功能描述 | 认证要求 |
|------|------|------|----------|----------|
| 1 | GET/POST/... | /api/xxx | 简要描述 | 需要/无需/未明确 |

如果文件中没有API接口，只输出一行：
**该文件未定义API接口**

## 禁止事项
- 禁止编造文档中未提及的接口
- 禁止生成请求/响应示例
- 禁止遗漏认证要求信息
"#;

/// API 接口汇总 Prompt（第二阶段）
pub const API_SUMMARY_PROMPT: &str = r#"请根据以下各文件提取的API接口信息，生成一份**精确、完整**的接口清单。

项目名称: {project_name}

各文件的接口信息:
{api_details}

## 严格要求

1. **不重不漏**：确保每个接口只出现一次，同时不遗漏任何接口
2. **禁止幻觉**：只输出上述信息中明确存在的接口，禁止编造
3. **保持原貌**：接口路径、方法、认证要求必须与原文完全一致
4. **固定格式**：严格按照下方模板输出，不要改变格式结构

## 输出模板（严格按此格式，不要修改结构）

```markdown
## 一、接口总览

| 序号 | 模块 | 方法 | 路径 | 功能描述 | 认证 |
|------|------|------|------|----------|------|
| 1 | 模块名 | GET | /xxx | 描述 | 是/否 |
| 2 | ... | ... | ... | ... | ... |

## 二、按模块分类

按以下固定顺序组织模块（如果该模块无接口则跳过）：

### 2.1 核心业务接口
（聊天、对话、主要业务功能相关的接口）

| 方法 | 路径 | 功能描述 | 认证 |
|------|------|----------|------|

### 2.2 资源管理接口
（文件、图片、模型等资源相关的接口）

| 方法 | 路径 | 功能描述 | 认证 |
|------|------|----------|------|

### 2.3 用户与认证接口
（登录、注册、Token管理等认证相关的接口）

| 方法 | 路径 | 功能描述 | 认证 |
|------|------|----------|------|

### 2.4 系统管理接口
（账号管理、配置管理、系统维护等管理接口）

| 方法 | 路径 | 功能描述 | 认证 |
|------|------|----------|------|

### 2.5 辅助接口
（健康检查、页面路由、静态资源等辅助接口）

| 方法 | 路径 | 功能描述 | 认证 |
|------|------|----------|------|

## 三、认证要求汇总

### 无需认证的接口
- `GET /xxx` - 描述
- `POST /xxx` - 描述

### 需要认证的接口
- 核心业务接口：全部需要认证
- 资源管理接口：除 xxx 外全部需要认证
- ...（按模块说明）
```

## 禁止事项
- 禁止编造不存在的接口
- 禁止生成请求/响应示例
- 禁止改变上述模板的结构
- 禁止添加模板中没有的章节
"#;

/// 格式化代码分析 Prompt
pub fn format_code_analysis_prompt(file_path: &str, code_content: &str) -> String {
    CODE_ANALYSIS_PROMPT
        .replace("{file_path}", file_path)
        .replace("{code_content}", code_content)
}

/// 格式化目录总结 Prompt
pub fn format_directory_summary_prompt(
    dir_name: &str,
    dir_path: &str,
    sub_documents: &str,
) -> String {
    DIRECTORY_SUMMARY_PROMPT
        .replace("{dir_name}", dir_name)
        .replace("{dir_path}", dir_path)
        .replace("{sub_documents}", sub_documents)
}

/// 格式化 README Prompt
pub fn format_readme_prompt(
    project_name: &str,
    project_path: &str,
    all_documents: &str,
) -> String {
    README_PROMPT
        .replace("{project_name}", project_name)
        .replace("{project_path}", project_path)
        .replace("{all_documents}", all_documents)
}

/// 格式化阅读指南 Prompt
pub fn format_reading_guide_prompt(
    project_name: &str,
    project_structure: &str,
    all_documents: &str,
) -> String {
    READING_GUIDE_PROMPT
        .replace("{project_name}", project_name)
        .replace("{project_structure}", project_structure)
        .replace("{all_documents}", all_documents)
}

/// 格式化 API 提取 Prompt
pub fn format_api_extract_prompt(file_path: &str, file_doc: &str) -> String {
    API_EXTRACT_PROMPT
        .replace("{file_path}", file_path)
        .replace("{file_doc}", file_doc)
}

/// 格式化 API 汇总 Prompt
pub fn format_api_summary_prompt(project_name: &str, api_details: &str) -> String {
    API_SUMMARY_PROMPT
        .replace("{project_name}", project_name)
        .replace("{api_details}", api_details)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_code_analysis_prompt() {
        let result = format_code_analysis_prompt("test.py", "print('hello')");
        assert!(result.contains("test.py"));
        assert!(result.contains("print('hello')"));
    }

    #[test]
    fn test_format_directory_summary_prompt() {
        let result = format_directory_summary_prompt("src", "/project/src", "doc content");
        assert!(result.contains("src"));
        assert!(result.contains("/project/src"));
        assert!(result.contains("doc content"));
    }
}
