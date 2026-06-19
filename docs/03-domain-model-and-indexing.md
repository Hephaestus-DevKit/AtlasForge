# 领域模型与索引体系

## 设计目标

AtlasForge 的数据模型要支持三类查询：

1. 当前状态：有哪些项目、仓库、任务、问题、发布。
2. 历史证据：某次 AI 做了什么、读了什么、改了什么、如何验证。
3. 语义知识：某个错误以前怎么修、某个项目如何发布、哪些 repo 有类似结构。

## 核心实体

### WorkspaceRoot

用户授权 AtlasForge 访问的根目录。

字段：

- `id`
- `path`
- `label`
- `accessMode`: `read_only | read_write`
- `scanEnabled`
- `includeGlobs`
- `excludeGlobs`
- `createdAt`
- `lastScannedAt`

规则：

- 所有文件访问必须能追溯到一个 root。
- root 可以是只读；只读 root 下禁止写操作和删除操作。
- root 删除后不立即删除历史任务证据，只停止未来扫描。

### ProjectAsset

泛化项目资产，Git repo 只是其中一种。

字段：

- `id`
- `rootId`
- `path`
- `kind`: `git_repo | directory_project | document_collection | artifact_bundle`
- `name`
- `description`
- `primaryLanguage`
- `frameworks`
- `tags`
- `lastObservedAt`

### Repository

Git 仓库画像。

字段：

- `id`
- `assetId`
- `worktreePath`
- `gitDirPath`
- `isBare`
- `isWorktree`
- `defaultBranch`
- `currentBranch`
- `headSha`
- `remoteOriginUrl`
- `remoteCanonical`
- `dirtyState`
- `aheadBehind`
- `lastCommitAt`

### RepoProfile

仓库工程画像，来自扫描器综合判断。

字段：

- `repoId`
- `languages`
- `packageManagers`
- `frameworks`
- `entrypoints`
- `scripts`
- `testCommands`
- `buildCommands`
- `lintCommands`
- `releaseCommands`
- `ciProviders`
- `deploymentTargets`
- `hasReadme`
- `hasLicense`
- `hasChangelog`
- `hasSecurityPolicy`

### RepoHealthSnapshot

某次体检结果。

字段：

- `id`
- `repoId`
- `scanId`
- `score`
- `categoryScores`
- `findings`
- `recommendedTasks`
- `createdAt`

分类：

- `runnable`
- `tests`
- `ci`
- `dependencies`
- `security`
- `docs`
- `release`
- `public_surface`
- `git_hygiene`
- `platform_compat`

### Finding

可追踪问题。

字段：

- `id`
- `repoId`
- `snapshotId`
- `severity`: `info | low | medium | high | critical`
- `category`
- `title`
- `body`
- `evidenceRefs`
- `fixability`: `manual | ai_assisted | auto_safe | auto_risky`
- `status`: `open | ignored | planned | fixed | obsolete`

### Job

所有长任务的根对象。

字段：

- `id`
- `type`
- `status`
- `input`
- `templateId`
- `modelPlan`
- `permissionSnapshot`
- `createdBy`
- `createdAt`
- `updatedAt`
- `completedAt`

### JobEvent

任务证据链。

字段：

- `id`
- `jobId`
- `seq`
- `type`
- `payload`
- `artifactRefs`
- `createdAt`

事件类型：

- `job_created`
- `context_selected`
- `model_called`
- `tool_requested`
- `tool_started`
- `tool_completed`
- `tool_failed`
- `patch_proposed`
- `patch_applied`
- `verification_started`
- `verification_completed`
- `permission_requested`
- `permission_granted`
- `job_failed`
- `job_completed`

### Artifact

任务产生的证据文件或结构化输出。

字段：

- `id`
- `kind`: `log | diff | screenshot | report | command_output | plan | patch | release_asset`
- `storagePath`
- `contentHash`
- `summary`
- `redactionStatus`
- `createdAt`

### PermissionGrant

权限快照。

字段：

- `id`
- `scope`
- `subject`
- `capability`
- `riskLevel`
- `expiresAt`
- `grantedBy`
- `createdAt`

示例：

- `read:C:\Work\repo`
- `write:C:\Work\repo`
- `shell:npm test in repo`
- `github:create_pr owner/repo`
- `github:create_release owner/repo`

### KnowledgeItem

长期记忆条目。

字段：

- `id`
- `scope`: `global | root | repo | task`
- `sourceType`: `manual | task_summary | scan_summary | release_summary | error_resolution`
- `title`
- `body`
- `sourceRefs`
- `confidence`
- `staleness`
- `createdAt`
- `updatedAt`

规则：

- 不能保存 secrets。
- 从任务总结生成的知识要附证据。
- 可标记过期。

## 索引对象

### SourceDocument

可索引源。

字段：

- `id`
- `assetId`
- `repoId`
- `path`
- `kind`
- `mime`
- `size`
- `mtime`
- `contentHash`
- `indexPolicy`
- `lastIndexedAt`

### Chunk

检索最小单位。

字段：

- `id`
- `documentId`
- `chunkerVersion`
- `ordinal`
- `text`
- `tokenEstimate`
- `startByte`
- `endByte`
- `symbolName`
- `language`
- `metadata`
- `contentHash`

### Embedding

向量索引记录。

字段：

- `id`
- `chunkId`
- `provider`
- `modelId`
- `dimension`
- `embeddingHash`
- `indexName`
- `createdAt`

## Chunk 策略

### 文档

- Markdown 按标题层级切分。
- 保留标题路径，例如 `README > Installation > Windows`。
- 小段合并到合理 token 范围。

### 代码

第一版：

- 按文件、函数附近、注释块和空行粗切。
- 保存语言、路径、扩展名。

后续：

- 接入 tree-sitter。
- 按 symbol、class、function、export、test case 切分。
- 建立 symbol reference。

### 配置

- JSON/YAML/TOML/package 配置按顶层 key 切分。
- 保存关键字段结构化索引，例如 scripts、dependencies、workflows。

### 任务日志

- 命令输出只保存摘要和关键错误。
- 原始长日志放 artifact，不全量塞入向量库。

## 检索策略

AtlasForge 查询分三层：

### 1. 结构化过滤

先限制范围：

- root
- repo
- language
- document kind
- modified recently
- task type
- source confidence

### 2. 关键词召回

用 SQLite FTS5：

- 文件名。
- README。
- 配置字段。
- 错误文本。
- 命令名。
- 包名。
- workflow 名。

### 3. 语义召回

用 LanceDB：

- 项目说明。
- 历史错误。
- 维护手册。
- 代码语义片段。
- 用户自然语言意图。

### 4. Rerank

第一版简单加权：

- 同 repo 加分。
- 最近任务加分。
- 标题/路径命中加分。
- 可信知识加分。
- 过期知识降分。

后续可用模型 rerank。

## 增量索引

触发：

- 手动扫描。
- 文件 watcher。
- Git commit/head 变化。
- 任务完成。
- 用户导入文档。

判断：

- path + size + mtime 快速判断。
- hash 确认内容变化。
- chunk hash 判断是否重算 embedding。

队列：

- `scan_queue`
- `chunk_queue`
- `embedding_queue`
- `summary_queue`

## 数据库迁移原则

- 所有 schema 迁移版本化。
- 不破坏旧任务审计。
- 大规模索引重建必须后台执行，不阻塞应用启动。
- 向量索引可以重建，审计日志不能丢。

## 隐私规则

默认不索引：

- `.env`
- `*.pem`
- `*.key`
- `id_rsa*`
- browser profile
- password manager export
- recovery code
- token cache
- 大型二进制
- `.git/objects`
- `node_modules`
- build cache

疑似 secret 的内容：

- 可以记录“发现疑似 secret 风险”。
- 不把原值写入日志、向量库或模型上下文。
- 报告中只显示脱敏片段和文件位置。

