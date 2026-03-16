# Text-Diff 布局优化方案

## 📊 当前布局问题分析

### 现状
```
工具栏 (水平排列):
[打开左侧] [打开右侧] | [↑ 上一个] [↓ 下一个] | [☑ 忽略空白] [☑ 忽略大小写] | [💾 导出差异] | 统计: ➕0 | ➖0 | ✏️0
```

### 问题
1. ❌ **所有按钮挤在一行** - 过于拥挤
2. ❌ **视觉混乱** - 不同类型按钮混在一起
3. ❌ **没有功能分区** - 用户难以快速找到功能
4. ❌ **统计信息不突出** - 被埋在按钮中间

---

## 🎯 推荐布局方案

### **方案 A: 三层布局 (推荐)**

```
┌─────────────────────────────────────────────────────────┐
│ 📂 文件操作                                             │
│ [打开左侧] [打开右侧]                         [导出差异] │
├─────────────────────────────────────────────────────────┤
│ 🔍 差异导航                                              │
│      [↑ 上一个] [↓ 下一个]                               │
├─────────────────────────────────────────────────────────┤
│ ⚙️ 选项                        📊 统计                 │
│ [☑ 忽略空白] [☑ 忽略大小写]    ➕ 0 | ➖ 0 | ✏️ 0     │
└─────────────────────────────────────────────────────────┘
```

**优点**:
- ✅ 清晰的功能分区
- ✅ 文件操作在顶部 (最常用)
- ✅ 导航和选项分离
- ✅ 统计信息独立区域

**实现**:
```tsx
<div className="toolbar-container">
  {/* 第一行: 文件操作 */}
  <div className="toolbar-row file-operations">
    <button onClick={onOpenLeft}>📂 打开左侧</button>
    <button onClick={onOpenRight}>📂 打开右侧</button>
    <div className="spacer" />
    <button onClick={onExport}>💾 导出差异</button>
  </div>

  {/* 第二行: 差异导航 */}
  <div className="toolbar-row navigation">
    <button onClick={onPreviousDiff}>↑ 上一个</button>
    <button onClick={onNextDiff}>↓ 下一个</button>
  </div>

  {/* 第三行: 选项 + 统计 */}
  <div className="toolbar-row options-stats">
    <div className="options">
      <label className="checkbox-label">
        <input type="checkbox" checked={ignoreWhitespace} onChange={handleWhitespaceChange} />
        忽略空白
      </label>
      <label className="checkbox-label">
        <input type="checkbox" checked={ignoreCase} onChange={handleCaseChange} />
        忽略大小写
      </label>
    </div>
    <div className="stats">
      ➕ {diffStats.additions} | ➖ {diffStats.deletions} | ✏️ {diffStats.modifications}
    </div>
  </div>
</div>
```

---

### **方案 B: 侧边栏布局**

```
┌──────────┬────────────────────────────────────────────┐
│ 📂 文件  │  编辑器区域                              │
│ [打开左] │                                          │
│ [打开右] │                                          │
│          │                                          │
│ 🔍 导航  │                                          │
│ [↑ 上个] │                                          │
│ [↓ 下个] │                                          │
│          │                                          │
│ ⚙️ 选项  │                                          │
│ [忽略空] │                                          │
│ [忽略大] │                                          │
│          │                                          │
│ 💾 导出  │                                          │
│ [导出]  │                                          │
└──────────┴────────────────────────────────────────────┘
```

**优点**:
- ✅ 垂直工具栏,节省横向空间
- ✅ 功能分组清晰
- ✅ 编辑器区域更宽敞

**缺点**:
- ❌ 占用左侧空间
- ❌ 可能需要图标 + 文字

---

### **方案 C: 顶部分组布局 (折中方案)**

```
┌─────────────────────────────────────────────────────────┐
│ [打开] [打开]  [↑ 上一个] [↓ 下一个]  [☑] [☑]  [导出] │
│  左侧    右侧                                        差异  │
│                                                          │
│  📊 差异统计: ➕ 0 | ➖ 0 | ✏️ 0                        │
└─────────────────────────────────────────────────────────┘
```

**优点**:
- ✅ 只占一行,节省垂直空间
- ✅ 所有操作可见
- ✅ 统计信息独立

**实现**:
```tsx
<div className="compact-toolbar">
  <div className="button-group">
    <button onClick={onOpenLeft}>📂 打开左侧</button>
    <button onClick={onOpenRight}>📂 打开右侧</button>
  </div>

  <div className="separator" />

  <div className="button-group">
    <button onClick={onPreviousDiff}>↑</button>
    <button onClick={onNextDiff}>↓</button>
  </div>

  <div className="separator" />

  <div className="button-group">
    <label className="checkbox-label">
      <input type="checkbox" checked={ignoreWhitespace} onChange={handleWhitespaceChange} />
      忽略空白
    </label>
    <label className="checkbox-label">
      <input type="checkbox" checked={ignoreCase} onChange={handleCaseChange} />
      忽略大小写
    </label>
  </div>

  <div className="separator" />

  <button onClick={onExport}>💾</button>
</div>

<div className="stats-bar">
  📊 差异统计: ➕ {diffStats.additions} | ➖ {diffStats.deletions} | ✏️ {diffStats.modifications}
</div>
```

---

## 🎨 推荐实现: 方案 A (三层布局)

### 为什么推荐三层布局?

1. **符合使用流程**:
   - 第一步: 打开文件 (顶行)
   - 第二步: 查看差异 (中间)
   - 第三步: 调整选项 (底行)

2. **清晰的视觉层次**:
   - 主要操作 (文件) 在顶部
   - 次要操作 (导航) 在中间
   - 辅助操作 (选项) 在底部

3. **未来扩展性**:
   - 可以添加更多功能而不拥挤
   - 每层可以独立隐藏/显示

---

## 💡 CSS 实现要点

### 关键样式
```css
.toolbar-container {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  background: white;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.toolbar-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 0;
}

.toolbar-row.file-operations {
  justify-content: space-between;
}

.toolbar-row.navigation {
  justify-content: center;
}

.toolbar-row.options-stats {
  justify-content: space-between;
}

.button-group {
  display: flex;
  gap: 12px;
}

.spacer {
  flex: 1;
}

/* 响应式: 小屏幕时合并为两行 */
@media (max-width: 768px) {
  .toolbar-row.file-operations {
    flex-wrap: wrap;
  }
}
```

---

## 📊 三种方案对比

| 特性 | 方案 A (三层) | 方案 B (侧边栏) | 方案 C (一行) |
|------|-------------|---------------|--------------|
| **空间利用** | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **功能分组** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| **易用性** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| **扩展性** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐ |
| **视觉清晰** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |

---

## 🎯 最终推荐

**如果空间充足**: 方案 A (三层布局)
**如果空间紧张**: 方案 C (一行分组)
**如果功能很多**: 方案 B (侧边栏)

我建议先实现**方案 C (一行分组)**,因为:
- 实现简单
- 节省空间
- 功能分组清晰
- 更符合现有布局

需要我实现吗?
