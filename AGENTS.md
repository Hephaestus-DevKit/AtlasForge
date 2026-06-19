# AtlasForge AI 工作规则

这些规则供后续负责实现 AtlasForge 的 AI 助手使用。

## 工作方式

- 默认先读 `README.md` 和 `docs/` 下的规划，再改代码。
- 每次实现前先确认当前分层目标、涉及文件和验收标准。
- 不要把规划文档当成死规定；可以改进，但要把原因写进对应文档或变更记录。
- 遇到用户资产、Git 仓库、发布流程、凭据和本机目录时，默认保守处理。
- 不要在仓库里写入真实 token、密码、私钥、身份证号、恢复码等敏感信息。

## 实现原则

- 先做可运行的薄闭环，再拓展复杂能力。
- 每个核心能力都要有测试或可重复的手工验收步骤。
- 高风险能力必须有 dry-run、diff preview、审计日志和 rollback strategy。
- 自动修复逻辑必须把“发现问题”和“应用修改”分开。
- 数据库 schema、任务事件、权限模型优先稳定，UI 可以逐步加厚。

## 代码边界

- 桌面壳、权限和系统调用边界放在本机可信层。
- AI provider、GitHub、Git、索引器、验证器都通过适配器接入。
- 不要让 UI 直接执行 shell、写文件或调用 GitHub mutation。
- 不要把长期任务只放在前端状态里；任务必须可恢复、可审计。

## 文档边界

- 新增架构决策，写入 `docs/01-research-and-decisions.md` 或新增 ADR。
- 新增数据实体，更新 `docs/03-domain-model-and-indexing.md`。
- 新增自动化任务类型，更新 `docs/04-agent-system.md` 和 `docs/05-repo-maintenance-engine.md`。
- 新增安全规则，更新 `docs/06-security-and-permissions.md`。

