# AtlasForge

AtlasForge 是一个本地优先的个人 AI 工作台和多仓库维护平台。

名字含义：

- Atlas：把电脑里的项目、仓库、文档、发布状态、知识资产画成一张可查询、可推理、可追踪的地图。
- Forge：把 AI 的分析、修复、验证、发布动作锻造成真实可交付成果，而不是停在聊天建议。

## 核心定位

AtlasForge 不是普通 todo 工具，也不是单仓库代码助手。它要成为一个长期运行的"个人工程运营系统"：

- 发现本机和 GitHub 上的项目资产。
- 给每个仓库建立健康画像、知识索引和维护历史。
- 用 AI 自动完成审查、修复、测试、发布、文档整理和复盘。
- 把所有高风险动作纳入权限、审计、回滚和验证链路。
- 逐步沉淀成个人知识库、项目手册、发布手册和自动化体系。

## 当前状态：开发中

AtlasForge 目前处于开发阶段，尚未按正式产品发布。当前重点是打通真实能力、权限边界、数据一致性和验证链路。

- 仅支持 Windows x64 平台。
- GitHub 写操作默认关闭，需设置 `ATLASFORGE_ENABLE_GITHUB_WRITE=1` 环境变量才能启用。
- AI 功能需要自行配置 API Provider（支持 Ollama 和 OpenAI 兼容接口）。
- 自动化当前只实现应用运行期间的定时通知。
- 不支持 ARM64、云同步、多用户或后台服务模式。
- UI 尚未经过大量真实数据测试，可能存在布局问题。

## 从源码运行

前置条件：

- Node.js ≥ 18
- Rust stable (x86_64-pc-windows-msvc target)
- Microsoft Visual Studio Build Tools 2022 (C++ 桌面开发工作负载)

```powershell
# 获取源码后进入项目目录
cd .\AtlasForge

# 安装前端依赖
npm install

# 开发模式运行
npm run tauri dev

# 构建安装包 (x64)
$env:RUSTUP_TOOLCHAIN="stable-x86_64-pc-windows-msvc"; npm run tauri -- build --target x86_64-pc-windows-msvc
```

构建产物位于 `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/`。

## 开发

```powershell
# 前端开发服务器（仅 UI，无后端）
npm run dev

# 完整开发模式（Tauri 窗口 + 热重载）
npm run tauri dev

# 类型检查
npm run typecheck

# 代码风格检查
npm run lint

# 运行单元测试
npm test

# 运行 Rust 测试
npm run test:rust

# 运行端到端测试（需先安装 Playwright 浏览器）
npx playwright install chromium
npm run test:e2e

# 生产构建
npm run build
```

## 验证

每个提交应通过以下验证：

```powershell
npm run verify-dev
```

需要浏览器冒烟验证时：

```powershell
npm run verify-dev -- -E2E
```

完整 Tauri 构建验证：

```powershell
$env:RUSTUP_TOOLCHAIN="stable-x86_64-pc-windows-msvc"; npm run tauri -- build --target x86_64-pc-windows-msvc
```

进入打包阶段后再运行：

```powershell
npm run verify-release -- -FullBuild
```

手动冒烟测试清单见 [docs/tauri-smoke-checklist.md](docs/tauri-smoke-checklist.md)。

## 安全模型

AtlasForge 遵循以下安全原则：

1. **前端不直接执行 Shell 或写文件**：所有写操作通过 Tauri IPC 经后端授权。
2. **路径授权**：只能访问用户明确添加的工作区根目录下的文件。
3. **只读模式**：工作区根目录可设为 read_only，阻止所有写操作。
4. **GitHub 写操作默认关闭**：需环境变量 `ATLASFORGE_ENABLE_GITHUB_WRITE=1` 显式启用。
5. **不持久化密钥**：AI Provider 的 API Key 通过环境变量引用（apiKeyRef），不存入数据库。
6. **审计日志**：所有高风险操作（代码修改、GitHub 写入、AI 调用）记录审计事件。
7. **补丁审批**：AI 生成的代码补丁必须经过用户确认才能应用。
8. **回滚路径**：已应用补丁通过反向补丁回退；存在后续冲突时要求人工检查工作树。

## 规划文档

- [产品定义](docs/00-product-definition.md)
- [调研与技术决策](docs/01-research-and-decisions.md)
- [系统架构](docs/02-system-architecture.md)
- [领域模型与索引体系](docs/03-domain-model-and-indexing.md)
- [AI 任务与智能体系统](docs/04-agent-system.md)
- [多仓维护引擎](docs/05-repo-maintenance-engine.md)
- [安全、权限与审计](docs/06-security-and-permissions.md)
- [界面与核心工作流](docs/07-ui-and-workflows.md)
- [实施分层与任务清单](docs/08-implementation-backlog.md)
- [验证、质量门与验收标准](docs/09-validation-and-quality-gates.md)
- [交给 AI 执行的提示词](docs/10-ai-execution-prompts.md)
- [Alpha 产品化执行计划](docs/12-alpha-productization-plan.md)

## 第一性原则

1. 本地优先：用户资产默认留在本机，外部 AI 只拿到完成任务所需的最小上下文。
2. 可验证：每个自动动作都要有证据链，不能只有“模型觉得可以”。
3. 可回滚：代码改动、配置修改、发布动作必须保留恢复路径。
4. 可解释：AI 做了什么、为什么做、用了哪些文件和命令，都要能复盘。
5. 可替换：模型、向量库、GitHub 接入、桌面壳、任务执行器都要保留替换边界。
6. 面向长期维护：先让系统稳定地观察、索引、审查，再逐步扩大自动修改和自动发布能力。

## 不做什么

- 不把它做成只有聊天框的外壳。
- 不默认上传全盘文件给云端模型。
- 不让 AI 在没有权限边界和审计记录的情况下随意执行 shell。
- 不把“自动化”理解成跳过测试、跳过 review、跳过发布校验。
- 不为了炫技引入微服务、分布式队列和复杂云基础设施作为第一版基础。
