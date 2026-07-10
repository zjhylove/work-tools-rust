# Design System: .wt-pane 叠加面板原语

## 概述

`.wt-pane` 是 Work Tools Platform 设计系统中的布局原语，用于实现"Keep-Alive 叠加面板"模式。

## 设计动机

在传统的标签页或面板切换中，通常使用 `display: none` 来隐藏非活动面板，但这会导致：
- **iframe 重载**：隐藏的 iframe 会被完全卸载，再次显示时需要重新加载
- **状态丢失**：组件状态会被销毁，用户需要重新输入或操作
- **性能成本**：频繁的创建/销毁带来性能开销

`.wt-pane` 通过 `visibility: hidden` + `content-visibility: hidden` 的组合实现了：
- ✅ **状态保持**：组件保持在 DOM 中，状态不丢失
- ✅ **渲染优化**：`content-visibility: hidden` 跳过隐藏面板的渲染，避免布局/样式计算
- ✅ **成本降低**：避免了频繁的创建/销毁开销

## 使用场景

### 适用场景

- **标签页切换**：需要保持每个标签页的滚动位置、表单输入等状态
- **多面板叠加**：多个面板在同一位置切换显示
- **向导步骤**：用户可能返回上一步修改内容
- **iframe 容器**：iframe 重载成本高，需要保持活跃状态

### 不适用场景

- **简单的显示/隐藏**：不需要保持状态的临时内容（如模态框、下拉菜单）
- **内存敏感场景**：大量面板同时保持在内存中可能带来压力
- **一次性内容**：不需要重访的临时内容

## API 参考

### HTML 结构

```html
<!-- 父容器需要 position: relative -->
<div class="parent-container" style="position: relative">
  
  <!-- 面板 1（当前显示） -->
  <div class="wt-pane">
    内容...
  </div>
  
  <!-- 面板 2（隐藏） -->
  <div class="wt-pane wt-pane--hidden">
    内容...
  </div>
  
  <!-- 面板 3（隐藏） -->
  <div class="wt-pane wt-pane--hidden">
    内容...
  </div>
  
</div>
```

### CSS 类

| 类名 | 描述 |
|------|------|
| `.wt-pane` | 面板容器，绝对定位填满父容器 |
| `.wt-pane--hidden` | 隐藏面板，跳过渲染但保持状态 |

### 样式特性

- **定位**：`position: absolute; inset: 0` 填满父容器
- **布局**：`display: flex; flex-direction: column` 纵向弹性布局
- **隐藏**：`visibility: hidden` 不参与命中测试
- **优化**：`content-visibility: hidden` 跳过子树渲染

## 实现示例

### React 示例

```tsx
function TabContainer({ tabs, activeTab }) {
  return (
    <div className="tab-container" style={{ position: 'relative', flex: 1 }}>
      {tabs.map(tab => (
        <div 
          key={tab.id}
          className={`wt-pane ${tab.id === activeTab ? '' : 'wt-pane--hidden'}`}
        >
          {tab.content}
        </div>
      ))}
    </div>
  );
}
```

### 在本项目中的应用

**位置**：`tauri-app/src/App.tsx:227`

```tsx
{visitedPlugins.map((pluginId) => (
  <div
    key={pluginId}
    className={`content-pane${pluginId === selectedPlugin ? "" : " content-pane--hidden"}`}
  >
    <ErrorBoundary>
      <PluginPlaceholder pluginId={pluginId} theme={theme} />
    </ErrorBoundary>
  </div>
))}
```

> **注意**：`content-pane` 是 `.wt-pane` 的语义化别名，专门用于插件内容区域。

## 性能考虑

### 内存成本

- **N 个面板 = N 倍内存**：所有面板都保持在 DOM 中
- **iframe 成本高**：每个 iframe 独立渲染上下文，内存占用较大
- **建议**：限制同时活跃的面板数量（如只保持最近访问的 5-10 个）

### 渲染优化

`content-visibility: hidden` 提供了关键优化：
- **跳过布局计算**：隐藏面板不参与布局
- **跳过样式计算**：隐藏面板的样式不更新
- **跳过绘制**：隐藏面板不产生绘制任务

### 最佳实践

1. **按需创建**：只在首次访问时创建面板（懒加载）
2. **限制数量**：考虑设置最大活跃面板数，超出时销毁最旧的
3. **监控内存**：使用 DevTools 监控内存占用
4. **提供清理**：允许用户手动清理不用的面板

## 浏览器兼容性

| 特性 | Chrome | Firefox | Safari | Edge |
|------|--------|---------|--------|------|
| `position: absolute` | ✅ All | ✅ All | ✅ All | ✅ All |
| `inset: 0` | ✅ 87+ | ✅ 66+ | ✅ 14.1+ | ✅ 87+ |
| `content-visibility` | ✅ 85+ | ✅ 113+ | ✅ 16.4+ | ✅ 85+ |

> **推荐**：本项目支持现代浏览器（Chrome 85+, Firefox 113+, Safari 16.4+），`content-visibility` 兼容性良好。

## 未来扩展

可能的增强方向：

1. **动画支持**：添加面板切换动画
2. **懒加载增强**：集成 IntersectionObserver 实现真正的懒加载
3. **内存管理**：自动销毁长时间未使用的面板
4. **状态持久化**：结合 localStorage 实现跨会话状态恢复

## 相关资源

- [MDN: content-visibility](https://developer.mozilla.org/en-US/docs/Web/CSS/content-visibility)
- [MDN: visibility](https://developer.mozilla.org/en-US/docs/Web/CSS/visibility)
- [CSS Containment](https://www.w3.org/TR/css-contain-2/)

---

**最后更新**：2025-01-10  
**维护者**：Work Tools Platform Team
