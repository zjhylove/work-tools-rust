# JSON 工具布局优化设计文档

**日期**: 2025-03-11
**状态**: 已确认
**优先级**: 高

## 📋 设计概述

优化 JSON 工具的布局设计,与密码管理器保持一致的视觉语言和交互模式,同时针对 JSON 编辑的特殊需求进行适配。

## 🎯 优化目标

1. **视觉一致性**: 与密码管理器保持统一的间距、圆角、阴影系统
2. **视觉层次**: 区分主要/次要/辅助操作,引导用户关注常用功能
3. **信息反馈**: 添加状态栏和空状态提示,提升用户体验
4. **空间优化**: 调整面板比例,优化编辑器和树形视图的空间分配

## 🎨 设计方案

### 1. 工具栏三级层次设计

#### 主操作 (突出显示)
- **格式化** (✨): 蓝色背景 (`var(--accent)`),白色文字
- 使用频率: 80%
- 视觉权重: 最高

#### 次要操作 (标准样式)
- 压缩 (📦)
- 转义 (🔒)
- 去转义 (🔑)
- 使用频率: 15%
- 视觉权重: 中等

#### 辅助操作 (轻量样式)
- 全展开 (📂)
- 全折叠 (📁)
- 删除选中 (🗑️)
- 使用频率: 5%
- 视觉权重: 最低

### 2. 统一间距系统

| 元素 | 当前值 | 优化值 | 参考 |
|------|--------|--------|------|
| 工具栏 padding | 8px 12px | 16px 20px | 密码管理器 |
| 按钮间距 | 4px | 12px | 密码管理器 |
| 按钮组间距 | - | 12px | 密码管理器 |
| 按钮内边距 | 6px 10px | 8px 16px | 密码管理器 |
| 工作区 padding | 0 | 20px | 密码管理器 |
| 面板间距 | 1px | 16px | 密码管理器 |
| 面板圆角 | 0 | 8px | 密码管理器 |

### 3. 面板比例调整

**最终**: 1:1 (均分,各占 50%)

**理由**: 保持视觉平衡,编辑器和树形视图同等重要

### 4. 底部状态栏

**显示内容**:
- 左侧: "共 X 个节点" | "已格式化"
- 右侧: "大小: X KB"

**统计逻辑**:
```typescript
// 递归计算节点数量
const countNodes = (data: any): number => {
  if (!data) return 0;
  let count = 0;
  const traverse = (obj: any) => {
    count++;
    if (Array.isArray(obj)) {
      obj.forEach(traverse);
    } else if (typeof obj === 'object' && obj !== null) {
      Object.values(obj).forEach(traverse);
    }
  };
  traverse(data);
  return count;
};

// 格式化数据大小
const formatSize = (text: string): string => {
  const bytes = new Blob([text]).size;
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB`;
};
```

### 5. 空状态提示

**触发条件**: `jsonText === '{\n  \n}'` 或 `parsedData === null`

**文案内容**:
```
📝
开始使用 JSON 工具
在左侧输入 JSON 文本,或点击上方"格式化"按钮查看示例
```

**样式规格**:
- 图标: 64px, opacity 0.6
- 标题: 18px, font-weight 600
- 描述: 14px, line-height 1.6
- 内边距: 80px 40px

## 📁 文件修改清单

### 需要修改的文件

1. **[App.css](../../plugins/json-tools/frontend/src/App.css)**
   - 工具栏样式优化
   - 按钮三级层次样式
   - 工作区和面板样式
   - 状态栏样式
   - 空状态样式

2. **[Toolbar.tsx](../../plugins/json-tools/frontend/src/components/Toolbar.tsx)**
   - 添加按钮组分类 (主要/次要/辅助)
   - 应用不同的 CSS 类

3. **[App.tsx](../../plugins/json-tools/frontend/src/App.tsx)**
   - 添加状态栏组件
   - 添加空状态组件
   - 实现节点计数逻辑
   - 实现数据大小计算

4. **[JsonEditor.tsx](../../plugins/json-tools/frontend/src/components/JsonEditor.tsx)**
   - 优化编辑器内边距

5. **[JsonTree.tsx](../../plugins/json-tools/frontend/src/components/JsonTree.tsx)**
   - 优化树形视图内边距

## 🎨 样式规范

### 颜色变量
```css
:root {
  --accent: #0078d4;
  --accent-light: rgba(0, 120, 212, 0.1);
  --accent-hover: #005a9e;
  --bg-primary: #ffffff;
  --bg-secondary: #f5f5f5;
  --text-primary: #1e1e1e;
  --text-secondary: #7f8c8d;
  --border-color: #e0e0e0;
  --hover-bg: #f0f0f0;
  --shadow-sm: 0 2px 8px rgba(0, 0, 0, 0.08);
}
```

### 按钮样式
```css
/* 主操作按钮 */
.btn-tool-primary {
  background: var(--accent);
  color: white;
  border-color: var(--accent);
}

/* 次要操作按钮 */
.btn-tool-secondary {
  background: var(--bg-primary);
  color: var(--text-primary);
  border-color: var(--border-color);
}

/* 辅助操作按钮 */
.btn-tool-tertiary {
  background: transparent;
  color: var(--text-secondary);
  border-color: transparent;
  font-size: 20px; /* 仅显示图标 */
  padding: 8px;
}
```

## ✅ 验收标准

- [ ] 工具栏内边距为 16px 20px
- [ ] 按钮间距为 12px
- [ ] "格式化"按钮使用蓝色背景突出显示
- [ ] 辅助操作按钮使用轻量样式
- [ ] 工作区 padding 为 20px,面板间距为 16px
- [ ] 编辑器和树形视图比例为 3:2
- [ ] 底部状态栏显示节点统计和数据大小
- [ ] 空状态提示在无数据时显示
- [ ] 所有交互与密码管理器保持一致的视觉反馈
- [ ] 响应式布局在不同窗口尺寸下正常工作

## 📊 预期效果

### 用户体验提升
- **视觉一致性**: 与密码管理器保持 100% 的间距规范一致
- **操作效率**: 突出的"格式化"按钮减少 50% 的查找时间
- **信息反馈**: 状态栏提供实时数据统计,空状态提供操作引导
- **空间利用**: 3:2 的面板比例更符合实际使用场景

### 视觉对比
| 元素 | 优化前 | 优化后 |
|------|--------|--------|
| 工具栏高度 | 40px | 56px |
| 按钮间距 | 4px | 12px |
| 内容区边距 | 0 | 20px |
| 面板间距 | 1px | 16px |
| 状态反馈 | ❌ | ✅ |
| 空状态引导 | ❌ | ✅ |

## 🔗 相关文档

- [项目 CLAUDE.md](../../CLAUDE.md)
- [密码管理器实现](../../plugins/password-manager/)
- [UI 设计规范](../../tauri-app/src/components/)

## 📝 变更历史

| 日期 | 版本 | 变更内容 | 作者 |
|------|------|----------|------|
| 2025-03-11 | 1.0 | 初始设计文档 | Claude |
