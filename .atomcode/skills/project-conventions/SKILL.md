# project-conventions

AtlasForge 项目规范知识。AI 每次工作时自动参考，确保遵循项目边界。

## 项目核心原则

1. **本地优先**：用户资产默认留在本机，外部 AI 只拿最小上下文。
2. **可验证**：每个自动动作都要有证据链。
3. **可回滚**：代码改动、配置修改、发布动作必须保留恢复路径。
4. **可解释**：AI 做了什么、为什么做、用了哪些文件和命令，都要能复盘。
5. **可替换**：模型、向量库、GitHub 接入、桌面壳、任务执行器都要保留替换边界。

## 代码边界

- 桌面壳、权限和系统调用边界放在本机可信层（Tauri/Rust command broker）。
- AI provider、GitHub、Git、索引器、验证器都通过适配器接入。
- **禁止** UI 直接执行 shell、写文件或调用 GitHub mutation。
- **禁止** 模型直接持有未过滤文件系统能力或直接执行 shell。
- **禁止** 模型在没有验证结果时宣称任务完成。
- 不要把长期任务只放在前端状态里；任务必须可恢复、可审计。

## 文档更新义务

- 新增架构决策 → 写入 `docs/01-research-and-decisions.md` 或新增 ADR。
- 新增数据实体 → 更新 `docs/03-domain-model-and-indexing.md`。
- 新增自动化任务类型 → 更新 `docs/04-agent-system.md` 和 `docs/05-repo-maintenance-engine.md`。
- 新增安全规则 → 更新 `docs/06-security-and-permissions.md`。

## 敏感信息

- **禁止** 在仓库里写入真实 token、密码、私钥、身份证号、恢复码。
- 凭据必须走 OS keychain 或环境变量，不落库。

## 技术栈约束

- UI：React + TypeScript + Vite + Tailwind
- 桌面壳：Tauri 2
- 核心层：Rust（权限、审计、命令 broker）
- 服务层：TypeScript worker（扫描、索引、AI 调度）
- 数据库：SQLite + FTS5（第一版）
- 向量索引：LanceDB（第一版）
- AI：provider-neutral，适配 OpenAI + Ollama
- 工具系统：内部 Tool Broker，MCP 仅作为外部 adapter

## 实施分层参考

详见 `docs/08-implementation-backlog.md`，从 Layer 0 开始逐层交付，不跳层。
