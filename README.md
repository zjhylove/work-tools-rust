# 动态库插件架构实施工作树

## ⚡ 快速开始

**在新会话中告诉 Claude:**

```
我正在实施动态库插件架构重构。
执行计划位于: docs/plans/IMPLEMENTATION_PLAN.md
使用 superpowers:executing-plans 技能开始执行。
```

或者更简洁:

```
使用 superpowers:executing-plans 执行 docs/plans/IMPLEMENTATION_PLAN.md
```

## 📂 工作树信息

- **分支**: feature/dynamic-plugin-architecture
- **基础分支**: main (commit: 8268115)
- **工作目录**: `.worktrees/dynamic-plugin-arch/`
- **绝对路径**: `/Users/zj/Project/Rust/work-tools-rust/.worktrees/dynamic-plugin-arch`
- **目标**: 实施动态库插件架构

## 📋 实施计划

### 主要文档

- **[docs/plans/IMPLEMENTATION_PLAN.md](docs/plans/IMPLEMENTATION_PLAN.md)** - 详细实施计划 (442 行)
- **[docs/plans/ARCHITECTURE_DESIGN.md](docs/plans/ARCHITECTURE_DESIGN.md)** - 架构设计文档 (105 行)

### 任务清单

#### Phase 1: 基础设施 (2-3h)
- [ ] Task 1.1: 创建 shared/plugin-api 库 (Plugin trait)
- [ ] Task 1.2: 添加 libloading 依赖

#### Phase 2: 插件管理器重构 (4-5h)
- [ ] Task 2.1: 重写 PluginManager 支持动态库加载
- [ ] Task 2.2: 添加 get_plugin_view Command

#### Phase 3: 前端动态渲染 (3-4h)
- [ ] Task 3.1: 创建 PluginView 组件
- [ ] Task 3.2: 实现插件通信 Bridge (pluginAPI)

#### Phase 4: 插件迁移 (4-6h)
- [ ] Task 4.1: 迁移 password-manager 到动态库
- [ ] Task 4.2: 迁移 auth-plugin 到动态库

#### Phase 5: 清理和优化 (2-3h)
- [ ] Task 5.1: 删除废弃的 JSON-RPC 代码
- [ ] Task 5.2: 增强错误处理和日志

#### Phase 6: 测试和文档 (2-3h)
- [ ] Task 6.1: 编写单元测试
- [ ] Task 6.2: 更新 CLAUDE.md 架构文档

**预计总时间**: 17-24 小时

## ✅ 基线状态

- ✅ 构建成功 (cargo build)
- ✅ 测试通过 (2 tests passed)
- ✅ 工作树已创建
- ✅ 实施计划已就绪
- ✅ 设计文档已就绪

## 🔧 技术栈

- **动态库加载**: libloading 0.8
- **序列化**: serde + serde_json
- **UI 渲染**: Tauri WebView + innerHTML
- **前端**: Solid.js + TypeScript

## 🎯 架构目标

从 **独立进程 IPC** 迁移到 **同进程动态库**:

| 维度 | 当前 | 目标 |
|------|------|------|
| 插件形式 | 可执行文件 | 动态库 |
| 通信方式 | JSON-RPC over stdin/stdout | 直接函数调用 |
| UI 渲染 | 硬编码组件 | 动态 HTML |
| 生命周期 | get_info, get_view | init, get_view, destroy |

## 📊 完成标准

- [ ] 所有现有插件迁移到动态库
- [ ] 单元测试覆盖率 > 70%
- [ ] 三个平台测试通过 (macOS/Linux/Windows)
- [ ] 文档完整更新
- [ ] 性能测试通过 (加载 < 100ms)

## 🔗 相关资源

- [Java 版本实现](../../../../Java/work-tools-platform) - 参考架构
- [主项目 CLAUDE.md](../../../../CLAUDE.md) - 项目说明
- [共享库 shared/](shared/) - plugin-api 等共享代码

---

**准备就绪! 在新会话中开始实施!** 🚀
