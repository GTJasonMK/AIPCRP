# AI Code Review Platform（AIPCRP）

<div align="center">
  <img src="assets/logo/aipcrp-wordmark.svg" alt="AIPCRP" width="760" />
</div>

AI 驱动的代码审查平台（桌面端）。提供代码浏览、目录树导航、知识图谱与 AI 辅助问答，帮助快速理解与审阅项目代码。

## 技术栈

- 前端：Electron + React + TypeScript + Monaco Editor
- 后端：Rust（Axum）
- 通信：REST API + WebSocket
- 状态管理：Zustand
- 样式：Tailwind CSS

## 功能概览

- 代码目录树导航与文件预览（Monaco Editor）
- AI 对话与建议问题
- 项目/模块知识图谱（Graph）
- 文档生成与进度推送（WebSocket）
- 最近打开项目记录

## 快速开始（Windows）

### 1) 环境要求

- Node.js（建议 18+）
- Rust 工具链（stable）

### 2) 安装依赖（首次运行）

```bat
install.bat
```

### 3) 启动开发模式

```bat
start.bat
```

说明：启动脚本会先构建 Rust 后端（release），再启动 Electron（开发模式）。

## 手动启动（开发）

### 后端

```bash
cd backend-rs
cargo build --release
./target/release/backend-rs.exe
```

后端默认监听：`127.0.0.1:8765`

### 前端

```bash
cd frontend
npm install
npm run dev
```

## 打包构建

### Windows 一键打包

```bat
build.bat win
```

输出目录：`frontend/dist/`

### 说明（Mac/Linux）

`frontend/electron-builder.json` 已包含 macOS / Linux 目标配置，但当前主进程默认查找并启动 `backend-rs.exe`。如需跨平台打包，需要为对应平台编译后端产物并调整启动逻辑与资源拷贝策略。

## 配置说明

后端配置文件为 `config.json`，位置为“后端可执行文件同级目录”。应用内设置界面会读写该配置。

配置示例：

```json
{
  "api_key": "YOUR_API_KEY",
  "base_url": "https://api.openai.com",
  "model": "gpt-4o",
  "temperature": 0.7,
  "max_tokens": 4096
}
```

## 数据与产物

- 配置文件：后端可执行文件同级目录 `config.json`
- 生成文档：项目根目录 `.docs/`
- 断点文件：`.docs/.checkpoint.json`

## 项目结构

```text
├── frontend/                 # Electron + React
│   ├── src/main/            # Electron 主进程（自动管理后端进程）
│   ├── src/preload/         # 预加载脚本
│   └── src/renderer/        # React 渲染进程
│       └── src/
│           ├── components/  # UI 组件
│           ├── stores/      # Zustand 状态管理
│           └── services/    # API 服务封装
├── backend-rs/              # Rust 后端（Axum）
│   └── src/
│       ├── main.rs          # 入口
│       ├── api/             # REST API + WebSocket 端点
│       ├── llm/             # LLM 客户端（OpenAI + Anthropic 格式）
│       ├── services/        # 业务逻辑（代码分析、文档生成）
│       └── config/          # 配置管理
├── assets/                   # 仓库资源（Logo 等）
└── build.bat / install.bat / start.bat
```

## 常用命令

前端（`frontend/`）：

```bash
npm run dev
npm run build
npm run build:win
```

后端（`backend-rs/`）：

```bash
cargo test
cargo build --release
```

## License

当前仓库未包含 `LICENSE` 文件。建议在上传 GitHub 前补充许可证声明（如 MIT/Apache-2.0 等）。

---

最后更新：2026-02-05（Codex）
