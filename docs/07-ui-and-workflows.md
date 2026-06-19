# 界面与核心工作流

## UI 总原则

AtlasForge 是工程运营工具，界面应当密集、清晰、可扫描。避免营销页、巨大 hero、装饰性卡片。第一屏直接是可用工作台。

## 信息架构

主导航：

- Dashboard
- Assets
- Repositories
- Tasks
- Knowledge
- Automations
- Settings

辅助区域：

- Global command bar。
- Current job drawer。
- Notification center。
- Permission review modal。

## Dashboard

目标：用户打开后立刻知道当前工程资产状态。

模块：

- 授权 roots 数量。
- 发现 repo 数量。
- dirty repo 数量。
- CI failing repo 数量。
- 可发布候选。
- 高风险 finding。
- 正在运行任务。
- 最近完成任务。

交互：

- 点击指标进入过滤后的 repo 列表。
- 一键启动 workspace scan。
- 一键查看推荐任务队列。

## Assets

目标：管理本机扫描范围。

功能：

- 添加 root。
- 设置 read-only/read-write。
- 配置 exclude globs。
- 暂停扫描。
- 查看扫描错误。
- 查看资产树。

必须展示：

- root path。
- access mode。
- last scanned。
- indexed files。
- ignored files。
- errors。

## Repositories

目标：多仓维护主界面。

列表列：

- Name。
- Path。
- Remote。
- Branch。
- Dirty。
- Health。
- CI。
- Release。
- Language。
- Last activity。
- Risk。

过滤：

- dirty。
- no CI。
- no README。
- failing CI。
- publish candidates。
- high risk。
- by root。
- by language。

详情页 tabs：

- Overview。
- Health。
- Files。
- CI。
- Releases。
- Tasks。
- Knowledge。
- Settings。

## Repo Health View

展示：

- 总分。
- 分类分。
- findings。
- evidence。
- recommended fixes。

操作：

- Run audit。
- Generate fix plan。
- Apply selected safe fixes。
- Create task from findings。

## Task Console

目标：让 AI 任务透明运行。

布局：

- 左侧任务列表。
- 中间事件流。
- 右侧上下文、权限、diff、验证结果。

事件流展示：

- 模型调用。
- 工具调用。
- 文件读取。
- patch proposal。
- command run。
- verification。
- report。

不要展示：

- 冗长原始日志默认折叠。
- secret 原文。

## Diff Review

功能：

- 文件列表。
- inline diff。
- risk label。
- apply/reject。
- open file。
- run verification。
- rollback。

对 AI patch：

- 必须能看到修改原因。
- 必须能只应用部分文件。
- 必须能回到任务上下文。

## Permission Review

高风险动作弹窗必须明确：

- 动作类型。
- 目标。
- 风险等级。
- 将执行命令。
- 将访问外部服务。
- 是否可回滚。
- 批准范围和有效期。

按钮：

- Approve once。
- Approve for this job。
- Deny。
- Edit scope。

## Knowledge

目标：把历史任务和项目知识变成可查询资产。

功能：

- 全局搜索。
- repo scoped search。
- 搜索结果带来源引用。
- 项目手册。
- 发布手册。
- 错误解决手册。
- 过期标记。

结果展示：

- 标题。
- 摘要。
- 来源。
- 可信度。
- 最近验证时间。

## Automations

目标：长期维护。

自动化类型：

- 定期扫描 root。
- 定期 repo audit。
- CI failure monitor。
- dependency update suggestion。
- release readiness check。
- knowledge refresh。

配置：

- scope。
- trigger。
- auto policy。
- notification。
- max risk。

默认：

- 自动扫描和报告。
- 不自动 push/release。

## Settings

设置：

- Roots。
- AI providers。
- Local models。
- GitHub integration。
- Permissions。
- Exclude patterns。
- Indexing。
- Data retention。
- Export/import。

## Command Bar

支持自然语言：

- “扫描 D:\Wonderful 下的项目”
- “审查这个 repo 是否能公开”
- “找出所有没有 CI 的仓库”
- “把最近失败的 GitHub Actions 给我排个优先级”
- “给这个项目生成维护手册”

Command Bar 不直接执行高风险动作，只创建任务。

## 空状态

第一次打开：

- 直接让用户添加 root。
- 说明本地优先和权限范围。
- 不做营销式介绍。

没有 repo：

- 显示扫描错误、忽略规则和添加 root 入口。

没有 AI provider：

- 仍可扫描和本地审查。
- AI 任务显示 provider 配置入口。

## 视觉要求

- 信息密度高。
- 表格可排序、过滤、保存视图。
- 状态颜色克制。
- 高风险动作使用明确红/橙标签。
- 操作按钮带图标和 tooltip。
- 长文本不撑破布局。
- 移动端可以只支持查看和审批，不强求完整维护操作。

