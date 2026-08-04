# 系统架构

## 架构目标

AtlasForge 必须同时满足：

- 能像桌面应用一样访问本机文件、Git、进程和通知。
- 能像工程平台一样管理任务、审计、权限、验证和发布。
- 能像知识库一样索引文档、代码、日志和历史经验。
- 能像 AI agent runtime 一样调度模型和工具。

因此采用分层架构，而不是单一聊天应用。下文同时描述长期边界和 0.1.0
已经落地的薄闭环；规划能力是否可用以 `13-capability-matrix.md` 为准。

## 0.1.0 代码边界

```text
src/pages              页面状态与工作流编排
    -> src/features    领域 UI 与可复用交互
    -> src/api/ipc     唯一前端 IPC 边界
        -> commands    Tauri 命令参数校验与领域编排
            -> Rust domain modules
               workspace / scanner / profiler / indexer / auditor
               permissions / verification / ai_provider / ai_fix
               github / job_engine / automations / tool_broker
            -> db + append-only migrations
```

- UI 不直接持有文件系统、Shell、Git 或 GitHub mutation 权限。
- `commands.rs` 是适配和编排层；可独立测试的校验、查询和领域行为放入专用模块。
- 页面不继续堆积领域子视图；大型交互按 `src/features/<domain>` 拆分。
- 扫描、索引、审计和 GitHub 同步中的阻塞工作在 Tauri blocking worker 上运行。
- Rust 领域适配器是 0.1.0 的实际 service layer；当前没有额外 TypeScript worker 或 Python 核心运行时。

## 总体结构

```text
┌──────────────────────────────────────────────────────────────┐
│ UI Layer                                                     │
│ React/TypeScript dashboard, task console, repo views          │
└───────────────┬──────────────────────────────────────────────┘
                │ typed IPC / event stream
┌───────────────▼──────────────────────────────────────────────┐
│ Trusted App Core                                             │
│ Tauri/Rust command broker, permissions, audit, OS integration │
└───────────────┬──────────────────────────────────────────────┘
                │ restricted tool calls
┌───────────────▼──────────────────────────────────────────────┐
│ Service Layer                                                │
│ job engine, indexer, repo scanner, AI orchestrator, validators│
└───────┬─────────────┬───────────────┬───────────────┬────────┘
        │             │               │               │
┌───────▼──────┐ ┌────▼─────┐ ┌───────▼──────┐ ┌──────▼──────┐
│ Local Store  │ │ Search   │ │ Tool Adapters │ │ AI Providers│
│ SQLite       │ │ FTS5     │ │ git/gh/fs     │ │ OpenAI/Ollama│
└──────────────┘ └──────────┘ └──────────────┘ └─────────────┘
```

## 进程模型

### Desktop Shell

职责：

- 启动 UI。
- 管理窗口、托盘、通知、更新入口。
- 持有本机可信权限边界。
- 暴露 typed IPC。

建议技术：

- Tauri 2。
- React + TypeScript + Vite。
- Tailwind 或稳定组件库，但 UI 不应被组件库强绑定。

### App Core

职责：

- 权限校验。
- Tool Broker。
- 审计日志写入。
- 文件根目录 allowlist。
- 高风险操作审批。
- 启动和监督 worker。
- OS keychain、通知、文件选择器、系统信息。

建议技术：

- Rust/Tauri commands。
- SQLite 连接可放 Rust，也可由服务层持有；权限和审计入口必须在 Core。

### Service Worker

职责：

- 扫描仓库。
- 调用 Git/GitHub。
- 索引文件。
- 调度 AI。
- 运行验证。
- 处理长任务队列。

当前实现：

- 文件扫描、画像、索引、审计、Git/GitHub 和验证均由 Rust 模块执行。
- 长任务状态持久化到 SQLite；阻塞操作不在前端状态或 Tauri 异步线程中长期运行。
- Python 只保留为未来可选工具运行时，不是核心依赖。

## 模块分解

### 1. Workspace Manager

职责：

- 管理用户授权的扫描根目录。
- 保存 include/exclude 规则。
- 发现项目、repo、文档目录、构建产物。
- 支持手动添加、暂停扫描、重新扫描。

关键规则：

- 未授权 root 不读。
- 默认忽略 `.git/objects`、`node_modules`、`.venv`、`dist`、`build`、大二进制、缓存目录。
- 扫描要增量化，用 mtime、size、hash、inode/path fingerprint 判断变化。

### 2. Repository Intelligence

职责：

- 识别 Git repo、worktree、submodule。
- 读取 remotes、branches、tags、status、ahead/behind。
- 识别语言和框架。
- 识别 package manager、test/build/lint scripts。
- 读取 CI、release、docs、license。
- 生成 repo health snapshot。

输出：

- `RepositoryProfile`
- `RepoHealthReport`
- `MaintenancePlan`

### 3. Indexing Pipeline

职责：

- 把文件、README、配置、任务日志、审查报告转成可检索 document。
- 0.1.0 执行 chunk、metadata 和 SQLite FTS5；embedding/vector 是后续保留点。
- 维护索引版本和重建队列。
- 支持按 root/repo/project/scope 查询。

流水线：

```text
discover files
  -> classify
  -> read allowed content
  -> normalize
  -> chunk
  -> write metadata
  -> update FTS5
  -> (future) enqueue embedding/vector upsert
```

### 4. AI Orchestrator

职责：

- 接收用户意图。
- 选择任务模板。
- 检索相关上下文。
- 选择模型。
- 让模型生成计划和工具调用。
- 调用 Tool Broker。
- 汇总证据并生成报告。

必须避免：

- 模型直接持有未过滤文件系统能力。
- 模型直接执行 shell。
- 模型在没有验证结果时宣称任务完成。

### 5. Tool Broker

职责：

- 工具注册。
- 参数 schema 校验。
- 权限校验。
- 风险分级。
- dry-run。
- 审计。
- 输出裁剪和脱敏。
- rollback hook。

0.1.x 已注册 Tool Broker 工具：

- Filesystem：`fs.list`、`fs.read`。
- Git：`git.status`、`git.diff`。

`shell.verify` 使用独立的检测、预览、单次审批、超时、进程树终止和结果持久化入口，不通过当前只读 Tool Broker 注册表暴露。

以下是扩展类型，不代表当前已开放：

- Filesystem：stat/write/patch/move/delete。
- Git：branch/commit/push/tag。
- GitHub：repo/pr/issues/actions/releases/pages。
- Shell：受控命令执行。
- Browser：网页打开、截图、DOM 检查。
- Package：npm/pnpm/uv/cargo/gradle 等包管理器。
- Index：查询知识库。

### 6. Job Engine

职责：

- 持久化任务。
- 状态机。
- 事件流。
- 失败重试。
- 取消和恢复。
- 子任务依赖。
- 并发限制。

任务状态：

```text
created -> queued -> planning -> awaiting_permission -> running
running -> verifying -> summarizing -> completed
running -> paused
running -> failed
running -> cancelled
completed -> archived
```

### 7. Verification Engine

职责：

- 根据 repo 类型选择验证命令。
- 运行 lint/test/build/smoke。
- 解析结果。
- 采集日志摘要。
- 标记阻塞问题。
- 把验证证据写入任务报告。

验证策略：

- 先用 repo 自带脚本。
- 再用 AtlasForge 的通用探测。
- 对网页项目增加 HTTP smoke 和 Playwright smoke。
- 对发布任务增加 GitHub Actions 和 release 状态检查。

### 8. Knowledge Memory

职责：

- 把任务产物总结成未来可检索记忆。
- 自动更新项目维护手册。
- 保存“命令-错误-解决”映射。
- 保存用户偏好和项目规则，但不保存敏感秘密。

## 通信协议

### IPC 原则

- UI 只调用 high-level command，例如 `startScan(rootId)`、`createTask(templateId, input)`。
- UI 不直接调用 shell、Git 或 GitHub mutation。
- 所有长任务用事件流返回。
- 所有 IPC 输入输出都有 schema。

### 事件格式

```json
{
  "eventId": "evt_...",
  "jobId": "job_...",
  "type": "tool_completed",
  "createdAt": "2026-01-01T00:00:00.000Z",
  "payload": {
    "tool": "git.status",
    "summary": "2 modified files, 0 staged files",
    "artifactRefs": ["artifact_..."]
  }
}
```

## 错误处理

错误分三类：

- User-actionable：需要用户授权、选择环境、登录、安装依赖、关闭占用文件。
- AI-actionable：模型可以换策略、修复代码、调整命令。
- System-actionable：应用 bug、数据库迁移失败、权限系统异常。

每个错误必须记录：

- 发生模块。
- 输入摘要。
- 原始错误摘要。
- 是否可重试。
- 下一步建议。

## 打包和分发

0.1.0 先支持 Windows 本机源码运行和源码发布。后续：

- Tauri installer。
- GitHub Release。
- 自动更新。
- crash/log export。

## 插件边界

AtlasForge 的插件不是任意代码注入。插件只允许声明：

- 新工具 adapter。
- 新扫描器。
- 新验证器。
- 新任务模板。
- 新报告渲染器。

每个插件必须声明权限和风险级别。

