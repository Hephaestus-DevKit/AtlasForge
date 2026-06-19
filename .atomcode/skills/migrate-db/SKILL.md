# migrate-db

数据库迁移脚手架。创建新的 SQLite 迁移文件，遵循 AtlasForge 的编号和命名规范。

## 用法

```
/migrate-db <migration-name>
```

## 步骤

1. 查看现有迁移文件列表：
   ```
   ls src-tauri/migrations/
   ```

2. 确定下一个序号（3 位数字，从现有最大序号 +1）：
   - 示例：已有 `011_automation.sql` → 下一个是 `012`

3. 创建迁移文件 `src-tauri/migrations/<NNN>_<migration-name>.sql`，包含：
   - `-- Up` 部分：CREATE TABLE / ALTER TABLE / CREATE INDEX 等正向变更
   - `-- Down` 部分：DROP TABLE / DROP INDEX 等回滚变更
   - 表和列注释（中文或英文均可）

4. 迁移文件模板：
   ```sql
   -- Up
   CREATE TABLE IF NOT EXISTS <table_name> (
     id INTEGER PRIMARY KEY AUTOINCREMENT,
     -- columns...
     created_at TEXT NOT NULL DEFAULT (datetime('now')),
     updated_at TEXT NOT NULL DEFAULT (datetime('now'))
   );

   CREATE INDEX IF NOT EXISTS idx_<table_name>_<column> ON <table_name>(<column>);

   -- Down
   DROP INDEX IF EXISTS idx_<table_name>_<column>;
   DROP TABLE IF EXISTS <table_name>;
   ```

5. 提醒用户：
   - 检查是否需要更新 `docs/03-domain-model-and-indexing.md` 中的数据实体文档
   - 检查是否需要更新 Rust 侧的 `models.rs` 和相关 `commands.rs`
   - 在 `db.rs` 中添加 `run_migrations` 调用（如果需要）

## 约定

- 迁移文件命名：`<NNN>_<snake_case_name>.sql`
- 序号从 `001` 开始，3 位数字，不跳号
- 每个迁移只做一件事（单一职责）
- 必须包含 `-- Up` 和 `-- Down` 两部分
- 所有表必须有 `created_at` 和 `updated_at`
- 外键用 `REFERENCES` 声明，不用应用层约束
