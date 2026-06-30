# 🪐 AtlasForge

<div align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%20x64-blue?style=for-the-badge&logo=windows" alt="Platform" />
  <img src="https://img.shields.io/badge/Framework-Tauri%20v2-eceff4?style=for-the-badge&logo=tauri&logoColor=24c8fa" alt="Tauri" />
  <img src="https://img.shields.io/badge/Frontend-React%20%2B%20TypeScript-blue?style=for-the-badge&logo=react&logoColor=61dafb" alt="React" />
  <img src="https://img.shields.io/badge/Backend-Rust-black?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" />
</div>

<br />

**AtlasForge** 是一款**本地优先（Local-First）的个人 AI 软件工程协同平台与多仓库自动维护引擎**。

它旨在成为开发者在本地机器上的“自动驾驶”工程运营系统，通过连接强大的本地或云端 AI 模型，帮助开发者自动进行代码审查、缺陷修复、健康度审计、测试验证以及变更沉淀。

---

## 🗺️ 名字的含义

* **Atlas（星图）**：将你本地与 GitHub 上的海量项目、源码库、文档以及资产状态，绘制成一张高维的、可语义检索、可推理追踪的知识地图。
* **Forge（锻造）**：不只是停留在聊天框中的修改建议，而是将 AI 的分析与方案真正“锻造”成可通过编译、经过自动化验证、具备安全撤销链路的真实交付代码。

---

## 🚀 核心特性

### 1. 📂 多仓库自动发现与画像
* **多核并行扫描**：采用 Rust 作用域线程（Scoped Threads）并行检索指定目录下的全部 Git 仓库，十倍级提升大容量硬盘的项目发现速度。
* **仓库深度画像**：自动识别项目技术栈（Rust, TypeScript, Python 等）、分析项目完整度（README、测试覆盖、CI 配置），自动生成健康审计报告。

### 2. ⚡ 高性能增量索引 (Knowledge base)
* **批量事务提交**：摒弃传统单文件频繁提交数据库的 I/O 瓶颈，在单次 SQLite 事务中批量保存索引，带来 **10x~50x 的写入加速**。
* **智能增量比对**：通过极速的 mtime 内存哈希比对，自动跳过未修改的文件，只有变更文件才会触发 AI 敏感词过滤、分块（Chunking）和数据库写入。
* **多线程并发切片**：并行对海量源码文件进行安全敏感词脱敏与 AST 语义分块。

### 3. 🛡️ 严格的安全与权限模型 (Security-First)
* **零密钥持久化**：所有 AI API 密钥均通过本地环境变量（如 `DEEPSEEK_API_KEY`）临时引用，数据库与配置文件中不保存任何明文 Token。
* **双层沙箱验证**：AI 提出的代码修改补丁（Unified Diff）会首先自动在隔离的 `git worktree` 沙箱中应用，并通过项目原生的编译与测试命令进行验证，验证通过后才会安全应用到你的工作区。
* **一键无损撤销**：对已应用的代码修改进行完整性哈希校验，支持一键无损 Rollback，绝不破坏你的手写代码。

### 4. 🎛️ 多协议 AI 提供商适配
* **4 种原生模式选择**：
  * **Local (Ollama)**：支持完全本地运行的开源模型（如 Llama3、Qwen2.5），数据不出本地。
  * **DeepSeek**：原生适配 DeepSeek 官方 API（支持 `deepseek-chat` / `deepseek-coder`），默认启用官方优化端点。
  * **OpenAI**：支持标准 GPT 家族模型及任意 OpenAI 兼容的第三方中转服务。
  * **Anthropic**：原生适配 Claude 官方 Messages API 协议，并深度兼容支持 `x-api-key` 与 `Bearer` 双头鉴权的中转网关（如 OneAPI、LiteLLM）。
* **动态参数注入 (JSON)**：支持为每个 AI 提供商配置自定义选项（如 `{"temperature": 0.1, "max_tokens": 8192}`），精细化控制生成效果。
* **一键连接性测试**：内建健康检测（Probe），实时反馈延迟（Latency）与可用模型列表。

---

## 🛠️ 快速开始

### 前置要求
为了在本地编译和运行 AtlasForge，你的机器需要具备以下环境：
* **Node.js** ≥ 18
* **Rust Stable** (推荐安装 `x86_64-pc-windows-msvc` 工具链)
* **Visual Studio Build Tools 2022** (勾选 "C++ 桌面开发" 工作负载)

### 从源码运行
```powershell
# 1. 克隆或进入项目目录
cd .\AtlasForge

# 2. 安装前端依赖
npm install

# 3. 启动开发模式（将自动启动前端 Vite 与后端 Tauri 壳）
npm run tauri dev
```

### 生产环境打包
```powershell
# 构建 Windows x64 平台安装包
$env:RUSTUP_TOOLCHAIN="stable-x86_64-pc-windows-msvc"
npm run tauri -- build --target x86_64-pc-windows-msvc
```
构建出的安装程序将位于：`src-tauri/target/x86_64-pc-windows-msvc/release/bundle/`

---

## 📂 架构设计与文档

关于 system 架构、数据库模型与 AI 智能体体系的详细设计，请参阅：
* 📑 [产品定义与目标](docs/00-product-definition.md)
* 📑 [核心架构与技术决策](docs/01-research-and-decisions.md)
* 📑 [数据模型与知识索引体系](docs/03-domain-model-and-indexing.md)
* 📑 [AI 任务与智能体系统设计](docs/04-agent-system.md)
* 📑 [本地安全与权限控制设计](docs/06-security-and-permissions.md)

---

## ⚖️ 核心原则

1. **本地优先**：你的项目代码与核心索引永远保存在本地 SQLite 中，只有执行任务时才会将最小必要上下文发送给指定的 AI 接口。
2. **可验证性**：AI 做出的任何代码修改与命令执行，必须留下明确的日志与哈希防篡改证据。
3. **高并发与顺畅体验**：后端数据库连接池原生支持 **WAL 并发读写**与 5000ms 锁超时机制，背景任务（扫描/索引）运行时，前台 UI 绝不卡顿。
