# CLAUDE.md

本文件为 Claude Code (claude.ai/code) 在此仓库中工作时提供指导。

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
```
服务器监听：`127.0.0.1:8765`

### 前端
```bash
cd frontend
npm install
npm run dev      # 开发模式
npm run build    # 构建
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

