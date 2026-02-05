# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

AI驱动的代码审查平台 - Electron桌面应用，用于代码浏览和AI辅助代码理解。

**技术栈：**
- 前端：Electron + React + TypeScript + Monaco Editor
- 后端：Rust (Axum)
- 通信：REST API + WebSocket
- 状态管理：Zustand
- 样式：Tailwind CSS

**界面设计（三栏布局）：**
- 左侧边栏：目录树（上部源码，下部文档生成进度），用于代码导航
- 中间面板：Monaco Editor代码展示区域 + 知识图谱
- 右侧边栏：AI辅助问答区域

## 快速启动

**1. 安装依赖（首次运行）：**
```bash
# Windows 批处理
install.bat

# 或 PowerShell
.\install.ps1
```
需要预先安装 Node.js 和 Rust 工具链。

**2. 启动应用：**
```bash
# Windows 批处理
start.bat

# 或 PowerShell
.\start.ps1
```

## 手动启动

### 后端
```bash
cd backend-rs
cargo build --release
./target/release/backend-rs.exe
# 或开发模式
cargo run
# 运行测试
cargo test
```
服务器监听：`127.0.0.1:8765`

### 前端
```bash
cd frontend
npm install
npm run dev      # 开发模式
npm run build    # 构建
npm run build:win  # Windows 发行版
```

## 项目结构

```
├── frontend/                 # Electron + React
│   ├── src/main/            # Electron主进程（自动管理后端进程）
│   ├── src/preload/         # 预加载脚本
│   └── src/renderer/        # React渲染进程
│       └── src/
│           ├── components/  # UI组件
│           ├── stores/      # Zustand状态管理
│           └── services/    # API服务封装
├── backend-rs/              # Rust后端 (Axum)
│   └── src/
│       ├── main.rs          # 入口
│       ├── api/             # REST API + WebSocket端点
│       ├── llm/             # LLM客户端（OpenAI + Anthropic）
│       ├── services/        # 业务逻辑（代码分析、文档生成）
│       └── config/          # 配置管理
```

## 关键文件

- `backend-rs/src/llm/client.rs` - LLM调用封装，支持OpenAI和Anthropic格式
- `backend-rs/src/api/chat.rs` - WebSocket聊天端点
- `backend-rs/src/api/docs.rs` - 文档生成API + WebSocket进度推送
- `backend-rs/src/services/doc_generator/` - 文档生成服务
- `frontend/src/main/index.ts` - Electron主进程，管理后端子进程
- `frontend/src/renderer/src/components/chat/ChatPanel.tsx` - AI聊天面板
- `frontend/src/renderer/src/stores/docStore.ts` - 文档生成状态管理

## 数据存储

| 数据类型 | 存储位置 |
|---------|---------|
| 配置文件 | `backend-rs.exe` 同级目录的 `config.json` |
| 生成的文档 | 项目根目录下 `.docs/` |
| 断点文件 | `.docs/.checkpoint.json` |

## 配置

API配置存储在后端可执行文件同级目录的 `config.json`，通过应用内设置界面修改。

## API 端点

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/api/health` | 健康检查 |
| GET/PUT | `/api/config` | 配置读取/更新 |
| POST | `/api/config/test` | 测试 LLM 连接 |
| POST | `/api/chat/suggest` | 获取建议问题 |
| WS | `/ws/chat` | WebSocket 聊天 |
| POST | `/api/graph/project` | 项目级知识图谱 |
| POST | `/api/graph/module` | 模块级知识图谱 |
| POST | `/api/docs/generate` | 启动文档生成 |
| WS | `/ws/docs/{task_id}` | 文档生成进度推送 |
| POST | `/api/docs/graph` | 获取项目 LLM 图谱 |
| POST | `/api/docs/file-graph` | 获取单文件 LLM 图谱 |

## 开发规范

### 语言要求
- 所有代码注释和文档必须使用中文
- 代码中禁止使用emoji，防止编码错误
- 后端日志和错误消息使用英文（避免Windows控制台编码问题）

### 问题解决日志
每次代码修改完成后需记录：
1. 发现的问题（为什么要改）
2. 解决的方式（怎么改）
3. 若同一问题再次出现，记录发现次数并深刻反思为何编码时未注意到

### 代码原则
- 调用API前必须查阅最新官方文档或源码中的接口定义
- 对不明确、有歧义的需求必须先确认再编码
- 实现新功能前优先搜索项目内是否已有可复用的代码
- 不实现回退方案或降级策略，功能无法正常执行直接报错
- 未使用的代码直接删除，不保留向后兼容的hack代码

## 问题解决日志

### 2026-02-04: 文档生成状态实时更新失败（第2次修复）

**问题描述：**
文档生成过程中，文件完成状态没有实时更新到UI界面。

**根本原因（共发现4个问题）：**

1. **broadcast channel 转发任务过早退出（最关键）**
   `docs.rs` 中 `let (tx, _) = broadcast::channel(100)` 初始接收器 `_` 立即被丢弃。当进度转发任务尝试 `tx.send(msg)` 时，由于没有活跃接收器（WebSocket 客户端尚未连接），`send` 返回 `Err`，导致转发任务 `break` 退出。之后 WebSocket 客户端连接时，已无任务在转发消息。

2. **docs_path 默认值不一致**
   `docs.rs` 中默认路径是 `{parent}/{name}_docs`，而 `processor.rs` 是 `.docs`。由于 `docs.rs` 先计算路径再传给 `processor.rs`，覆盖了正确的默认值。

3. **前端时序问题**
   `relativeToSourceMap` 在 WebSocket 消息到达时可能为空（原本依赖 React useEffect 延迟生成）。

4. **前端路径处理脆弱**
   使用简单字符串 `replace(projectPath, '')` 可能因路径格式差异失败。

**解决方式：**

1. 在 `docs.rs` 中保留初始接收器（`_keep_alive_rx`），移入转发任务的 async block 中保持存活；`tx.send()` 失败时不再 break
2. 统一 `docs.rs` 默认路径为 `source_path.join(".docs")`
3. 在 `docStore.ts` 的 `startGeneration` 中，API 响应后、WebSocket 订阅前立即生成文档树映射
4. 改用路径规范化方式计算相对路径（统一正斜杠后用 `startsWith` + `slice`）
5. 前端文件树扫描深度从4层增加到10层

**涉及文件：**
- `backend-rs/src/api/docs.rs`（broadcast channel 修复 + docs_path 默认值）
- `frontend/src/renderer/src/stores/docStore.ts`（时序 + 路径处理 + 映射逻辑）
- `frontend/src/renderer/src/components/file-tree/SplitFileTree.tsx`（注释更新）
- `frontend/src/main/index.ts`（深度限制）

**反思：**
第2次修复此问题。第1次只关注了前端时序，忽略了后端 WebSocket 消息根本没有被发送出去。应该优先检查消息发送端（后端）是否工作正常，再排查接收端（前端）的处理逻辑。

### 2026-02-04: 文档生成并发状态显示不完善 + 目录并发生成

**问题描述：**
1. 多个文件并发处理时，前端只能显示一个文件正在处理，无法同时显示多个 "processing" 状态
2. 目录文档生成是完全串行的，未能利用同层级目录可并发的特性
3. 文件和目录分成两个阶段处理（先全部文件，再全部目录），导致总结目录时可能缺少同层目录的文档信息

**根本原因：**
1. 后端只在 Progress 消息中携带 `current_file`（单个），缺少独立的 `FileStarted` 和 `DirStarted` 消息类型
2. 前端通过 Progress 消息的 `current_file` 更新状态，无法感知多个文件同时开始处理
3. 处理架构有误：应该按深度逐层处理（文件+目录统一），而不是分开两个阶段

**解决方式：**

1. **后端 `types.rs`：** 添加 `FileStarted` 和 `DirStarted` 消息类型

2. **后端 `processor.rs`：** 彻底重构处理架构
   - 移除原有的 `process_files_parallel` 和 `process_directories` 两个独立方法
   - 新增 `process_by_depth` 统一调度：按深度从深到浅，每层内先并发处理文件，再并发处理目录
   - 新增 `process_files_batch` 和 `process_dirs_batch` 处理具体的并发逻辑
   - 这样当处理某个目录时，它的所有子节点（包括文件和子目录）文档都已在更深层完成

3. **前端 `docService.ts`：** 添加 `onFileStarted` 和 `onDirStarted` 回调

4. **前端 `docStore.ts`：** 收到 Started 消息时将节点设为 `processing`，移除 Progress 消息中的状态更新

**涉及文件：**
- `backend-rs/src/services/doc_generator/types.rs`
- `backend-rs/src/services/doc_generator/processor.rs`（重构）
- `frontend/src/renderer/src/services/docService.ts`
- `frontend/src/renderer/src/stores/docStore.ts`

**反思：**
初次实现时将文件和目录分成两个独立阶段处理，没有考虑到目录总结依赖同层子节点的文档完整性。正确的架构应该以「深度」作为处理单元，自底向上逐层推进，这也是更自然的树形处理方式。

### 2026-02-04: 文档生成状态显示不完整（断点续传+消息竞争）

**问题描述：**
文档生成时，只有已完成的文档显示为绿色，但正在处理的文档不显示蓝色（processing状态）。

**根本原因（共发现2个问题）：**

1. **断点续传跳过的文件不发送WebSocket事件**
   `processor.rs` 中，当文件通过断点恢复被跳过时，没有发送 `FileCompleted`/`DirCompleted` 消息，导致前端无法将这些已完成的文件标记为绿色。

2. **WebSocket连接的竞争条件**
   后端在HTTP响应返回前就已经开始处理文件（spawn了异步任务），但WebSocket连接要等前端收到HTTP响应后才能建立。在这个时间差内发送的消息（包括 `FileStarted` 等）全部丢失，因为此时还没有WebSocket客户端订阅。

**解决方式：**

1. **修改 `processor.rs`：** 断点续传跳过文件/目录时也发送 `FileCompleted`/`DirCompleted` 消息

2. **修改 `state.rs`：** 新增 `TaskState` 结构，记录已完成的路径（区分文件和目录类型）
   - 新增 `CompletedPathType` 枚举区分文件和目录
   - `mark_file_completed()`/`mark_dir_completed()` 方法记录历史

3. **修改 `api/docs.rs`：**
   - 进度转发任务中记录已完成的路径到 `TaskState`
   - WebSocket连接建立时，先重放所有历史完成消息，再订阅后续消息

**涉及文件：**
- `backend-rs/src/services/doc_generator/processor.rs`
- `backend-rs/src/state.rs`
- `backend-rs/src/api/docs.rs`

**反思：**
WebSocket消息重放是处理竞争条件的标准模式。之前没有考虑到HTTP响应和WebSocket连接之间的时间差会导致消息丢失。对于需要展示历史状态的场景，必须在服务端维护消息历史，并在连接建立时进行重放。

### 2026-02-04: 实现LLM知识图谱提取功能

**问题描述：**
原有的知识图谱是基于静态代码分析生成的，需要新增基于LLM的知识图谱提取功能，在每次文档生成调用时顺便进行图谱提取。

**实现方案：**

1. **修改Prompt模板** (`prompts.rs`)
   - 在 `CODE_ANALYSIS_PROMPT` 末尾添加知识图谱提取指令
   - 使用 `<!-- GRAPH_DATA_START -->` 和 `<!-- GRAPH_DATA_END -->` 标记分隔图谱JSON

2. **添加图谱数据结构** (`types.rs`)
   - `LlmGraphNode`: 节点结构（id, label, type, line）
   - `LlmGraphEdge`: 边结构（source, target, type）
   - `FileGraphData`: 单文件图谱数据
   - `LlmGraphRawData`: LLM返回的原始JSON结构
   - `ProjectGraphData`: 项目级聚合图谱

3. **修改文档生成器** (`generator.rs`)
   - 新增 `FileAnalysisResult` 结构，同时返回文档内容和图谱数据
   - 新增 `parse_llm_response()` 方法解析LLM响应，分离文档和图谱JSON
   - 新增 `save_file_graph()` 方法保存单文件图谱为 `.graph.json`

4. **修改处理器** (`processor.rs`)
   - 在 `process_files_batch()` 中调用 `save_file_graph()` 保存图谱
   - 新增 `aggregate_project_graph()` 方法聚合所有文件图谱为 `_project_graph.json`
   - 新增 `collect_graph_files()` 递归收集所有 `.graph.json` 文件

5. **修改断点服务** (`checkpoint.rs`)
   - 添加 `project_graph_completed` 字段跟踪聚合状态
   - 新增 `mark_project_graph_completed()` 和 `is_project_graph_completed()` 方法

6. **添加后端API** (`api/docs.rs`)
   - 新增 `POST /api/docs/graph` 端点读取项目图谱文件

7. **前端集成** (`graphStore.ts`, `KnowledgeGraph.tsx`)
   - 新增 `graphSource` 状态切换静态/LLM图谱
   - 新增 `loadLLMProjectGraph()` 方法加载LLM图谱
   - 添加 Static/LLM 切换按钮

**涉及文件：**
- `backend-rs/src/services/doc_generator/prompts.rs`
- `backend-rs/src/services/doc_generator/types.rs`
- `backend-rs/src/services/doc_generator/generator.rs`
- `backend-rs/src/services/doc_generator/processor.rs`
- `backend-rs/src/services/doc_generator/checkpoint.rs`
- `backend-rs/src/services/doc_generator/mod.rs`
- `backend-rs/src/api/docs.rs`
- `frontend/src/renderer/src/stores/graphStore.ts`
- `frontend/src/renderer/src/components/graph/KnowledgeGraph.tsx`

**技术要点：**
- 使用HTML注释标记（`<!-- -->`）分隔结构化数据，避免影响Markdown渲染
- 单文件图谱存储为 `{filename}.graph.json`，便于增量更新
- 项目级图谱聚合时合并所有节点和边，去重处理
- 前端通过 `docsPath` 判断LLM图谱是否可用

### 2026-02-04: 同一深度的文件和目录未真正并发处理（第2次修复）

**问题描述：**
文档生成时，同一深度层级的文件和目录实际上是分开处理的，而不是真正交错并发。

**根本原因：**
使用 `tokio::join!` 分别启动 `process_files_batch` 和 `process_dirs_batch` 两个 future，看似并发但实际上：
1. 每个 batch 内部都有自己的 `for_each_concurrent` 流调度
2. 两个流独立地从各自的迭代器中拉取任务
3. 由于流调度的时序差异，可能导致一个流先占满信号量，另一个流等待
4. 最终表现为文件先全部处理，再处理目录

**解决方式：**

1. 将文件和目录合并成单一的 `NodeTask` 枚举类型：
   ```rust
   enum NodeTask {
       File { name: String, relative_path: String, path: PathBuf },
       Dir { name: String, relative_path: String, path: PathBuf },
   }
   ```

2. 在 `process_by_depth` 中交错合并文件和目录到一个 `Vec<NodeTask>`

3. 使用单一的 `process_merged_batch` 方法统一调度所有任务

4. 根据任务类型分发到 `process_single_file` 或 `process_single_dir` 处理

**涉及文件：**
- `backend-rs/src/services/doc_generator/processor.rs`

**反思：**
第2次修复此问题。第1次尝试用 `tokio::join!` 并发两个独立的批处理流，但忽略了流调度的内部时序问题。正确的做法是将所有任务放入同一个流中统一调度，这样 `for_each_concurrent` 才能真正实现任务级别的交错并发。

### 2026-02-04: 知识图谱不随文件切换更新

**问题描述：**
知识图谱页面只显示项目级图谱，切换文件时图谱内容不变，没有显示当前文件对应的 LLM 知识图谱。

**根本原因：**
1. 只实现了加载项目级图谱 (`_project_graph.json`) 的功能
2. 没有实现加载单文件图谱 (`.graph.json`) 的 API 和方法
3. 前端没有监听当前文件变化

**解决方式：**

1. **后端添加 `/api/docs/file-graph` 端点** (`api/docs.rs`)
   - 接收 `docs_path` 和 `file_path` 参数
   - 读取 `docs_path/{dir}/{filename}.graph.json` 文件并返回

2. **前端添加 `loadLLMFileGraph` 方法** (`graphStore.ts`)
   - 调用新的 `/api/docs/file-graph` 端点
   - 更新 store 中的图谱数据

3. **修改 KnowledgeGraph 组件** (`KnowledgeGraph.tsx`)
   - 监听 `activeFile` 变化
   - 切换文件时自动调用 `loadLLMFileGraph` 加载对应图谱
   - 使用 `useRef` 避免重复加载同一文件

**涉及文件：**
- `backend-rs/src/api/docs.rs`
- `frontend/src/renderer/src/stores/graphStore.ts`
- `frontend/src/renderer/src/components/graph/KnowledgeGraph.tsx`

**反思：**
最初实现知识图谱功能时只考虑了项目级聚合图谱，忽略了用户实际需要的是查看当前文件的图谱。应该从用户使用场景出发设计功能，而不是只关注数据的存储和聚合。

### 2026-02-04: 目录级知识图谱提取功能

**问题描述：**
原有的知识图谱只在文件级别提取，目录级别没有图谱数据。项目级聚合图谱中缺少目录节点和目录包含关系。

**实现方案：**

1. **修改目录总结 Prompt** (`prompts.rs`)
   - 在 `DIRECTORY_SUMMARY_PROMPT` 中添加知识图谱提取指令
   - 要求 LLM 提取目录内模块关系：contains、depends、calls、inherits 等
   - 使用相同的 `<!-- GRAPH_DATA_START -->` 标记格式

2. **添加目录图谱数据结构** (`types.rs`)
   - 新增 `DirGraphData` 结构，包含 dir_path、dir_id、nodes、edges、imports 字段
   - 与 `FileGraphData` 结构类似，但使用目录相关的命名

3. **修改文档生成器** (`generator.rs`)
   - 新增 `DirAnalysisResult` 结构，同时返回文档内容和目录图谱数据
   - 重构 `parse_llm_response` 为 `parse_llm_response_raw`，返回 `LlmGraphRawData` 原始数据
   - 调用方根据需要创建 `FileGraphData` 或 `DirGraphData`
   - 新增 `get_dir_graph_path()` 和 `save_dir_graph()` 方法

4. **修改处理器** (`processor.rs`)
   - 修改 `process_single_dir()` 保存目录图谱数据
   - 修改 `aggregate_project_graph()` 处理文件图谱和目录图谱两种类型
   - 新增 `generate_structure_edges()` 从文件树生成目录包含关系边

5. **前端更新** (`KnowledgeGraph.tsx`)
   - 在 `EDGE_STYLES` 中添加 `implements` 和 `depends` 边类型样式

**目录图谱文件格式：**
- 文件名: `_dir.graph.json`
- 位置: 每个目录下的 `.docs/{dir}/_dir.graph.json`

**项目图谱聚合改进：**
- 区分处理 `.graph.json`（文件图谱）和 `_dir.graph.json`（目录图谱）
- 添加目录节点（类型为 `directory`）
- 生成目录包含子节点的 `contains` 边

**涉及文件：**
- `backend-rs/src/services/doc_generator/prompts.rs`
- `backend-rs/src/services/doc_generator/types.rs`
- `backend-rs/src/services/doc_generator/generator.rs`
- `backend-rs/src/services/doc_generator/processor.rs`
- `frontend/src/renderer/src/components/graph/KnowledgeGraph.tsx`

### 2026-02-05: 知识图谱布局算法优化

**问题描述：**
原有的图谱布局算法简单按类型分行排列，没有考虑边的关系，导致：
1. 连接线可能出现大量交叉
2. 相关节点没有靠近放置
3. 布局不够美观和易读

**解决方式：**

实现了基于 Sugiyama 算法思想的层次化布局算法：

1. **层级分配** (`assignLayers`)
   - 根据节点类型优先级分配层级（file→class→function→method）
   - 结合 `contains` 边的拓扑关系，确保父节点在子节点上方
   - 处理循环依赖等边界情况

2. **边交叉最小化** (`minimizeCrossings`)
   - 使用重心法（Barycenter Method）进行迭代优化
   - 考虑所有边类型（contains、calls、imports、inherits等）
   - 从上到下、从下到上双向扫描，共6次迭代

3. **坐标分配** (`assignPositions`)
   - 每层水平居中对齐
   - 使用固定间距避免节点重叠
   - 可配置的布局参数（nodeWidth、horizontalGap、verticalGap等）

**布局配置参数：**
```typescript
const LAYOUT_CONFIG = {
  nodeWidth: 180,      // 节点宽度
  nodeHeight: 40,      // 节点高度
  horizontalGap: 60,   // 水平间距
  verticalGap: 80,     // 垂直间距（层间距）
  padding: 50          // 边缘留白
}
```

**涉及文件：**
- `frontend/src/renderer/src/components/graph/KnowledgeGraph.tsx`

**技术要点：**
- 分离层级结构边（contains）和其他边类型用于不同目的
- 层级分配用 contains 边建立树形结构
- 交叉最小化用所有边类型计算重心
- 无连接的节点放在层末尾，按字母顺序排列

### 2026-02-06: 知识图谱布局与可读性优化（第2次优化）

**问题描述：**
原有布局虽然使用了 Dagre 库，但所有边权重相同、节点类型没有层级约束，导致：
1. 不同类型的节点（file/class/function/method）混排，缺乏层次感
2. 边的粗细和样式统一，不同关系类型难以区分
3. contains 结构关系和 calls/imports 逻辑关系在视觉上没有主次之分
4. 节点较多时无法快速识别某个节点的关联关系

**解决方式：**

1. **节点类型分层约束**
   - 定义 `NODE_TYPE_RANK` 映射：file/module(0) > class/interface/struct/enum(1) > function/constant(2) > method(3)
   - 在相邻层级间添加弱辅助边（weight=0.1）引导 Dagre 将同类型节点放在同一层

2. **边权重分级**
   - contains=10（最强，强制父子垂直排列），inherits/implements=5，imports=2，calls/depends=1
   - contains 边 minlen=1（紧密排列），其他边 minlen=2（允许跨层）

3. **边样式层次化**
   - 逻辑关系（imports/calls/inherits）使用粗线（strokeWidth=2~2.5），颜色鲜明
   - 结构关系（contains）使用细点线（strokeWidth=1, opacity=0.5），灰色淡化
   - contains 边不显示标签和使用更小的箭头，减少视觉噪音

4. **关联高亮交互**
   - 悬停节点时淡化不相关的节点（opacity=0.15）和边（opacity=0.05）
   - 使用 `baseNodesRef`/`baseEdgesRef` 保存原始数据，离开时恢复
   - 过渡动画 0.15s ease

5. **图例指示**
   - 状态栏右侧添加边类型图例，显示 imports/calls/inherits/contains 的颜色和线型

6. **Dagre 算法优化**
   - 使用 `ranker: 'tight-tree'` 算法减少边交叉
   - 边标签添加半透明背景避免与边线重叠

**涉及文件：**
- `frontend/src/renderer/src/components/graph/KnowledgeGraph.tsx`

**反思：**
第2次优化图谱布局。第1次实现了自定义 Sugiyama 算法但后来替换为 Dagre 库。Dagre 的优势在于内置了成熟的边交叉最小化，但需要通过 weight/minlen 参数和辅助边来表达节点类型层级约束。视觉层次的关键是将结构关系（contains）和逻辑关系（calls/imports）在样式上明确区分主次。

### 2026-02-06: 已有文档的项目无法显示 Code + Docs 分屏

**问题描述：**
打开一个已有 `.docs/` 文件夹的项目时，无法同时查看代码和 LLM 解释文档。"Code + Docs" 按钮不显示，分屏模式无法启用。

**根本原因（共发现2个问题）：**

1. **`canShowDocs` 条件只检查 `docStore` 的状态**
   `MainLayout.tsx` 中 `canShowDocs` 依赖 `docStore.docsPath` 和 `docStore.status`。但 `docStore` 只在当前会话启动文档生成时才会设置 `docsPath` 和更新 `status`。当用户打开已有文档的项目时，`fileStore.docsPath` 被正确设置（`loadProject()` 检测到 `.docs/` 目录），但 `docStore.docsPath` 为 `null`，`docStore.status` 为 `'idle'`。导致 `canShowDocs = false`，"Code + Docs" 按钮被隐藏。

2. **`editorStore.docsBasePath` 未从 `fileStore` 同步**
   `MainLayout` 只将 `docStore.docsPath` 同步到 `editorStore.docsBasePath`。已有文档的项目中 `docStore.docsPath` 为 `null`，所以 `editorStore.docsBasePath` 也为 `null`，导致 `loadDocForFile()` 无法找到文档文件。

**解决方式：**

1. **引入 `effectiveDocsPath`** - 综合 `docStore.docsPath`（当前会话生成）和 `fileStore.docsPath`（已有文档目录），取第一个非空值
2. **修改 `canShowDocs` 条件** - 当 `fileStoreDocsPath` 存在时也允许显示文档，不再强制要求 `docGenStatus` 为特定值
3. **同步 `effectiveDocsPath` 到 `editorStore.docsBasePath`** - 无论文档来源是当前生成还是已有目录，都设置 `docsBasePath`
4. **自动启用分屏** - 当打开已有文档的项目（`fileStoreDocsPath` 非空且 `docGenStatus === 'idle'`）时，自动切换到 `code-and-docs` 模式

**涉及文件：**
- `frontend/src/renderer/src/components/layout/MainLayout.tsx`

**反思：**
三个 store（`fileStore`、`docStore`、`editorStore`）各自管理部分 `docsPath` 相关状态，但缺乏统一的来源：`fileStore.docsPath`（加载项目时检测）、`docStore.docsPath`（文档生成时设置）、`editorStore.docsBasePath`（用于文档加载）。应该在设计时考虑"项目已有文档"和"当前会话生成文档"两种场景的统一处理。

### 2026-02-06: 文档生成假完成问题（文件未生成但状态显示完成）

**问题描述：**
文档生成过程中存在某些文件显示为"生成成功"但实际文件不存在的情况，导致后续打开文档时出现找不到文件的错误。

**根本原因（共发现3个问题）：**

1. **断点续传不验证文件实际存在**（最关键）
   `checkpoint.rs` 中 `is_file_completed()` 仅检查路径是否在 `HashSet` 中，不验证对应的 `.md` 文件是否真正存在于磁盘上。如果上一次生成中断时断点已记录完成但文件写入失败（如磁盘满、权限问题、进程被杀），续传时会跳过该文件并发送 `FileCompleted` 消息，导致前端显示为完成。

2. **LLM 空响应未验证**
   `generator.rs` 中 `analyze_file()` 和 `summarize_directory()` 没有检查 LLM 返回内容是否为空。如果 LLM 返回空字符串（网络超时、token 限制、服务异常），空内容会被保存为文档并标记完成。

3. **文件写入后未验证**
   `save_document()` 在 `write_all()` 后直接返回 `Ok(())`，没有验证文件是否真正写入成功（存在且非空）。

**解决方式：**

1. **`checkpoint.rs` - 新增 `verify_file_completed()` 和 `verify_dir_completed()` 异步方法**
   - 先检查记录是否存在
   - 然后验证对应的文档文件是否存在且非空（通过 `fs::metadata` 检查文件大小）
   - 如果文件不存在或为空，自动清除该记录（从 `completed_files`/`completed_dirs` 和 `doc_path_map` 中移除），返回 `false`

2. **`processor.rs` - 断点检查改用验证方法**
   - `process_single_file()` 中将 `checkpoint.read().await.is_file_completed()` 改为 `checkpoint.write().await.verify_file_completed().await`
   - `process_single_dir()` 中同样改用 `checkpoint.write().await.verify_dir_completed().await`

3. **`generator.rs` - LLM 响应内容验证**
   - `analyze_file()` 中在获取 LLM 响应后检查 `result.content.trim().is_empty()`，为空则返回错误
   - 解析后的 `doc_content` 也检查是否为空
   - `summarize_directory()` 中添加相同的验证逻辑

4. **`generator.rs` - 文件写入后验证**
   - `save_document()` 中 `write_all()` 后通过 `fs::metadata()` 检查文件存在且 `len() > 0`

**涉及文件：**
- `backend-rs/src/services/doc_generator/checkpoint.rs`（新增 verify 方法）
- `backend-rs/src/services/doc_generator/processor.rs`（改用 verify 方法）
- `backend-rs/src/services/doc_generator/generator.rs`（LLM 响应验证 + 写入验证）

**反思：**
核心问题是"完成标记"和"文件实际存在"之间缺乏一致性保证。断点续传机制信任了历史记录但没有验证实际状态。正确的做法是在任何涉及"跳过已完成任务"的场景中，都要验证产出物确实存在。这是一种防御性编程实践：不信任缓存/记录，始终验证实际状态。

### 2026-02-06: 实现文档生成失败即停止机制

**问题描述：**
原有的文档生成流程在某个文件/目录处理失败时只记录日志并继续处理，导致最终生成不完整的文档树。用户需要完整生成才能达到最佳效果。

**解决方式：**

实现"快速失败"（Fail-Fast）机制：任何文件或目录处理失败时立即停止整个生成流程。

1. **`process_merged_batch` - 并发任务失败检测**
   - 每个并发任务开始前检查 `TaskStatus::Failed`（除了原有的 `Cancelled`）
   - 批处理完成后检查任务状态，如果是 `Failed` 则返回 `ProcessorError::GeneratorError`

2. **`process_single_file` - 文件处理失败触发停止**
   - LLM 分析失败时：调用 `task.fail(error_msg)` 设置任务状态为 Failed
   - 文档保存失败时：同上
   - 同时发送 `WsDocMessage::Error { message }` 通知前端

3. **`process_single_dir` - 目录处理失败触发停止**
   - 目录总结生成失败时：调用 `task.fail(error_msg)`
   - 目录文档保存失败时：同上
   - 同时发送 WebSocket 错误消息

4. **`generate_final_docs` - 最终文档生成失败触发停止**
   - README 生成/保存失败：返回 `ProcessorError::GeneratorError`
   - 阅读指南生成/保存失败：同上
   - 项目图谱聚合失败：同上
   - 每个失败都发送 WebSocket 错误消息

**涉及文件：**
- `backend-rs/src/services/doc_generator/processor.rs`

**技术要点：**
- 使用 `TaskStatus::Failed` 作为全局停止信号
- 并发任务通过检查任务状态实现协同停止
- `task.fail()` 方法同时设置 `status` 和 `error` 字段
- WebSocket 错误消息让前端能够及时显示失败原因
- 断点机制保证失败后可以重新运行续传

### 2026-02-06: 实现最近打开项目功能

**需求描述：**
用户希望能够记录最近打开的项目，方便快速重新打开。

**实现方案：**

1. **Electron 主进程** (`frontend/src/main/index.ts`)
   - 使用 `app.getPath('userData')` 下的 `recent-projects.json` 文件持久化存储
   - `RecentProject` 接口：path、name、lastOpened（Unix时间戳）
   - `loadRecentProjects()` / `saveRecentProjects()` / `addRecentProject()` 存储函数
   - 最多保留 10 个最近项目
   - IPC 处理器：`app:getRecentProjects`、`app:addRecentProject`、`app:removeRecentProject`

2. **预加载脚本** (`frontend/src/preload/index.ts`, `index.d.ts`)
   - 暴露 `getRecentProjects()`、`addRecentProject()`、`removeRecentProject()` API

3. **状态管理** (`frontend/src/renderer/src/stores/fileStore.ts`)
   - 新增 `recentProjects` 状态
   - 新增 `openProjectByPath(path)` 方法：加载项目并记录到最近列表
   - 重构 `loadProject()` 复用 `openProjectByPath`
   - 新增 `loadRecentProjects()` 和 `removeRecentProject()` 方法

4. **工具栏 UI** (`frontend/src/renderer/src/components/layout/Toolbar.tsx`)
   - "Open Project" 按钮拆分为左按钮（打开）+ 右按钮（下拉箭头）
   - 下拉菜单显示最近项目列表：项目名、完整路径、相对时间
   - 当前已打开的项目高亮显示
   - 每个项目右侧有删除按钮（hover 时显示）
   - 点击外部自动关闭下拉菜单

**涉及文件：**
- `frontend/src/main/index.ts`（存储函数 + IPC 处理器）
- `frontend/src/preload/index.ts`（API 暴露）
- `frontend/src/preload/index.d.ts`（类型声明）
- `frontend/src/renderer/src/stores/fileStore.ts`（状态管理）
- `frontend/src/renderer/src/components/layout/Toolbar.tsx`（UI）

### 2026-02-06: 目录文档路径错误（_summary.md vs _dir_summary.md）

**问题描述：**
点击 DOCS 栏的目录时报错 `ENOENT: no such file or directory`，找不到 `_summary.md` 文件。

**根本原因：**
后端生成的目录总结文件名是 `_dir_summary.md`（在 `DocGenConfig` 中定义），但前端两处错误地硬编码为 `_summary.md`。

**解决方式：**
1. **`editorStore.ts`** 第205-207行：`_summary.md` → `_dir_summary.md`
2. **`docStore.ts`** 第119行：`_summary.md` → `_dir_summary.md`

**涉及文件：**
- `frontend/src/renderer/src/stores/editorStore.ts`
- `frontend/src/renderer/src/stores/docStore.ts`

**反思：**
文件名应该从配置或常量中统一获取，而不是在多处硬编码。前端有 `docPathMapper.ts` 中定义了 `DIR_SUMMARY_NAME = '_dir_summary.md'`，但其他地方没有使用这个常量。

### 2026-02-06: 完善最近项目功能（自动打开+UI优化）

**问题描述：**
1. 启动应用时没有自动打开上次的项目
2. 最近项目下拉按钮高度与主按钮不一致，视觉效果差

**解决方式：**

1. **自动打开上次项目** - 在 `Toolbar.tsx` 添加 useEffect：
   - 使用 `hasAutoOpenedRef` 防止重复触发
   - 当 `recentProjects` 加载完成且当前没有打开项目时，自动调用 `openProjectByPath(recentProjects[0].path)`

2. **UI 优化** - 使用固定高度确保按钮一致：
   - 主按钮和下拉箭头都使用 `h-[26px]` 固定高度
   - 移除 `py-1` 改为 `flex items-center` 垂直居中
   - 箭头图标从 `w-3.5 h-3.5` 调整为 `w-3 h-3`

**涉及文件：**
- `frontend/src/renderer/src/components/layout/Toolbar.tsx`

