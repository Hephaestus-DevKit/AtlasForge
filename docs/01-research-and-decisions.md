# 调研与技术决策

本文件记录 AtlasForge 的第一轮外部调研和架构取舍。目标不是追逐最新框架，而是为一个本地优先、长期维护、能执行高风险工程动作的 AI 工作台选稳定边界。

## 调研来源

优先参考官方文档和主流项目文档：

- Tauri 架构文档：<https://v2.tauri.app/concept/architecture/>
- Electron 官网：<https://www.electronjs.org/>
- GitHub REST API：<https://docs.github.com/en/rest>
- GitHub Actions workflow runs API：<https://docs.github.com/en/rest/actions/workflow-runs>
- GitHub Actions 安全使用参考：<https://docs.github.com/en/actions/reference/security/secure-use>
- GitHub CLI manual：<https://cli.github.com/manual/>
- SQLite FTS5：<https://sqlite.org/fts5.html>
- LanceDB quickstart：<https://docs.lancedb.com/quickstart>
- Qdrant overview：<https://qdrant.tech/documentation/overview/>
- Ollama embeddings：<https://docs.ollama.com/capabilities/embeddings>
- OpenAI function calling：<https://developers.openai.com/api/docs/guides/function-calling>
- OpenAI background mode：<https://developers.openai.com/api/docs/guides/background>
- Anthropic Model Context Protocol 介绍：<https://www.anthropic.com/news/model-context-protocol>
- Playwright：<https://playwright.dev/>
- Tantivy Rust docs：<https://docs.rs/tantivy/latest/tantivy/>

## 总体判断

AtlasForge 的难点不是某个模型调用，而是四个系统同时成立：

1. 本机资产索引系统：能长期扫描、去重、增量更新、检索。
2. 工程状态判断系统：能理解 repo、CI、release、依赖和文档质量。
3. AI 执行系统：能把任务拆解成可审计工具调用。
4. 安全控制系统：能防止 AI 越权、误删、误发布、泄漏隐私。

因此架构应当“核心保守、边缘可插拔”：

- 本地数据库、任务队列、权限、审计日志要稳定。
- AI provider、向量库、GitHub 接入、UI 框架可以替换。
- 所有外部动作必须通过能力适配器，不让模型直接拿到无限 shell。

## 决策 1：优先桌面本地应用，而不是纯网页 SaaS

### 选择

第一主线选择桌面本地应用，推荐 Tauri + React/TypeScript + Rust command layer。

### 理由

AtlasForge 必须读取本机目录、运行 Git、执行测试、管理本地索引、和 OS 级凭据或浏览器状态交互。纯网页应用会被浏览器沙箱限制，必须引入本地 agent；这会让架构变成“网页 + 本地 daemon”，反而更复杂。

Tauri 官方架构强调 WebView UI 与 Rust 后端通过消息传递交互，应用可以接入插件、系统托盘、更新器和文件系统能力。它不打包完整浏览器运行时，适合本地工具型产品。

### 备选

Electron 生态成熟、Node 能力强、调试便利，适合快速做重前端桌面应用。但 AtlasForge 的核心包含权限边界、系统调用、长期任务和本地安全，Tauri 的 Rust command boundary 更适合作为第一版可信层。

### 风险

- Rust/Tauri 学习成本高于纯 Electron。
- Windows WebView2 环境差异需要测试。
- 如果大量生态功能只在 Node 侧成熟，可能需要 Node sidecar。

### 缓解

- UI 和业务逻辑保持 TypeScript；Rust 只做系统边界、权限和命令 broker。
- 复杂索引、AI provider、GitHub API 可以先用 TypeScript worker。
- 所有能力通过 IPC contract 暴露，后续可替换实现语言。

## 决策 2：本地元数据用 SQLite，文本检索先用 FTS5

### 选择

SQLite 作为第一版主数据库；SQLite FTS5 作为第一版全文检索。

### 理由

SQLite 是本地应用最稳妥的内嵌数据库。FTS5 是 SQLite 官方全文检索虚拟表模块，适合文件名、README、配置、任务日志、项目手册和代码片段的关键词检索。

AtlasForge 的第一阶段更需要“可靠保存和查询状态”，而不是一开始就追求复杂检索排名。

### 风险

- FTS5 对代码搜索、中文分词、跨语言 tokenization 不一定足够。
- 大规模代码库和多模态资产会逐步超过简单 FTS 的舒适区。

### 缓解

- 把 SearchProvider 抽象出来。
- 初期 schema 保留 `search_documents`、`chunks`、`metadata` 和 `source_refs`，不要把 FTS5 写死到业务层。
- 后续可替换或叠加 Tantivy、LanceDB、Qdrant。

## 决策 3：语义索引用 LanceDB 起步，保留 Qdrant 升级路径

### 选择

第一版语义索引优先 LanceDB OSS，本地 embedded 模式；数据规模和协作需求上来后再考虑 Qdrant。

### 理由

LanceDB 文档明确支持作为 embedded database 在进程内运行，连接本地文件路径即可起步，适合桌面本地知识库。它能保存向量并做相似度检索，也能随着需求加入过滤、全文和混合检索能力。

Qdrant 更像独立向量搜索服务。官方文档强调 client-server 架构、dense/sparse hybrid retrieval、payload index、分片、复制和规模化能力。它适合后续 AtlasForge 进入多机器、团队版或超大资产规模时使用。

### 风险

- LanceDB 在桌面打包、Windows 文件锁、索引迁移方面需要实测。
- 向量模型升级会导致 embedding 维度、索引版本和召回结果变化。

### 缓解

- 所有 embedding 写入必须带 `embedding_model_id`、`embedding_dimension`、`chunker_version`。
- 支持同一 chunk 多套 embedding。
- 保留重建索引任务和迁移状态。

## 决策 4：AI 调用必须 provider-neutral

### 选择

设计 `AiProvider` 抽象，不把系统绑定到某一家 API。第一批适配器：

- OpenAI Responses API：适合强推理、工具调用、长任务和云端模型。
- Ollama：适合本地模型、离线推理、embedding 和隐私敏感任务。
- Generic OpenAI-compatible：适合接入 AtomCode 或其他兼容接口。

### 理由

OpenAI 官方 function calling 文档把模型连接外部系统的工具调用作为标准能力；background mode 支持长任务异步执行。Ollama 官方 embedding 文档说明本地模型可生成向量用于语义搜索和 RAG。

AtlasForge 需要同时支持“强模型处理复杂工程任务”和“本地模型处理隐私敏感索引/摘要”。因此 provider 必须可替换。

### 风险

- 不同模型工具调用格式、上下文长度、流式事件、推理摘要差异大。
- 本地模型对复杂代码修复能力可能不足。

### 缓解

- 统一内部事件协议：`model_started`、`tool_requested`、`tool_result`、`patch_proposed`、`verification_requested`、`final_report`。
- 复杂任务允许 planner 用强模型，局部摘要和 embedding 用本地模型。
- 任务模板声明最低模型能力，例如 `requires_tool_calling`、`requires_large_context`、`requires_background_execution`。

## 决策 5：工具系统兼容 MCP，但内部先做强约束 Tool Broker

### 选择

内部第一版实现自己的 Tool Broker 和权限系统；MCP 作为外部工具接入协议之一。

### 理由

MCP 的定位是让 AI 应用与数据源、工具建立标准连接。它适合接外部能力，例如 GitHub、浏览器、文档系统、数据库。但 AtlasForge 的危险动作包括删除文件、改代码、执行 shell、发布 release，这些动作需要更细的本机策略、审计、审批和回滚。

### 风险

- 过早 MCP 化会让内部权限模型受外部协议牵制。
- 完全自定义会损失生态工具。

### 缓解

- Tool Broker 是唯一执行入口。
- MCP server/client 都只能作为 Tool Broker 后面的 adapter。
- 每个工具声明 risk level、输入 schema、输出 schema、可访问 root、是否可 dry-run、是否可 rollback。

## 决策 6：Git 操作先用系统 git/gh，后续再嵌入 libgit2

### 选择

第一版调用本机 `git` 和 `gh` CLI，封装在 GitAdapter 和 GitHubAdapter 中。

### 理由

Git 和 GitHub 的边界复杂，CLI 行为最接近用户本机真实环境，也能复用用户已有认证。GitHub CLI 官方定位就是把 PR、issue、repo 等 GitHub 能力带到终端；GitHub REST API 可以覆盖 workflow runs、pull requests、releases、checks 等需要结构化查询的场景。

### 风险

- CLI 输出需要稳定解析。
- 用户本机 Git 配置、凭据、代理、换行符和路径编码可能导致差异。
- libgit2 更可控，但实现认证、submodule、worktree、LFS 的成本更高。

### 缓解

- 只解析机器可读输出：`git status --porcelain=v2 -z`、`git branch --format`、`gh api --jq`。
- 对写操作先 dry-run，再执行。
- 保留 adapter 接口，未来可替换为 libgit2/git2。

## 决策 7：长期任务需要 durable job system

### 选择

所有扫描、索引、审查、修复、验证、发布都进入持久化任务系统。

### 理由

AI 工程任务可能持续很久，会失败、重试、被用户暂停、跨进程恢复。只靠前端状态会丢上下文，也无法审计。

### 任务必须记录

- 输入意图。
- 任务模板版本。
- 权限快照。
- 读取文件列表。
- 工具调用列表。
- 文件 diff。
- 命令输出摘要。
- 验证结果。
- 最终报告。

## 决策 8：Playwright 用于 UI 和网页验证

### 选择

对 AtlasForge 自己的 UI 和被维护项目的网页 smoke test，优先使用 Playwright。

### 理由

Playwright 官方定位覆盖现代 Web 测试、自动化和截图，支持 Chromium、Firefox、WebKit。AtlasForge 很多任务需要验证本地网站、GitHub Pages、截图、登录态旁路或视觉检查，浏览器自动化是基础设施。

### 风险

- 浏览器自动化有 flakiness。
- 登录态和用户浏览器数据涉及隐私。

### 缓解

- 默认 headless 独立 profile。
- 只有用户授权时才接入现有 Chrome 会话。
- 用 HTTP smoke、DOM assertion、screenshot 三层验证，不把视觉检查作为唯一证据。

## 近期不做的技术

- 不先做分布式后端。
- 不先做团队账号系统。
- 不先做云同步。
- 不先做自研向量数据库。
- 不让 AI 直接调用任意 shell。
- 不先追求全自动发布所有仓库；先做到可审计半自动。

## 架构保留点

未来可以升级但第一版不强依赖：

- Qdrant：大规模向量搜索、多用户、服务化部署。
- Tantivy：更强本地代码全文检索。
- Tree-sitter：跨语言代码符号、调用关系和结构化 chunk。
- MCP：外部工具生态。
- OpenTelemetry：跨任务 tracing。
- OS keychain：正式凭据存储。
- Tauri updater：桌面客户端自动更新。

