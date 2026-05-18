# 对象存储（object-storage）

> 统一的云对象存储管理，支持阿里云 OSS 和腾讯云 COS，提供文件浏览、上传、下载、删除功能。

## 功能特性

- 支持阿里云 OSS 和腾讯云 COS 两种云服务商
- 多连接管理：保存多个云存储账号，一键切换
- 文件浏览器：目录式浏览存储桶中的文件和文件夹
- 文件上传：选择本地文件上传到指定路径
- 文件下载：下载对象到本地指定目录
- 文件删除：删除不需要的对象（含二次确认）
- 文件搜索：按文件名过滤对象列表
- 路径导航：面包屑式目录导航，支持返回上级
- 密钥加密存储：AccessKey / SecretKey 加密保存
- 连接信息脱敏：列表接口不返回密钥，编辑时解密显示
- 自动 MIME 识别：上传时根据文件扩展名设置 Content-Type

## 使用方法

### 基本操作

1. **添加连接**：点击「添加连接」，选择服务商（阿里云/腾讯云），填写 AccessKey、SecretKey、Region、Bucket
2. **选择连接**：从下拉菜单选择已保存的连接，自动加载 Bucket 中的文件列表
3. **浏览文件**：点击文件夹进入子目录，面包屑导航可跳转到任意层级
4. **上传文件**：点击「上传文件」按钮，选择本地文件上传到当前目录
5. **下载文件**：点击文件行的「下载」按钮，选择本地保存目录
6. **删除文件**：点击「删除」按钮，确认后删除对象
7. **管理连接**：支持编辑和删除已保存的连接

### 配置项

| 参数 | 说明 | 默认值 |
|------|------|--------|
| provider | 云服务商 | aliyun |
| name | 连接显示名称 | -- |
| access_key | AccessKey ID（加密存储） | -- |
| secret_key | AccessKey Secret（加密存储） | -- |
| region | 区域（如 oss-cn-hangzhou） | oss-cn-hangzhou |
| bucket | 默认存储桶名称 | -- |
| endpoint | 自定义 Endpoint（可选） | 空 |

## 技术实现

### 后端（Rust）

**模块结构**：

```
src/
├── lib.rs       # 插件主入口，handle_call 方法分发
├── crypto.rs    # 密钥加密/解密
├── models.rs    # 数据模型（ConnectionConfig, BucketInfo, ObjectInfo）
├── provider.rs  # ObjectStoreProvider trait + 共享工具函数
├── oss.rs       # 阿里云 OSS 客户端实现
└── cos.rs       # 腾讯云 COS 客户端实现
```

**handle_call 方法列表**：

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `add_connection` | `{ provider, name, access_key, secret_key, region, bucket, endpoint? }` | `{ success, id }` | 新建连接（密钥加密存储） |
| `update_connection` | `{ id, provider, name, ... }` | `{ success }` | 更新连接 |
| `list_connections` | -- | `Vec<ConnectionConfig>` | 获取连接列表（脱敏，不返回密钥） |
| `get_connection` | `{ id }` | `ConnectionConfig` | 获取单个连接详情（含解密密钥） |
| `delete_connection` | `{ id }` | `{ success }` | 删除连接 |
| `list_buckets` | `{ connection_id }` | `Vec<BucketInfo>` | 列出存储桶 |
| `list_objects` | `{ connection_id, bucket, prefix?, delimiter?, max_keys? }` | `{ objects, prefixes }` | 列出对象 |
| `get_object_info` | `{ connection_id, bucket, key }` | `ObjectInfo` | 获取对象元数据 |
| `download_object` | `{ connection_id, bucket, key, file_path }` | `{ success, size }` | 下载对象到本地文件 |
| `upload_object` | `{ connection_id, bucket, key, file_path }` | `{ success }` | 上传本地文件 |
| `delete_object` | `{ connection_id, bucket, key }` | `{ success }` | 删除对象 |

**核心设计**：

- **策略模式**：`ObjectStoreProvider` trait 定义统一接口，`OssClient` 和 `CosClient` 各自实现
- **工厂方法**：`build_provider()` 根据配置中的 `provider` 字段创建对应客户端（`Box<dyn ObjectStoreProvider>`）
- 新增云服务商只需：实现 `ObjectStoreProvider` trait -> 在 `build_provider` 的 match 中添加分支
- API 签名：OSS 使用 HMAC-SHA1 签名（V1 签名），COS 使用 SHA1 签名算法
- XML 解析：OSS 和 COS 的 API 返回 XML 格式，使用逐行解析提取字段
- 使用 `reqwest::blocking::Client` 进行同步 HTTP 请求（无需 async runtime）

**数据存储方式**：
- JSON 文件：`~/.worktools/history/plugins/object-storage.json`
- 存储内容：连接配置列表
- AccessKey 和 SecretKey 加密存储

**依赖的外部库**：

| 库 | 用途 |
|----|------|
| `reqwest` (blocking) | 同步 HTTP 客户端 |
| `hmac` + `sha1` + `sha2` | API 签名 |
| `base64` | 签名编码 |
| `md5` | 上传时 Content-MD5 计算 |
| `quick-xml` | XML 序列化（备用） |
| `url` | URL 编码 |
| `chrono` | 时间处理 |
| `uuid` | 生成连接 ID |
| `dirs` | 系统目录路径 |

### 前端（React + TypeScript）

**组件结构**：

- `App` -- 主组件，管理连接列表、文件浏览、操作交互
- 工具栏：连接选择下拉框、添加/编辑/删除连接按钮、刷新按钮
- 连接表单：新建/编辑连接的表单区域
- 文件浏览面板：面包屑导航 + 文件列表表格 + 上传/下载/删除操作
- 模态框：删除对象确认、删除连接确认

**pluginAPI.call 调用列表**：

| 调用方法 | 用途 |
|----------|------|
| `list_connections` | 加载连接列表 |
| `add_connection` | 新建连接 |
| `update_connection` | 更新连接 |
| `get_connection` | 获取连接详情（编辑时回填） |
| `delete_connection` | 删除连接 |
| `list_objects` | 加载文件列表 |
| `upload_object` | 上传文件 |
| `download_object` | 下载文件 |
| `delete_object` | 删除文件 |

**特殊依赖**：
- 无额外第三方前端依赖

## 开发与调试

```bash
# Rust 检查
cargo check -p object-storage

# 运行测试
cargo test -p object-storage

# 前端开发
cd plugins/object-storage/frontend && npm run dev

# 前端构建
cd plugins/object-storage/frontend && npm run build
```

## 已知限制

- 使用 `reqwest::blocking` 同步 HTTP 请求，大文件上传/下载时可能阻塞插件
- 列出对象最多返回 200 条（前端传参 `max_keys: 200`），大量文件的目录可能显示不全
- 不支持分片上传（大文件上传受限于内存）
- 不支持文件夹创建（只能通过上传带前缀的文件间接创建）
- OSS 签名使用 V1 版本（HMAC-SHA1），不支持 V4 签名
- COS 签名使用 SHA1，不支持 SHA256 签名
- Endpoint 配置为可选，但部分私有云环境必须配置
- 文件大小格式化为 GB 时最多两位小数
