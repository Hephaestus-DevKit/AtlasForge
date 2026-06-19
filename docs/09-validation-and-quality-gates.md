# 验证、质量门与验收标准

## 质量原则

AtlasForge 自己必须成为它想帮助用户达到的样子：

- 可运行。
- 可测试。
- 可发布。
- 可审计。
- 可恢复。

## 基础质量门

每次合并前：

- typecheck。
- lint。
- unit tests。
- migration tests。
- UI smoke。
- build。

示例命令以后由真实项目决定：

```text
pnpm typecheck
pnpm lint
pnpm test
pnpm test:e2e
pnpm build
```

## 数据库验证

必须测试：

- fresh database migration。
- old schema migration。
- rollback 或 backup。
- job event append。
- audit immutability。
- FTS indexing。

验收：

- 新用户启动无错误。
- 旧数据迁移不丢 job history。
- 搜索索引可重建。

## 权限验证

测试场景：

- 未授权路径读取。
- read-only root 写入。
- deny glob 命中。
- high risk tool 触发审批。
- critical action 被默认拒绝。
- tool output redaction。

验收：

- 所有拒绝都有清晰错误。
- 审计日志记录拒绝事件。

## 扫描验证

测试 fixtures：

- clean git repo。
- dirty git repo。
- nested repo。
- worktree。
- no remote repo。
- multiple remotes。
- Node project。
- Python project。
- Rust project。
- docs-only project。
- repo with `.env`。

验收：

- 不重复发现。
- 不索引 secret 文件。
- repo profile 正确。

## AI 验证

测试：

- provider unavailable。
- malformed model output。
- tool call rejected。
- patch outside root。
- prompt injection in README。
- context too large。

验收：

- AI 失败不会破坏本机文件。
- 模型输出不可信时任务进入 failed/needs review。
- prompt injection 不能绕过 broker。

## 修复验证

每个 fixer 必须有：

- dry-run。
- patch snapshot。
- before/after diff。
- verification。
- rollback path。

验收：

- README fixer 不编造已验证事实。
- CI fixer 不扩大权限。
- dependency fixer 跑测试。
- release fixer 发布前确认 remote。

## GitHub 验证

Read：

- auth missing。
- repo not found。
- rate limit。
- workflow no runs。
- release missing。

Write：

- create PR dry-run。
- create PR real fixture repo。
- rerun workflow。
- create draft release。
- upload artifact。

验收：

- 所有 mutation 有 audit。
- 失败时不声称成功。
- release 后可用 API 读回。

## UI 验证

Playwright flows：

- first run add root。
- scan root。
- open repo。
- run audit。
- review findings。
- create task。
- approve permission。
- review diff。
- view final report。

截图验证：

- Dashboard 不空白。
- 表格无文本重叠。
- Permission modal 信息完整。
- Diff view 可读。

## 性能目标

第一版目标不是极限性能，但要避免不可用：

- 大目录扫描不会冻结 UI。
- 长命令可取消。
- 搜索结果在可感知范围内返回。
- 日志输出不会撑爆数据库。

## 发布验收

发布 AtlasForge 自身前：

- clean worktree。
- version consistent。
- build artifacts generated。
- installer smoke。
- release notes。
- GitHub release created。
- download link verified。

## 失败处理标准

任何任务失败都必须给出：

- 失败步骤。
- 错误摘要。
- 已做改动。
- 是否已回滚。
- 证据链接。
- 下一步建议。

不允许：

- “出错了”但没有上下文。
- 测试没跑却说通过。
- release 未核验却说发布完成。

