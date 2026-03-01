# Password Manager Plugin

本地密码管理器插件,用于安全存储和管理密码。

## 功能

- 保存密码信息(服务名称、用户名、密码)
- 本地加密存储
- 密码搜索和过滤

## 编译

```bash
cd /Users/zj/Project/Rust/work-tools-rust/plugins/password-manager
cargo build --release
```

## 安装

```bash
mkdir -p ~/.worktools/plugins/password-manager
cp target/release/password-manager ~/.worktools/plugins/password-manager/
```

## 测试

```bash
# 测试插件信息
echo '{"jsonrpc":"2.0","method":"get_info","params":{},"id":1}' | \
  ~/.worktools/plugins/password-manager/password-manager

# 测试获取 UI Schema
echo '{"jsonrpc":"2.0","method":"get_view","params":{},"id":2}' | \
  ~/.worktools/plugins/password-manager/password-manager
```
