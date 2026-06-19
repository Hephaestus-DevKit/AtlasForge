# create-layer

根据 AtlasForge 实施分层文档生成分层骨架代码。

## 参数

- `$ARGUMENTS`: 层号，例如 `0`、`1`、`5`

## 指令

1. 读取 `docs/08-implementation-backlog.md`，找到对应 Layer 的交付物和验收标准。
2. 读取 `docs/02-system-architecture.md`，确认该层涉及的模块和边界。
3. 读取 `docs/06-security-and-permissions.md`，确认该层的安全约束。
4. 根据以上信息生成：
   - 该层需要的文件结构和空模块
   - 核心类型定义（TypeScript interface / Rust struct）
   - IPC command 声明（如涉及 Tauri bridge）
   - SQLite migration 文件（如涉及新表）
   - 基础单元测试骨架
5. 输出每个文件的路径和内容，等待用户确认后再写入。

## 规则

- 不跳层：必须前置层已完成或本层不需要前置层的运行时依赖。
- 遵循适配器模式：外部能力通过 adapter 接入，不硬编码。
- 所有新模块必须声明权限需求。
- 生成的代码必须能编译（至少能通过类型检查）。

## 验收

- 生成的文件结构与 `docs/08-implementation-backlog.md` 描述一致。
- 类型定义与 `docs/03-domain-model-and-indexing.md` 实体对齐。
- 不引入规划外的依赖。
