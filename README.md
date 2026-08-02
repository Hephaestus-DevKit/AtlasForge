# AtlasForge

[![CI](https://github.com/Hephaestus-DevKit/AtlasForge/actions/workflows/ci.yml/badge.svg)](https://github.com/Hephaestus-DevKit/AtlasForge/actions/workflows/ci.yml)
[![Pages](https://github.com/Hephaestus-DevKit/AtlasForge/actions/workflows/deploy.yml/badge.svg)](https://github.com/Hephaestus-DevKit/AtlasForge/actions/workflows/deploy.yml)
[![Release](https://img.shields.io/github/v/release/Hephaestus-DevKit/AtlasForge?include_prereleases)](https://github.com/Hephaestus-DevKit/AtlasForge/releases)

AtlasForge 是一个面向 Windows 的本地优先多仓库工程工作台。它使用 Tauri 2、Rust、React 和 SQLite，在可信的本机边界内完成仓库发现、工程画像、健康审计、文本索引、验证命令和受控 AI 修复。

当前版本：`0.1.0`。这是首个 alpha 版本，重点是建立可运行、可审计、可恢复的本地闭环，不代表所有规划能力都已完成。

[打开 Web Demo](https://hephaestus-devkit.github.io/AtlasForge/)

> Web Demo 用于查看界面与浏览器内交互。浏览器无法访问 Tauri 本机后端，因此目录扫描、Git、SQLite、命令执行和本机 AI Provider 等能力需要从源码运行桌面应用。

## 当前能力

| 领域 | 0.1.0 状态 |
| --- | --- |
| 工作区 | 校验 include/exclude glob，发现并画像多个 Git 仓库，有界并行扫描 |
| 审计与索引 | 确定性工程规则、健康快照、增量文本索引、敏感信息脱敏、全文检索 |
| 任务 | 持久化生命周期、进度事件、取消与受限重试、通知和审计记录 |
| 验证 | 从项目清单检测命令；批准后隔离执行，限制输出并记录证据 |
| AI 修复 | Provider 探测、上下文预览、修复计划、单文件补丁提案、批准、验证和哈希保护回滚 |
| GitHub | 读取并缓存 Actions、Pull Request 和 Release 信息；写操作默认关闭 |
| Tool Broker | 仅开放已实现的 `fs.list`、`fs.read`、`git.status`、`git.diff`、`shell.verify` |

完整状态和非目标以 [能力矩阵](docs/13-capability-matrix.md) 为准。GitHub 写操作、自动更新器、正式安装包发布、向量检索和无人值守修改不属于 0.1.0 的已交付范围。

## 安全边界

- UI 只能通过 Tauri IPC 请求操作，不能直接执行 Shell、写文件或修改 GitHub。
- 工作区具有只读/读写访问模式；路径必须通过授权根目录和排除规则校验。
- 仓库控制的验证命令需要一次性、限时且绑定仓库状态的批准。
- 补丁只支持干净工作树中的单文件文本变更，并先在临时 detached worktree 中验证。
- 回滚前会检查基线哈希；文件已有后续修改时拒绝覆盖。
- API 密钥只引用环境变量名，不写入数据库；发送或索引文本前执行敏感信息脱敏。
- GitHub mutation 在具备专用预览与批准界面前保持硬关闭。

这些约束及其威胁模型见 [安全与权限设计](docs/06-security-and-permissions.md) 和 [可信执行 ADR](docs/14-adr-trusted-execution.md)。

## 快速开始

### 环境要求

- Windows 10/11 x64
- Node.js `>= 20.19`
- Rust stable，目标 `x86_64-pc-windows-msvc`
- Visual Studio Build Tools 2022，安装“使用 C++ 的桌面开发”工作负载

### 运行桌面应用

```powershell
git clone https://github.com/Hephaestus-DevKit/AtlasForge.git
cd AtlasForge
npm ci
npm run tauri dev
```

AI Provider 的密钥通过环境变量提供，例如 `OPENAI_API_KEY`、`ANTHROPIC_API_KEY` 或 `DEEPSEEK_API_KEY`；本地 Ollama 不需要云端密钥。应用设置中只保存变量名和非敏感连接参数。

### 运行质量门禁

```powershell
# TypeScript、ESLint、前端测试、构建、Rust 测试和依赖审计
npm run verify-dev

# 额外运行 Playwright 浏览器测试
npm run verify-dev -- -E2E

# 发布前额外构建 Tauri bundle
npm run verify-release -- -E2E -FullBuild
```

也可以分别执行 `npm run typecheck`、`npm run lint`、`npm test`、`npm run test:rust`、`npm run build` 和 `npm run test:e2e`。

## 项目结构

```text
AtlasForge/
├─ src/                         React UI
│  ├─ api/                     类型化 Tauri IPC 适配层
│  ├─ components/              跨页面基础组件
│  ├─ features/                按领域组织的功能组件
│  ├─ pages/                   页面编排与状态连接
│  ├─ types/                   前端共享领域类型
│  └─ utils/                   无 UI 依赖的通用工具
├─ src-tauri/
│  ├─ src/                     Rust 可信核心与领域适配器
│  ├─ migrations/              只追加的 SQLite schema 迁移
│  ├─ capabilities/            Tauri 权限声明
│  └─ tauri.conf.json          桌面构建配置
├─ e2e/                        Playwright 浏览器验收
├─ scripts/                    可重复的发布门禁
├─ docs/                       产品、架构、安全和验收文档
└─ .github/workflows/          Windows CI 与 GitHub Pages
```

分层原则：页面负责交互编排；`src/features` 承载领域 UI；`src/api` 是唯一前端 IPC 边界；Rust `commands` 只做命令编排，扫描器、索引器、验证器、GitHub、AI Provider、权限和工作区逻辑分别放在独立模块中。阻塞型文件系统、Git 和数据库任务不占用 Tauri 异步运行时。

## 数据与维护

- SQLite 数据库位于 Tauri 应用数据目录，不写入源码仓库。
- SQLite 使用 WAL 和 busy timeout；迁移前创建一致性备份，并在启动时执行完整性检查。
- schema 迁移只追加，不修改已发布迁移。
- 新增高风险能力时必须同时提供 preview、approval、audit 和 rollback strategy。
- 代码行为变更应更新测试、[CHANGELOG](CHANGELOG.md) 和对应设计文档。

## 设计文档

- [产品定义](docs/00-product-definition.md)
- [架构与技术决策](docs/01-research-and-decisions.md)
- [系统架构](docs/02-system-architecture.md)
- [领域模型与索引](docs/03-domain-model-and-indexing.md)
- [任务与 Agent 系统](docs/04-agent-system.md)
- [仓库维护引擎](docs/05-repo-maintenance-engine.md)
- [安全与权限](docs/06-security-and-permissions.md)
- [验证与质量门禁](docs/09-validation-and-quality-gates.md)
- [Windows 原生验收](docs/11-windows-native-validation.md)
- [能力矩阵](docs/13-capability-matrix.md)

## 版本策略

AtlasForge 在 `0.x` 阶段遵循快速演进语义：次版本可能调整内部 API、数据库实体和 UI 工作流，但已发布的迁移与安全边界保持向前兼容。发布记录见 [Releases](https://github.com/Hephaestus-DevKit/AtlasForge/releases) 和 [CHANGELOG](CHANGELOG.md)。
