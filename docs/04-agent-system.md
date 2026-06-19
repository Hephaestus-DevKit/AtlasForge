# AI 任务与智能体系统

## 目标

AtlasForge 的 AI 系统不是简单聊天，而是一个可审计的工程执行系统。它要把自然语言意图转成：

1. 上下文检索。
2. 风险判断。
3. 计划生成。
4. 工具调用。
5. 文件修改。
6. 验证。
7. 报告和知识沉淀。

## 核心原则

- 模型不直接拿无限权限。
- 工具调用必须经过 Tool Broker。
- 任务必须有证据链。
- 完成定义必须包含验证，不只是模型回答。
- 高风险动作默认需要用户授权或显式 auto-policy。

## 任务生命周期

```text
Intent
  -> Task Template Selection
  -> Context Retrieval
  -> Risk Classification
  -> Plan Draft
  -> Permission Check
  -> Tool Execution
  -> Patch / Action Proposal
  -> Apply
  -> Verification
  -> Report
  -> Memory Update
```

## 内部角色

这些角色是任务模板中的职责，不一定对应真实多进程 agent。

### Scout

职责：

- 发现上下文。
- 扫描 repo。
- 读取 README、package、CI、release 配置。
- 找出风险和未知项。

输出：

- `ContextPack`
- `Unknowns`
- `RiskHints`

### Architect

职责：

- 把用户意图转成执行计划。
- 判断是否需要分阶段。
- 决定工具和验证策略。

输出：

- `ExecutionPlan`
- `PermissionRequests`
- `AcceptanceCriteria`

### Maintainer

职责：

- 应用代码、配置、文档修复。
- 生成 patch。
- 处理测试失败。

输出：

- `PatchSet`
- `ChangeSummary`

### Verifier

职责：

- 运行测试和验证。
- 解析失败。
- 判断是否达到验收标准。

输出：

- `VerificationReport`
- `ResidualRisks`

### Publisher

职责：

- 处理 GitHub PR、CI、release、Pages、tag。
- 严格遵守发布权限。

输出：

- `PublishPlan`
- `ReleaseEvidence`

### Librarian

职责：

- 把任务结果总结成项目知识。
- 更新维护手册。
- 标记过期知识。

输出：

- `KnowledgeItems`
- `ProjectManualPatch`

## 任务模板

### repo.audit

输入：

- repo path 或 repo id。
- 审查深度：quick/full/release-ready。

动作：

- 扫描工程画像。
- 检查 Git 状态。
- 检查 README/license/CI/test/release/security。
- 生成 findings。
- 生成修复优先级。

默认权限：

- read repo。
- run safe inspection commands。

禁止：

- 默认不改文件。
- 默认不 push。

验收：

- 输出结构化健康报告。
- 每个 finding 有证据和修复建议。

### repo.fix

输入：

- findings 或用户目标。
- 允许修改范围。

动作：

- 读取相关文件。
- 生成 patch。
- 应用 patch。
- 运行验证。

权限：

- write selected repo。
- run declared verification commands。

验收：

- diff 可查看。
- 验证通过或明确说明失败原因。

### repo.public_initial_release

输入：

- repo id。
- 目标发布方式：GitHub release / Pages / package / app installer。

动作：

- 清理公开面。
- 补 README/license/screenshots。
- 补 CI。
- 统一版本号。
- 创建 tag/release。
- 核验线上状态。

权限：

- write repo。
- git commit/tag。
- GitHub release。
- 可能需要 push。

风险：

- 高。

验收：

- 远端仓库状态明确。
- release 可访问。
- CI 状态明确。
- 本地无意外脏改。

### workspace.scan

输入：

- root ids。

动作：

- 发现资产。
- 更新仓库状态。
- 更新索引队列。

权限：

- read selected roots。

验收：

- 资产清单更新。
- 扫描错误可见。

### knowledge.build_manual

输入：

- repo id。

动作：

- 读取项目文件和历史任务。
- 生成项目手册。
- 保存 KnowledgeItem。

验收：

- 手册包含运行、测试、发布、风险、历史坑点。

### ci.fix_failure

输入：

- GitHub PR/check/run。

动作：

- 拉取 CI 日志。
- 定位失败。
- 本地复现。
- 修复。
- 推送。
- 等待 CI。

风险：

- 中到高，取决于是否 push。

验收：

- CI run 状态最终明确。
- 若失败，报告阻塞原因和下一步。

## ContextPack

模型不应读全仓库。每个任务先构造 ContextPack：

```json
{
  "task": "repo.fix",
  "repo": {
    "name": "example",
    "path": "C:/Work/example",
    "language": ["TypeScript"],
    "frameworks": ["React", "Vite"],
    "branch": "main",
    "dirtyState": "clean"
  },
  "files": [
    {
      "path": "package.json",
      "reason": "scripts and dependencies",
      "contentRef": "artifact_..."
    }
  ],
  "findings": [],
  "memory": [],
  "constraints": [
    "do not modify files outside repo",
    "do not push without permission"
  ]
}
```

## Tool Contract

每个工具必须声明：

```json
{
  "name": "git.status",
  "description": "Read repository status",
  "inputSchema": {},
  "outputSchema": {},
  "riskLevel": "low",
  "supportsDryRun": true,
  "requiresPermission": ["repo:read"],
  "redactionPolicy": "standard"
}
```

风险级别：

- `none`：纯计算。
- `low`：只读。
- `medium`：写文件但可回滚。
- `high`：删除、移动、提交、推送、发布。
- `critical`：force push、删除远端资源、公开敏感内容、凭据操作。

## 模型选择策略

### 强推理模型

用于：

- 复杂跨文件修复。
- 发布计划。
- CI 失败根因分析。
- 架构决策。

要求：

- 支持工具调用。
- 支持较长上下文。
- 支持结构化输出。

### 本地模型

用于：

- 文档摘要。
- 非敏感代码摘要。
- embedding。
- 简单分类。
- 离线模式。

要求：

- 不把输出当最终真相，关键动作仍需验证。

### 降级策略

如果强模型不可用：

- 审查任务可继续。
- 自动修复降级为生成计划。
- 发布任务不自动执行。

## 计划格式

模型生成计划必须包含：

- 目标。
- 约束。
- 需要读取的上下文。
- 需要的权限。
- 操作步骤。
- 验证步骤。
- 回滚策略。
- 不确定项。

## Patch 策略

- 修改前记录 baseline hash。
- 小改动用 patch。
- 大规模机械变更可由受控脚本生成，但脚本也要纳入 artifact。
- 应用后立即读取 diff。
- 不允许模型声称修改成功而不检查 diff。

## 自我纠错循环

验证失败时：

1. Verifier 生成失败摘要。
2. Maintainer 读取相关日志和文件。
3. 生成最小修复。
4. 再验证。

限制：

- 默认最多 N 轮，避免无限循环。
- 每轮都要记录为什么改变策略。

## 任务报告格式

最终报告包含：

- 完成状态。
- 修改文件。
- 关键决策。
- 验证命令和结果。
- 未解决风险。
- 后续建议。
- 写入的知识条目。

## 记忆写入规则

可以写：

- 项目运行命令。
- 发布步骤。
- CI 特殊要求。
- 环境坑点。
- 错误和修复方式。
- 用户公开偏好。

不能写：

- token。
- password。
- 私钥。
- 真实身份敏感信息。
- 未脱敏日志。

