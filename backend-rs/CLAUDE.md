# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 启动命令

```bash
# 开发模式
cargo run

# 发布构建（单一 exe）
cargo build --release
```

服务器监听：`127.0.0.1:8765`

## 项目结构

```
backend-rs/
├── src/
│   ├── main.rs                 # 入口：tokio + axum 服务器
│   ├── error.rs                # 统一错误类型 AppError
│   ├── state.rs                # AppState 共享状态
│   ├── config/
│   │   └── app_config.rs       # 配置管理（Lazy + RwLock 单例）
│   ├── api/
│   │   ├── health.rs           # GET /api/health
│   │   ├── config.rs           # GET/PUT /api/config, POST /api/config/test
│   │   ├── chat.rs             # POST /api/chat/suggest, WS /ws/chat
│   │   └── graph.rs            # POST /api/graph/project, POST /api/graph/module
│   ├── llm/
│   │   ├── types.rs            # ChatMessage, ChatChunk, LlmError
│   │   ├── format.rs           # API 格式检测 + URL 构建
│   │   ├── openai.rs           # OpenAI 流式实现
│   │   ├── anthropic.rs        # Anthropic SSE 实现
│   │   └── client.rs           # LlmClient 统一入口
│   ├── services/
│   │   ├── llm_service.rs      # LLM 服务封装
│   │   ├── prompt_service.rs   # Prompt 构建 + 建议问题
│   │   └── code_analyzer/
│   │       ├── mod.rs          # CodeAnalyzer 主结构体
│   │       ├── types.rs        # GraphNode, GraphEdge, GraphData
│   │       ├── python.rs       # Python 语言分析
│   │       ├── javascript.rs   # JS/TS 语言分析
│   │       ├── java.rs         # Java 语言分析
│   │       ├── go.rs           # Go 语言分析
│   │       ├── generic.rs      # 通用分析（回退）
│   │       └── imports.rs      # 导入提取与解析
│   ├── models/
│   │   ├── ws.rs               # WebSocket 消息类型
│   │   └── api.rs              # REST API 模型
│   └── utils/
│       └── request_logger.rs   # JSONL 请求日志（未集成）
├── config.json                 # 运行时配置
└── storage/                    # 日志目录
```

## 核心架构

### 双 API 格式支持

`llm/client.rs` 根据模型名自动选择 API 格式：
- 模型名包含 "claude" → Anthropic Messages API
- 其他模型 → OpenAI Chat Completions API

### WebSocket 聊天协议

与 Python 版完全兼容：
- 入站：`ping`, `chat_message`（含 conversationId, content, context）
- 出站：`pong`, `chat_chunk`, `chat_done`, `chat_error`

### 配置管理

- 文件：`config.json`（与 Python 版格式相同）
- 全局单例：`once_cell::Lazy<RwLock<AppConfig>>`
- 函数：`get_config()`, `update_config()`

### 代码分析器

`services/code_analyzer/` 模块提供知识图谱生成：

- **项目级图谱** (`analyze_project`)：文件节点 + 导入依赖边
- **模块级图谱** (`analyze_module`)：类、函数、方法节点 + 包含/继承关系

支持的语言：Python, JavaScript/TypeScript, Java, Go, Rust

## 已实现功能

- [x] Phase 1：项目骨架 + 配置 + 健康检查
- [x] Phase 2：LLM 客户端（OpenAI + Anthropic 流式）
- [x] Phase 3：WebSocket 聊天 + Prompt 服务
- [x] Phase 4：代码分析器 + 知识图谱
- [x] Phase 5：集成测试 + 部署

## 前端集成

Electron 前端已更新，开发模式下会优先使用 Rust 后端：
- 优先检查 `backend-rs/target/release/backend-rs.exe`
- 其次检查 `backend-rs/target/debug/backend-rs.exe`
- 若均不存在则回退到 Python 后端

Release 构建：7.4MB 单一可执行文件

## API 端点

| 方法 | 路径 | 状态 |
|------|------|------|
| GET | `/api/health` | OK |
| GET | `/api/config` | OK |
| PUT | `/api/config` | OK |
| POST | `/api/config/test` | OK |
| POST | `/api/chat/suggest` | OK |
| WS | `/ws/chat` | OK |
| POST | `/api/graph/project` | OK |
| POST | `/api/graph/module` | OK |

## 依赖版本

- axum 0.7（Web 框架）
- tokio 1（异步运行时）
- reqwest 0.12（HTTP 客户端，支持 HTTP/2）
- serde 1 + serde_json 1（JSON）
- parking_lot 0.12 + once_cell 1（并发）
