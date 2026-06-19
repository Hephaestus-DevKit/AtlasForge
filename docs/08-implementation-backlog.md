# 实施分层与任务清单

用户明确不需要时间规划，因此这里按“可交付层”组织，而不是按日期组织。每一层都应该形成可运行、可验证的闭环。

## Layer 0：项目骨架

目标：建立可持续开发基础。

交付物：

- Tauri + React + TypeScript 项目。
- 基础路由和布局。
- SQLite migration 系统。
- typed IPC。
- 日志系统。
- 单元测试和基础 E2E。
- 开发、构建、测试脚本。

验收：

- 应用可启动。
- Dashboard 空状态可见。
- 数据库可创建和迁移。
- 一个 IPC command 能被 UI 调用。
- 测试命令可运行。

## Layer 1：Workspace Roots

目标：让用户授权本机目录，并可安全扫描。

任务：

- root 添加/编辑/删除。
- read-only/read-write 标记。
- include/exclude globs。
- root 权限校验。
- ignore 默认规则。
- scan job 初版。

验收：

- 未授权路径读取失败。
- 只读 root 写入失败。
- 能发现目录下 repo。
- 扫描错误显示在 UI。

## Layer 2：Repo Discovery

目标：建立多仓资产地图。

任务：

- 发现 `.git` repo。
- 识别 worktree/submodule。
- 读取 `git status --porcelain=v2`。
- 读取 branch、remote、head sha。
- 识别 package files。
- 展示 repo table。

验收：

- repo 列表能显示 path、branch、dirty、remote。
- nested repo 不重复误判。
- 大目录扫描不会卡死 UI。

## Layer 3：Repo Profile

目标：理解项目技术栈和运行方式。

任务：

- package manager detector。
- language detector。
- framework detector。
- scripts detector。
- CI detector。
- README/license detector。
- profile detail view。

验收：

- TypeScript/Node、Python、Rust、Android、静态站至少能识别基础信息。
- 能列出 test/build/lint scripts。
- profile 可持久化。

## Layer 4：Job Engine

目标：所有长任务持久化。

任务：

- job 表。
- job event 表。
- job queue。
- 状态机。
- 取消、失败、重试。
- UI event timeline。

验收：

- scan 作为 job 运行。
- 刷新应用后 job 记录仍存在。
- 失败 job 有错误摘要。

## Layer 5：Tool Broker

目标：把危险能力关进统一入口。

任务：

- tool registry。
- input/output schema。
- permission checker。
- risk level。
- audit writer。
- output redaction。
- dry-run protocol。

基础工具：

- `fs.list`
- `fs.read`
- `fs.write_patch`
- `git.status`
- `git.diff`
- `shell.verify`

验收：

- UI/worker 不直接执行 shell。
- 工具调用都有 audit event。
- forbidden path 被拒绝。

## Layer 6：Indexing v1

目标：本地全文索引可用。

任务：

- SourceDocument。
- Chunk。
- SQLite FTS5。
- Markdown chunker。
- config chunker。
- code rough chunker。
- search UI。

验收：

- 可搜索 README、package scripts、任务报告。
- 搜索结果带来源路径。
- `.env` 默认不索引。

## Layer 7：Repo Audit v1

目标：生成第一版健康报告。

任务：

- runnable checker。
- test checker。
- CI checker。
- docs checker。
- release checker。
- security surface checker。
- finding model。
- health view。

验收：

- 任意 repo 可生成结构化 report。
- finding 有 evidence。
- report 可保存并再次打开。

## Layer 8：AI Provider v1

目标：AI 能基于 ContextPack 生成计划和报告。

任务：

- provider abstraction。
- OpenAI-compatible adapter。
- Ollama adapter。
- model config UI。
- context retrieval。
- structured output parser。
- task report generation。

验收：

- 无 provider 时系统仍能扫描和审查。
- 有 provider 时能生成 repo audit summary。
- 发送上下文前做 secret scan。

## Layer 9：AI Assisted Fix

目标：AI 能生成和应用受控 patch。

任务：

- fix task template。
- patch proposal artifact。
- diff review UI。
- apply/reject。
- verification trigger。
- rollback baseline。

验收：

- AI 修改只发生在授权 repo。
- 应用后自动显示 diff。
- 验证失败能标记任务 failed 或需要继续修复。

## Layer 10：Verification Engine

目标：测试和构建变成任务证据。

任务：

- command detector。
- command runner。
- output summarizer。
- timeout。
- log artifact。
- Playwright smoke adapter。

验收：

- 能运行 npm/pnpm/cargo/python 基础验证。
- exit code 和日志摘要进入报告。
- 超时可取消。

## Layer 11：GitHub Read Integration

目标：读远端状态。

任务：

- gh auth status detector。
- GitHub repo resolver。
- workflow runs reader。
- PR reader。
- releases reader。
- Pages status reader。

验收：

- repo detail 显示最近 workflow run。
- release 列表可见。
- 认证失败有可理解提示。

## Layer 12：GitHub Write Integration

目标：半自动 PR 和 release。

任务：

- create PR。
- comment PR。
- rerun workflow。
- create tag。
- create release。
- upload artifacts。
- wait for checks。

验收：

- 所有写操作需要权限。
- 发布前展示 remote/branch/tag。
- 操作后用 API 核验。

## Layer 13：Semantic Index

目标：语义搜索和历史记忆。

任务：

- embedding queue。
- Ollama embedding adapter。
- LanceDB adapter。
- hybrid search。
- knowledge item writer。
- project manual generator。

验收：

- 能用自然语言找历史任务和项目说明。
- embedding model 版本被记录。
- 可重建索引。

## Layer 14：Automations

目标：长期维护开始运行。

任务：

- automation rule。
- scheduler。
- trigger。
- notification。
- max risk。
- result digest。

验收：

- 可定期扫描。
- 可监控 CI failure。
- 默认不自动 push/release。

## Layer 15：Public Product Polish

目标：把 AtlasForge 自己也做成可发布产品。

任务：

- README。
- screenshots。
- installer。
- release workflow。
- update strategy。
- telemetry opt-in/out。
- docs site。

验收：

- 新机器可安装。
- release 可下载。
- 基础 smoke 通过。

## 跨层技术债清单

- Windows 路径编码。
- 大目录扫描性能。
- 文件 watcher 去抖。
- 数据库迁移和备份。
- 输出脱敏准确率。
- AI 上下文预算。
- GitHub rate limit。
- 本地模型不可用时降级。
- 任务恢复。
- 插件隔离。

## 推荐执行顺序

不按时间，但按依赖：

1. Layer 0-2：先看到资产。
2. Layer 3-5：再建立可信执行边界。
3. Layer 6-7：再让系统能审查和检索。
4. Layer 8-10：再引入 AI 修复和验证。
5. Layer 11-12：再接远端 GitHub 写操作。
6. Layer 13-14：再沉淀长期记忆和自动化。
7. Layer 15：最后包装发布。

