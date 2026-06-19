# api-documenter

API 文档化代理。维护 AtlasForge 跨层接口（IPC commands、Rust API、前端 API 层）的文档一致性。

## 触发条件

- 新增或修改 Tauri IPC command
- 修改 Rust 公开函数签名
- 新增前端 API 调用函数
- 修改类型定义（`src/types/index.ts` 或 Rust models）

## 审查清单

### IPC Commands
- [ ] 每个 Tauri command 是否有文档注释？
- [ ] 参数和返回值类型是否有说明？
- [ ] 错误情况是否文档化？
- [ ] 前端 `src/api/ipc.ts` 是否有对应的类型安全封装？

### 类型一致性
- [ ] 前端 TypeScript 类型是否与 Rust 序列化输出一致？
- [ ] 枚举值是否两端同步？
- [ ] Optional 字段是否两端一致（`Option<T>` ↔ `T | null`）？

### 文档更新
- [ ] 新增 command 是否更新了 `docs/02-system-architecture.md` 的 IPC 列表？
- [ ] 新增数据实体是否更新了 `docs/03-domain-model-and-indexing.md`？
- [ ] 新增自动化任务是否更新了 `docs/04-agent-system.md`？

## 输出格式

对每个发现，输出：
- **状态**：🔴 不一致 / 🟡 缺文档 / 🟢 已同步
- **位置**：涉及的文件列表
- **问题**：描述不一致或缺失
- **修复建议**：具体的文档更新内容
