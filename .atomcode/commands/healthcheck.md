对指定仓库执行 AtlasForge 健康检查清单。

## 参数

- `$ARGUMENTS`（可选）: 仓库路径或项目名称。不填则对当前项目执行。

## 指令

根据 `docs/03-domain-model-and-indexing.md` 中 `RepoHealthSnapshot` 的分类，执行以下检查：

### 可运行性 (runnable)
- [ ] 项目能否成功安装依赖？
- [ ] 项目能否成功构建？
- [ ] 项目能否启动/运行？

### 测试 (tests)
- [ ] 是否有测试脚本？
- [ ] 测试能否通过？

### CI (ci)
- [ ] 是否配置了 CI？
- [ ] CI 是否通过？

### 依赖 (dependencies)
- [ ] 是否有已知漏洞的依赖？
- [ ] 是否有过时的依赖？

### 安全 (security)
- [ ] 是否有 secrets 泄漏风险？
- [ ] GitHub Actions 权限是否最小化？

### 文档 (docs)
- [ ] README 是否存在且完整？
- [ ] 是否有 LICENSE？
- [ ] 是否有贡献说明？

### 发布 (release)
- [ ] 是否有 release workflow？
- [ ] 版本号是否一致？

### 公开面 (public_surface)
- [ ] 是否有截图/demo URL？
- [ ] 安装说明是否完整？

### Git 卫生 (git_hygiene)
- [ ] 是否有未提交改动？
- [ ] 是否有未推送提交？
- [ ] 默认分支是否保护？

### 平台兼容 (platform_compat)
- [ ] Windows 兼容性如何？

## 输出

对每个分类给出 0-100 评分和发现列表，最后汇总为健康报告。
