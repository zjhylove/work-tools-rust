# 数据库路由（db-router）

> 根据业务编号解析目标数据库和表名，支持自定义 Rhai 脚本、多种哈希算法和一致性哈希。

## 功能特性

- 自定义路由规则：通过 Rhai 脚本编写路由逻辑，灵活适配各种分库分表策略
- 多种哈希算法：FNV-1a、MurmurHash3、BKDR、Java hashCode
- 一致性哈希：基于虚拟节点的分布式一致性哈希，适合节点动态增减场景
- 规则匹配：根据编号长度和前缀自动匹配适用规则
- 规则模板：内置常用路由模式（按位置截取、取模分片、日期分表、一致性哈希分片）
- 多表关联：一条规则可同时解析多个关联表（如订单表 + 订单明细表）
- 规则导入/导出：JSON 文件格式，方便团队共享
- 复制结果：一键复制解析后的数据库名和表名

## 使用方法

### 基本操作

1. **新建规则**：点击「新建规则」按钮，填写规则名称、编号长度、编号前缀、关联表名
2. **编写脚本**：在「解析脚本」区域编写 Rhai 脚本，或从模板加载预置脚本
3. **输入编号**：在「输入编号」区域输入业务编号
4. **执行解析**：匹配的规则卡片会高亮显示，点击播放按钮执行解析
5. **查看结果**：右侧面板显示解析后的数据库名和表名列表，点击可复制

### Rhai 脚本编写

脚本中必须设置两个变量：

- `database`：目标数据库名
- `table_suffix`：表名后缀

**可用内置函数**：

| 分类 | 函数 | 说明 |
|------|------|------|
| 字符串 | `bytes(s)`, `to_upper(s)`, `to_lower(s)`, `trim(s)` | 字符串转换 |
| 字符串 | `split(s, sep)`, `find(s, sub)`, `replace(s, from, to)` | 字符串操作 |
| 字符串 | `substring(s, start, end)`, `contains(s, sub)`, `is_empty(s)` | 子串操作 |
| 字符串 | `starts_with(s, prefix)`, `ends_with(s, suffix)` | 前后缀检查 |
| 字符串 | `pad_left(s, n, ch)`, `pad_right(s, n, ch)`, `repeat(s, n)` | 填充和重复 |
| 数学 | `abs(n)`, `min(a, b)`, `max(a, b)`, `pow(base, exp)` | 基础数学 |
| 数学 | `sqrt(n)`, `floor(n)`, `ceil(n)`, `round(n)` | 高级数学 |
| 哈希 | `hash_code(s)`, `murmur32(s)`, `fnv_hash(s)`, `bkdr_hash(s)` | 哈希算法 |
| 分布式 | `consistent_hash(replicas, nodes, key)` | 一致性哈希 |
| 日期 | `parse_datetime(s, pattern)` | 日期解析（返回 Map） |
| 转换 | `to_int(s)`, `to_float(s)`, `to_string(n)` | 类型转换 |

**脚本示例**：

```rhai
// 按位置截取
let database = "db_order_" + code[3..7];
let table_suffix = "_" + code[15..18];

// 取模分片
let n = code.len();
let shard = (n % 16).to_string();
let database = "db_order_" + shard;
let table_suffix = "_" + shard;

// 一致性哈希
let nodes = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
let sharding = consistent_hash(32, nodes, code);
let database = "0";
let table_suffix = "_" + pad_left(sharding.to_string(), 2, "0");
```

### 配置项

| 参数 | 说明 | 默认值 |
|------|------|--------|
| code_length | 编号长度限制（0 = 不限制） | 0 |
| code_prefix | 编号前缀过滤（逗号分隔多个） | 空 |
| route_script | Rhai 路由脚本（必填） | -- |
| tables | 关联表名列表（每行一个） | 空 |

## 技术实现

### 后端（Rust）

**模块结构**：

```
src/
├── lib.rs      # 插件主入口，handle_call 8 个方法分发
├── model.rs    # 数据模型（RouteRule, RouteResult, RouteData）
└── engine.rs   # Rhai 脚本引擎 + 自定义函数注册 + 哈希算法
```

**handle_call 方法列表**：

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `list_rules` | -- | `RouteData` | 获取所有路由规则 |
| `save_rule` | `{ rule }` | `RouteData` | 保存规则（无 ID = 新建，有 ID = 更新） |
| `delete_rule` | `{ id }` | `RouteData` | 删除规则 |
| `parse_route` | `{ code, rule_id }` | `RouteResult` | 执行路由解析 |
| `match_rules` | `{ code }` | `Vec<&RouteRule>` | 根据编号匹配适用规则 |
| `import_rules` | `{ rules }` | `RouteData` | 批量导入规则 |
| `export_rules` | -- | `Vec<RouteRule>` | 导出所有规则 |
| `get_templates` | -- | `Vec<RouteRule>` | 获取预置模板列表 |

**核心设计**：

- `engine.rs` 是最核心的模块，嵌入 Rhai 脚本引擎实现自定义路由逻辑
- 脚本沙箱安全限制：最多 10 万次操作、禁止加载外部模块、最大调用深度 32
- 哈希算法实现与 Java 版完全一致，确保跨语言路由结果相同
- 一致性哈希使用 `BTreeMap` 实现哈希环，支持 `range` 查找最近节点
- FNV-1a 实现 Avalanche 雪崩混合，保证哈希分布均匀

**数据存储方式**：
- JSON 文件：`~/.worktools/history/plugins/db-router.json`
- 存储内容：路由规则列表
- 使用 `PluginStorage` 进行读写

**依赖的外部库**：

| 库 | 用途 |
|----|------|
| `rhai` | 嵌入式脚本引擎 |
| `chrono` | 日期时间解析（脚本内置函数） |
| `uuid` | 生成规则 ID |
| `serde` / `serde_json` | 序列化 |
| `anyhow` | 错误处理 |

### 前端（React + TypeScript）

**组件结构**：

- `App` -- 主组件，管理规则列表、编号输入、解析结果
- 左面板：规则卡片列表（搜索、匹配高亮、编辑/删除/解析操作）
- 右面板：编号输入框 + 解析结果面板（数据库名、表名列表）
- 模态框：规则新建/编辑表单、删除确认

**pluginAPI.call 调用列表**：

| 调用方法 | 用途 |
|----------|------|
| `list_rules` | 加载规则列表 |
| `save_rule` | 新建或更新规则 |
| `delete_rule` | 删除规则 |
| `parse_route` | 执行路由解析 |
| `match_rules` | 匹配适用规则（前端也有本地匹配逻辑） |
| `import_rules` / `export_rules` | 规则导入导出 |
| `get_templates` | 加载脚本模板 |

**特殊依赖**：
- 无额外第三方前端依赖
- 内联 SVG 图标组件（Icons 对象），不依赖图标库

## 开发与调试

```bash
# Rust 检查
cargo check -p db-router

# 运行测试
cargo test -p db-router

# 前端开发
cd plugins/db-router/frontend && npm run dev

# 前端构建
cd plugins/db-router/frontend && npm run build
```

## 已知限制

- Rhai 脚本不支持并发（单线程执行）
- 哈希算法仅支持 32-bit（与 Java 版保持一致）
- 一致性哈希的虚拟节点数需要在脚本中手动指定
- 日期解析仅支持 `yyyy-MM-dd` 和 `yyyy-MM-dd HH:mm:ss` 两种格式
- 脚本执行超时受 `max_operations` 限制（10 万次），复杂逻辑可能触发
- `code` 变量的字符串切片 `code[3..7]` 使用 Rhai 的原生字符串索引，基于 Unicode 字符而非字节
