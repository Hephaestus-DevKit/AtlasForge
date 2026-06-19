# 交给 AI 执行的提示词

这些提示词用于把 AtlasForge 分层交给 AI 实现。使用前让 AI 先阅读 `README.md`、`AGENTS.md` 和对应 docs。

## 总提示词

```text
你正在实现 AtlasForge：一个本地优先的个人 AI 工作台和多仓库维护平台。

先阅读：
- README.md
- AGENTS.md
- docs/00-product-definition.md
- docs/01-research-and-decisions.md
- docs/02-system-architecture.md
- docs/06-security-and-permissions.md
- 当前任务相关的 docs 文件

要求：
- 主动实现，不只给计划。
- 每次改动前先理解现有代码。
- 保持本地优先、安全可审计、权限边界清晰。
- 不要写入真实 secrets。
- 不要让 UI 直接执行 shell 或 GitHub mutation。
- 每个长任务都要有 job/event 记录。
- 完成后运行相关验证，并报告结果。
```

## Layer 0 提示词

```text
实现 AtlasForge Layer 0 项目骨架。

目标：
- 建立 Tauri + React + TypeScript 基础应用。
- 建立 SQLite migration 系统。
- 建立 typed IPC 示例。
- 建立基础布局：Dashboard / Assets / Repositories / Tasks / Knowledge / Automations / Settings。
- 建立日志和基础测试命令。

验收：
- 应用可启动。
- Dashboard 显示空状态。
- 数据库能初始化。
- UI 能调用一个 IPC command 并显示返回值。
- typecheck/lint/test/build 至少有可运行脚本。

不要实现复杂业务，先打牢骨架。
```

## Layer 1-2 提示词

```text
实现 AtlasForge Workspace Roots 和 Repo Discovery。

先阅读：
- docs/02-system-architecture.md
- docs/03-domain-model-and-indexing.md
- docs/06-security-and-permissions.md
- docs/08-implementation-backlog.md

目标：
- 用户能添加本机 root。
- root 有 read-only/read-write。
- 扫描 root 发现 Git repo。
- 读取 repo path、branch、remote、dirty state。
- Repositories 页面显示表格。

安全要求：
- 未授权路径不能读取。
- 只读 root 不能写。
- 默认忽略 node_modules、.git/objects、build、dist、.env。

验收：
- 用 fixture 或真实临时目录验证 clean/dirty/no remote repo。
- 扫描任务有 job/event 记录。
```

## Layer 3-5 提示词

```text
实现 Repo Profile、Job Engine 和 Tool Broker。

目标：
- 识别 package manager、语言、framework、scripts、CI、README、license。
- 所有 scan/audit 类长任务进入 Job Engine。
- Tool Broker 统一执行 fs/git/shell 只读和验证工具。
- 工具调用写审计事件。

重点：
- UI 不直接执行 shell。
- 工具 input/output schema 化。
- risk level 生效。
- forbidden path 和 read-only write 都要被拒绝。

验收：
- Repositories detail 能看到 profile。
- Task Console 能看到事件流。
- 权限测试通过。
```

## Layer 6-7 提示词

```text
实现 Indexing v1 和 Repo Audit v1。

目标：
- SQLite FTS5 全文索引。
- SourceDocument、Chunk、search UI。
- repo audit 生成 health snapshot 和 findings。

检查类别：
- runnable
- tests
- ci
- dependencies
- security
- docs
- release
- public_surface
- git_hygiene
- platform_compat

验收：
- 可以搜索 README/package/scripts/任务报告。
- 任意 repo 可生成结构化健康报告。
- finding 必须有 evidence，不写空泛建议。
- .env 不进入索引。
```

## Layer 8-10 提示词

```text
实现 AI Provider、AI Assisted Fix 和 Verification Engine。

目标：
- provider-neutral AI abstraction。
- OpenAI-compatible adapter。
- Ollama adapter。
- ContextPack 构建。
- AI 生成修复计划和 patch proposal。
- Diff Review。
- Apply patch。
- 运行验证命令。

安全要求：
- 发送云端模型前执行 secret scan。
- patch 不能越过授权 root。
- 高风险动作需要权限。
- 验证结果来自命令，不来自模型自述。

验收：
- 无 AI provider 时扫描和审查仍可用。
- 有 provider 时能生成 repo audit summary。
- AI patch 可预览、应用、验证、回滚。
```

## Layer 11-12 提示词

```text
实现 GitHub read/write integration。

目标：
- 读取 repo metadata、workflow runs、PR、releases、Pages。
- 创建 PR。
- rerun workflow。
- 创建 tag/release。
- 上传 release artifact。

安全要求：
- 优先使用 gh 现有认证，不在数据库保存 token。
- mutation 前展示 canonical remote、branch、目标 repo、风险。
- mutation 后用 GitHub API/gh 读回核验。
- 所有 mutation 写 audit。

验收：
- 认证缺失有明确提示。
- read-only GitHub 状态能显示。
- create PR/release 在测试 repo 上可完整跑通。
```

## Layer 13-14 提示词

```text
实现 Semantic Index 和 Automations。

目标：
- embedding queue。
- Ollama embedding adapter。
- LanceDB adapter。
- hybrid search。
- KnowledgeItem 写入。
- project manual generator。
- automation scheduler。

安全要求：
- embedding 记录 model/version/dimension。
- 可重建索引。
- 自动化默认只扫描和报告，不 push/release。
- secrets 不进入向量库。

验收：
- 自然语言能找项目历史和维护手册。
- 任务完成后能写入知识条目。
- 定期扫描自动化能运行并生成 digest。
```

## Release 提示词

```text
把 AtlasForge 自身整理成可发布版本。

要求：
- clean worktree。
- README 完整。
- screenshots。
- CI。
- installer/build artifact。
- version consistent。
- release notes。
- GitHub release。
- release 下载和启动 smoke。

发布前必须确认：
- origin/canonical remote。
- branch。
- tag。
- artifact。
- GitHub Actions 状态。

不要在未核验 release 状态前声称完成。
```

