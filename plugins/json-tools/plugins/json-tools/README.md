# JSON 工具插件

一个强大的 JSON 编辑和可视化工具,提供格式化、压缩、转义、去转义以及树形视图编辑功能。

## 功能特性

- ✨ **格式化**: 美化 JSON,提高可读性
- 📦 **压缩**: 压缩 JSON,减小文件大小
- 🔒 **转义**: 转义特殊字符,用于字符串嵌入
- 🔑 **去转义**: 还原转义序列
- 📂 **树形视图**: 可视化展示 JSON 结构
- 🗑️ **节点删除**: 选择并删除树形视图中的节点
- ⚡ **实时验证**: 即时检测 JSON 语法错误

## 安装方法

### 方式一: 插件包安装 (推荐)

1. 下载 `json-tools.wtplugin.zip`
2. 打开 Work Tools 应用
3. 点击插件商店按钮 (🧩)
4. 选择插件包文件导入

### 方式二: 手动安装

```bash
# 解压插件包到用户目录
mkdir -p ~/.worktools/plugins/json-tools
unzip json-tools.wtplugin.zip -d ~/.worktools/plugins/json-tools/

# 重启应用
```

## 使用方法

### 基础使用

1. 在左侧编辑器输入或粘贴 JSON
2. 右侧自动显示树形视图
3. 使用工具栏按钮进行各种操作

### 高级功能

#### 节点删除
1. 在右侧树形视图中点击选择节点
2. 点击"删除选中"按钮
3. 节点被删除,左侧编辑器自动更新

#### 展开/折叠
- 点击"全展开": 展开所有节点
- 点击"全折叠": 只保留根节点展开
- 点击节点箭头: 切换单个节点的展开/折叠状态

## 开发

### 环境要求

- Rust 1.70+
- Node.js 18+
- npm 或 yarn

### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/worktools/json-tools.git
cd json-tools

# 构建后端
cargo build --release

# 构建前端
cd frontend
npm install
npm run build

# 打包插件
cd ..
zip -r json-tools.wtplugin.zip \
  manifest.json \
  target/release/libjson_tools.dylib \
  frontend/dist/
```

## 技术栈

- **后端**: Rust + serde_json
- **前端**: React 18 + TypeScript + Vite
- **样式**: CSS3
- **插件系统**: Work Tools Plugin API

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request!

## 作者

Work Tools Team
