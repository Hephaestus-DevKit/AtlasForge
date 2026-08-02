# 多仓维护引擎

## 目标

多仓维护引擎负责把散落在本机和 GitHub 上的项目变成可观察、可评分、可修复、可发布的资产组合。

它不是只跑 `git status`，而是要回答：

- 这个仓库现在健康吗？
- 能不能在新机器上跑起来？
- 能不能公开展示？
- 能不能发 release？
- 哪些问题 AI 可以自动修？
- 哪些问题必须人工确认？

## 仓库发现

### 本机发现

扫描授权 root：

- 查找 `.git`。
- 识别 Git worktree。
- 识别 nested repo。
- 识别普通项目：`package.json`、`pyproject.toml`、`Cargo.toml`、`pom.xml`、`build.gradle`、`go.mod`、`pubspec.yaml`。
- 识别静态站点、文档站、Android、桌面应用。
- include glob 按 root 相对路径或仓库目录名筛选；exclude glob 优先并可剪枝目录遍历。
- 仓库元数据分析采用有界工作池，线程数不超过 CPU 可用并行度、任务数和实现上限（当前为 8）。

### 远端关联

从 `origin` 推断：

- GitHub owner/repo。
- 默认分支。
- PR 状态。
- Actions 状态。
- releases。
- Pages。

规则：

- 不自动改 remote。
- remote 不明确时标记 `remote_uncertain`。
- 多 remote 时记录全部，并要求发布前确认 canonical remote。

## 仓库画像

画像字段：

- 项目类型。
- 技术栈。
- 包管理器。
- 运行命令。
- 测试命令。
- 构建命令。
- 发布命令。
- CI provider。
- 依赖管理策略。
- 文档状态。
- License。
- 公开面风险。
- 最近活跃度。

## 健康评分

评分不用于虚假精确，而用于排序和比较。每项 0-5：

### Runnable

检查：

- 是否有安装说明。
- 是否能识别依赖管理器。
- 是否有 dev/build/test 脚本。
- lockfile 是否存在且匹配。
- 是否存在明显缺失配置。

### Tests

检查：

- 是否有测试框架。
- 是否有测试命令。
- 测试是否可本地运行。
- 是否覆盖核心逻辑。

### CI

检查：

- 是否有 GitHub Actions。
- 是否有 lint/test/build。
- 是否 pin action 或至少有 Dependabot 策略。
- workflow permissions 是否最小化。
- 是否支持主要平台。

### Dependencies

检查：

- lockfile drift。
- 过期依赖。
- deprecated package。
- 安全 advisory。
- package manager 版本漂移。

### Security

检查：

- secrets 风险。
- `.env` 是否误提交。
- workflow 权限过大。
- 脚本是否下载执行远端代码。
- release artifact 是否可追溯。

### Docs

检查：

- README 是否说明定位、安装、运行、测试、发布。
- screenshots/demo 是否存在。
- API/CLI 用法是否清晰。
- 维护说明是否存在。

### Release

检查：

- 版本源是否唯一。
- tag/release 是否一致。
- changelog 是否可生成。
- 构建产物是否可复现。
- release workflow 是否存在。

### Public Surface

检查：

- 是否有内部计划、私密路径、调试日志。
- README 是否面向外部用户。
- repo 名、描述、topics 是否清楚。
- license 是否允许公开。

### Git Hygiene

检查：

- dirty worktree。
- untracked 文件。
- 未推送 commit。
- branch 偏离。
- 大文件。
- CRLF/LF 风险。

### Platform Compatibility

检查：

- Windows 路径和 shell 脚本。
- Node/Python/Java/Rust 版本。
- 文件名大小写。
- 本地服务端口冲突。
- Android/iOS/desktop 特定要求。

## Findings

Finding 必须是可执行问题，不写空泛建议。

示例：

```json
{
  "severity": "medium",
  "category": "ci",
  "title": "GitHub Actions workflow grants broad write permissions",
  "evidence": [".github/workflows/release.yml"],
  "impact": "A compromised dependency step could write repository contents or releases.",
  "recommendedFix": "Set top-level permissions to contents: read and grant write only in release job.",
  "fixability": "ai_assisted"
}
```

## 修复器

### README Fixer

能力：

- 生成项目定位。
- 补安装、运行、测试、构建、发布。
- 补 screenshots 占位或引用。
- 补维护注意事项。

限制：

- 不编造不存在的功能。
- 不声称 CI/release 已通过，除非有证据。

### CI Fixer

能力：

- 补 GitHub Actions lint/test/build。
- 增加 cache。
- 设置 permissions。
- 设置 matrix。
- 增加 artifact 上传。

限制：

- 不默认加入秘密变量。
- 不默认开启发布。

### Release Fixer

能力：

- 统一版本号。
- 补 changelog。
- 补 tag/release 流程。
- 生成 release notes。

限制：

- push/tag/release 都是高风险。
- 发布前必须确认 remote 和分支。

### Security Fixer

能力：

- 加 `.gitignore`。
- 移除误入仓库的构建产物。
- 收紧 workflow permissions。
- 添加 secret scan workflow。

限制：

- 已泄漏 secret 不能只删除文件，要提示用户轮换凭据。

### Dependency Fixer

能力：

- 小版本升级。
- lockfile 刷新。
- Dependabot 配置。
- npm/pnpm/cargo/gradle audit。

限制：

- major upgrade 默认需要人工确认。
- 依赖更新必须跑测试。

## 发布工作流

发布前检查：

- canonical remote。
- 当前 branch。
- dirty state。
- 版本号。
- changelog。
- CI 状态。
- release artifact。
- secrets 风险。
- 是否需要签名。

发布动作：

1. 创建 release branch 或确认当前 branch。
2. 应用发布准备改动。
3. 运行本地验证。
4. commit。
5. push。
6. 等待 CI。
7. tag。
8. 创建 GitHub Release。
9. 上传 artifact。
10. 核验 release 页面/API。
11. 写入发布记忆。

每一步都可失败并生成恢复建议。

## 多仓排序策略

当用户说“全面推进”时，排序依据：

1. 用户显式指定。
2. 最近活跃项目。
3. dirty/未发布但可修复项目。
4. 公开展示价值高的项目。
5. 修复成本低但收益高的问题。
6. 高风险安全问题。

默认不碰：

- 未授权 root。
- 无法判断用途的目录。
- 明显是缓存、依赖、构建产物的目录。
- 存在大量未提交改动且用户未确认归属的 repo。

## GitHub 集成

读取：

- repo metadata。
- branches。
- PR。
- issues。
- workflow runs。
- check runs。
- releases。
- Pages。

写入：

- create PR。
- comment PR。
- rerun failed workflow。
- create release。
- upload assets。

写操作都必须带权限和审计。

## 本地命令策略

命令分级：

- Safe read：`git status`、`git log`、`npm -v`。
- Project verify：`npm test`、`pnpm build`、`cargo test`。
- Project mutate：`npm install`、`pnpm update`。
- Git mutate：`git commit`、`git tag`、`git push`。
- Destructive：`rm -rf`、force push、delete release。

策略：

- Safe read 可自动。
- Verify 可在授权 repo 内自动。
- Mutate 需要任务权限。
- Destructive 默认人工确认。

## 输出报告

多仓报告包含：

- 资产总览。
- 健康排名。
- 高风险问题。
- 快速修复候选。
- 需要用户决策的问题。
- 已完成任务。
- 验证证据。
- 建议下一批任务。

