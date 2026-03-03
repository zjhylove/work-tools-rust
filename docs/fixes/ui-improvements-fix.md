# 密码管理器 UI 改进和问题修复

## 修复日期
2026-03-03

## 修复的问题

### 1. ✅ 导出按钮无响应

**问题描述**:
点击导出按钮后没有任何响应,无法下载密码备份文件。

**根本原因**:
插件返回的 JSON 格式为 `{ "data": "..." }`,但前端代码期望直接返回字符串。

**修复方案**:
```typescript
// 修复前:
const result = (await window.pluginAPI?.call(...)) as string;
const blob = new Blob([result], { type: "application/json" });

// 修复后:
const result = (await window.pluginAPI?.call(...)) as { data: string };
const blob = new Blob([result.data], { type: "application/json" });
```

**文件**: `plugins/password-manager/frontend/src/App.tsx:369-374`

---

### 2. ✅ 导入文件选择器残留

**问题描述**:
点击导入按钮打开文件选择器后,即使关闭选择器,界面上仍显示 "Choose File no file selected" 字样,点击该文字还能再次唤起文件选择器。

**根本原因**:
动态创建的 `<input type="file">` 元素没有设置隐藏样式,导致它在页面中可见。

**修复方案**:
```typescript
// 修复前:
const input = document.createElement("input");
input.type = "file";
input.accept = "application/json";

// 修复后:
const input = document.createElement("input");
input.type = "file";
input.accept = "application/json";
input.style.position = "absolute";
input.style.left = "-9999px";
input.style.visibility = "hidden";
```

**文件**: `plugins/password-manager/frontend/src/App.tsx:395-400`

---

### 3. ✅ 表单标题横线距离太近

**问题描述**:
新建密码和编辑密码界面,标题下方的横线距离"新建密码"/"编辑密码"文字太近,视觉效果不协调。

**根本原因**:
CSS 中 `.form-header` 的 `padding-bottom` 设置为 24px,`border-bottom` 紧贴着标题,没有足够的留白。

**修复方案**:
```css
/* 修复前: */
.form-header {
  padding: 28px 28px 24px 28px;
  border-bottom: 2px solid var(--accent);
}

/* 修复后: */
.form-header {
  padding: 32px 28px 28px 28px;
  margin-bottom: 8px;
  border-bottom: 2px solid var(--accent);
}
```

**改进点**:
- 增加上边距从 28px → 32px
- 增加下边距从 24px → 28px
- 添加 `margin-bottom: 8px` 在横线下方增加额外空间

**文件**: `plugins/password-manager/frontend/src/App.css:175-181`

---

## 测试验证

### 导出功能测试
1. 创建至少一个密码条目
2. 点击"导出"按钮
3. 验证显示安全警告对话框
4. 点击"确定"继续
5. 验证:
   - ✅ 文件自动下载
   - ✅ 文件名为 `passwords-backup-YYYY-MM-DD.json`
   - ✅ 文件包含正确的 JSON 数据
   - ✅ 显示成功提示

### 导入功能测试
1. 点击"导入"按钮
2. 验证:
   - ✅ 文件选择器打开
   - ✅ 无"Choose File"残留文字
   - ✅ 界面保持干净
3. 选择有效的 JSON 文件
4. 验证预览和导入流程正常

### 表单样式测试
1. 点击"添加密码"按钮
2. 验证:
   - ✅ 标题与横线之间距离协调
   - ✅ 横线与表单字段之间有适当留白
   - ✅ 整体视觉效果美观

---

## 相关文件

### 修改的文件
1. `plugins/password-manager/frontend/src/App.tsx`
   - 修复导出功能的数据解析
   - 修复导入文件选择器的可见性

2. `plugins/password-manager/frontend/src/App.css`
   - 调整表单标题的间距和边距

### 插件包
- `plugins/password-manager/password-manager.wtplugin.zip` (362 KB)

---

## 部署说明

### 开发环境
1. 前端已重新构建
2. 插件已重新打包
3. 需要重新导入插件到应用

### 生产环境
1. 重新构建前端: `npm run build`
2. 重新打包插件: `zip -r password-manager.wtplugin.zip ...`
3. 用户重新导入插件

---

## 技术细节

### 插件 API 返回格式

**export_passwords**:
```json
{
  "data": "JSON_STRING"
}
```

前端需要访问 `result.data` 来获取实际的 JSON 字符串。

### 文件选择器最佳实践

在 Tauri/Web 应用中动态创建文件选择器时:
1. 设置 `position: absolute` 和负 `left` 值隐藏元素
2. 或设置 `visibility: hidden`
3. 使用完成后从 DOM 中移除
4. 不要使用 `display: none`,因为这会阻止元素交互

### CSS 间距设计原则

- 标题上方: 32px (给标题留出呼吸空间)
- 标题下方: 28px (在横线上方留出空间)
- 横线下方: 8px margin (横线与内容分隔)

---

## Git 提交

```bash
# 查看修改
git diff plugins/password-manager/frontend/src/App.tsx
git diff plugins/password-manager/frontend/src/App.css

# 提交
git add plugins/password-manager/
git commit -m "🐛 fix: improve password manager UI and fix export/import issues

- Fix export functionality: correctly parse {data: string} response
- Fix import file picker: hide input element to prevent visual residue
- Improve form styling: increase spacing around form header title
- Rebuild frontend and plugin package"
```

---

## 已知问题
无

## 后续优化建议
1. 考虑使用 Tauri 的文件选择器 API 替代原生 input
2. 添加导出文件的加密选项
3. 改进表单验证的视觉反馈
4. 添加导出/导入进度指示

---

**状态**: ✅ 已修复,待用户测试
**下一步**: 用户重新导入插件并验证功能
