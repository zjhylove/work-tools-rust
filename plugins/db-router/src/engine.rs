//! # 路由脚本引擎
//!
//! 嵌入 Rhai 脚本语言，允许用户编写自定义路由逻辑。
//! 这是整个项目中最能体现 Rust 强大扩展能力的模块。
//!
//! ## 什么是 Rhai？
//! Rhai 是一个轻量级的、嵌入 Rust 的脚本语言。类似于：
//! - Lua 之于 C/C++
//! - JavaScript 之于 Java (Nashorn)
//!
//! Rhai 的特点：
//! - **安全**: 沙箱执行，默认不能访问文件系统/网络
//! - **简单**: 语法类似 Rust/JavaScript
//! - **可扩展**: 可以注册 Rust 函数让脚本调用
//!
//! ## 示例脚本
//! ```rhai
//! let database = "db_order_" + code[3..7];
//! let table_suffix = "_" + code[15..18];
//! ```
//!
//! ## Rust 知识点
//! - `Engine::register_fn`: 注册 Rust 函数到脚本引擎
//! - `Scope`: 脚本的变量作用域
//! - `BTreeMap`: 有序的键值对（用于一致性哈希环）
//! - 位运算: `wrapping_mul`, `rotate_left`, 溢出安全
//! - FNV-1a 哈希: 简单高效的字符串哈希算法

use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use chrono::{Datelike, Timelike};
use rhai::{Engine, Scope};

use super::model::RouteRule;

/// FNV-1a 32-bit 哈希算法
///
/// FNV-1a 是一种简单但高效的哈希算法，广泛用于哈希表。
/// 这个实现与 Java 版 `HashUtil.fnvHash` 完全一致，
/// 确保相同的输入产生相同的哈希值。
///
/// ## 算法步骤
/// 1. 初始化哈希值为 FNV offset basis (2166136261)
/// 2. 对每个字节: hash = (hash XOR byte) × FNV prime (16777619)
/// 3. 应用 avalanche (雪崩) 混合使分布更均匀
///
/// ## Rust 知识点: 位运算
/// - `wrapping_mul`: 环绕乘法（溢出时回绕，不 panic）
/// - `wrapping_add`: 环绕加法
/// - `^`: 按位异或 (XOR)
/// - `<<` / `>>`: 左/右移位
/// - `hash.abs()`: 返回绝对值
fn fnv1a_hash(s: &str) -> i32 {
    let p: i32 = 16777619; // FNV prime
    let mut hash: i32 = 2166136261_u32 as i32; // FNV offset basis
    for &b in s.as_bytes() {
        hash = (hash ^ b as i32).wrapping_mul(p);
    }
    // Avalanche 混合 — 使哈希值的每一位都依赖于多个输入位
    hash = hash.wrapping_add(hash << 13);
    hash ^= hash >> 7;
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 17;
    hash = hash.wrapping_add(hash << 5);
    // 取绝对值确保非负（因为需要用于 HashMap 索引）
    hash.abs()
}

/// 向 Rhai 引擎注册所有自定义函数
///
/// ## Rust 知识点: 泛型函数注册
/// `engine.register_fn("function_name", |params| body)` 将 Rust 闭包注册为脚本函数。
/// Rhai 通过函数签名自动推断参数类型，调用时自动类型转换。
fn register_functions(engine: &mut Engine) {
    // ── 字符串方法 ───────────────────────────────────
    // bytes(): 将字符串按字节迭代，返回 Vec<i64>
    // `as i64` 转换：Rhai 没有 u8 类型，所以用 i64 表示字节值
    engine.register_fn("bytes", |s: &str| {
        s.as_bytes().iter().map(|&b| b as i64).collect::<Vec<i64>>()
    });
    engine.register_fn("to_upper", |s: &str| s.to_uppercase());
    engine.register_fn("to_lower", |s: &str| s.to_lowercase());
    engine.register_fn("trim", |s: &str| s.trim().to_string());
    engine.register_fn("trim_start", |s: &str| s.trim_start().to_string());
    engine.register_fn("trim_end", |s: &str| s.trim_end().to_string());
    engine.register_fn("contains", |s: &str, sub: &str| s.contains(sub));
    engine.register_fn("starts_with", |s: &str, prefix: &str| s.starts_with(prefix));
    engine.register_fn("ends_with", |s: &str, suffix: &str| s.ends_with(suffix));
    engine.register_fn("replace", |s: &str, from: &str, to: &str| s.replace(from, to));

    // split(): 字符串分割，返回 Vec<String>
    engine.register_fn("split", |s: &str, sep: &str| -> Vec<String> {
        s.split(sep).map(String::from).collect()
    });

    // find(): 查找子串位置，未找到返回 -1
    engine.register_fn("find", |s: &str, sub: &str| -> i64 {
        s.find(sub).map(|i| i as i64).unwrap_or(-1)
    });

    // repeat(): 重复字符串 n 次
    engine.register_fn("repeat", |s: &str, n: i64| s.repeat(n as usize));

    // pad_left() / pad_right(): 字符串填充
    engine.register_fn("pad_left", |s: &str, n: i64, ch: &str| {
        let fill = ch.chars().next().unwrap_or(' ');
        let width = n as usize;
        if s.len() >= width {
            s.to_string()
        } else {
            // `std::iter::repeat(fill).take(n)` 生成 n 个填充字符
            std::iter::repeat(fill).take(width - s.len()).collect::<String>() + s
        }
    });
    engine.register_fn("pad_right", |s: &str, n: i64, ch: &str| {
        let fill = ch.chars().next().unwrap_or(' ');
        let width = n as usize;
        if s.len() >= width {
            s.to_string()
        } else {
            s.to_string() + std::iter::repeat(fill).take(width - s.len()).collect::<String>().as_str()
        }
    });

    // substring(): 提取子串 [start, end)
    engine.register_fn("substring", |s: &str, start: i64, end: i64| -> String {
        let start = start.max(0) as usize;
        let end = end.max(start as i64) as usize;
        s.get(start..end.min(s.len())).unwrap_or("").to_string()
    });

    engine.register_fn("is_empty", |s: &str| s.is_empty());

    // ── 数学函数 ─────────────────────────────────────
    engine.register_fn("abs", |n: i64| n.abs());
    engine.register_fn("min", |a: i64, b: i64| a.min(b));
    engine.register_fn("max", |a: i64, b: i64| a.max(b));
    engine.register_fn("pow", |base: i64, exp: i64| -> i64 {
        base.saturating_pow(exp as u32) // saturating: 溢出时返回最大值，而非 panic
    });
    // `as f64` 将整数转为浮点数进行计算
    engine.register_fn("sqrt", |n: i64| -> i64 { (n as f64).sqrt() as i64 });
    engine.register_fn("floor", |n: f64| n.floor() as i64);
    engine.register_fn("ceil", |n: f64| n.ceil() as i64);
    engine.register_fn("round", |n: f64| n.round() as i64);

    // ── 类型转换 ─────────────────────────────────────
    engine.register_fn("to_int", |s: &str| -> i64 {
        s.trim().parse().unwrap_or(0) // 解析失败返回 0（优雅降级）
    });
    engine.register_fn("to_float", |s: &str| -> f64 {
        s.trim().parse().unwrap_or(0.0)
    });
    engine.register_fn("to_string", |n: i64| n.to_string());
    engine.register_fn("to_string", |n: f64| n.to_string());
    engine.register_fn("to_string", |b: bool| b.to_string());

    // ── 哈希函数 ─────────────────────────────────────

    // Java 兼容的简单 hashCode
    engine.register_fn("hash_code", |s: &str| -> i64 {
        let mut h: i64 = 0;
        for &b in s.as_bytes() {
            h = h.wrapping_mul(31).wrapping_add(b as i64);
        }
        h
    });

    // MurmurHash3 32-bit（与 Java 版一致）
    // 这是分布式系统中最常见的哈希算法之一
    engine.register_fn("murmur32", |s: &str| -> i64 {
        let bytes = s.as_bytes();
        let c1: u32 = 0xcc9e2d51;
        let c2: u32 = 0x1b873593;
        let mut h1: u32 = 0;
        let n = bytes.len();
        let rounded_end = n & !3; // 向下对齐到 4 的倍数

        // 按 4 字节一组处理
        let mut i = 0;
        while i < rounded_end {
            // `u32::from_le_bytes` 用 4 字节构建 u32（小端序）
            let mut k1 = u32::from_le_bytes([bytes[i], bytes[i+1], bytes[i+2], bytes[i+3]]);
            k1 = k1.wrapping_mul(c1);
            k1 = k1.rotate_left(15); // 循环左移
            k1 = k1.wrapping_mul(c2);
            h1 ^= k1;
            h1 = h1.rotate_left(13);
            h1 = h1.wrapping_mul(5).wrapping_add(0xe6546b64);
            i += 4;
        }

        // 处理尾部不足 4 字节的剩余部分
        let mut k1: u32 = 0;
        let tail = n & 3;
        if tail >= 3 { k1 ^= (bytes[rounded_end + 2] as u32) << 16; }
        if tail >= 2 { k1 ^= (bytes[rounded_end + 1] as u32) << 8; }
        if tail >= 1 {
            k1 ^= bytes[rounded_end] as u32;
            k1 = k1.wrapping_mul(c1);
            k1 = k1.rotate_left(15);
            k1 = k1.wrapping_mul(c2);
            h1 ^= k1;
        }

        // 最终混合
        h1 ^= n as u32;
        h1 ^= h1 >> 16;
        h1 = h1.wrapping_mul(0x85ebca6b);
        h1 ^= h1 >> 13;
        h1 = h1.wrapping_mul(0xc2b2ae35);
        h1 ^= h1 >> 16;
        // i32 → i64 绝对值和 JS 的 Math.abs 行为一致
        (h1 as i32 as i64).abs()
    });

    // FNV-1a 哈希注册
    engine.register_fn("fnv_hash", |s: &str| -> i64 { fnv1a_hash(s) as i64 });

    // BKDR 哈希（简单高效的字符串哈希）
    engine.register_fn("bkdr_hash", |s: &str| -> i64 {
        let seed: i32 = 131;
        let mut hash: i32 = 0;
        for &b in s.as_bytes() {
            hash = hash.wrapping_mul(seed) + b as i32;
        }
        (hash & 0x7FFFFFFF) as i64 // 只保留低 31 位（正数）
    });

    // ── 一致性哈希 ──
    // 这是分布式系统中用于决定数据路由到哪个节点的经典算法
    // 使用虚拟节点解决数据倾斜问题
    engine.register_fn(
        "consistent_hash",
        |replicas: i64, nodes: rhai::Array, key: &str| -> i64 {
            let replicas = replicas as usize;
            // 从 Rhai Array 中提取实际的节点列表
            let real_nodes: Vec<i64> = nodes.iter()
                .filter_map(|v| v.as_int().ok())
                .collect();

            // 构建哈希环: BTreeMap<哈希值, 节点编号>
            // BTreeMap 是有序的，支持 range 查询（找到下一个大于等于哈希值的节点）
            let mut circle: BTreeMap<i32, i64> = BTreeMap::new();
            for &node in &real_nodes {
                for i in 0..replicas {
                    let vnode_key = format!("{}{}", node, i);
                    circle.insert(fnv1a_hash(&vnode_key), node);
                }
            }

            if circle.is_empty() {
                return -1;
            }

            // 在哈希环上顺时针查找最近的节点
            let hash = fnv1a_hash(key);
            circle
                .range(hash..)            // 从 hash 开始找
                .next()                    // 下一个节点
                .or_else(|| circle.first_key_value()) // 环末尾回绕到开头
                .map(|(_, &node)| node)
                .unwrap_or(-1)
        },
    );

    // ── 日期时间解析 ──
    // 支持 yyyy-MM-dd 和 yyyy-MM-dd HH:mm:ss 两种格式
    engine.register_fn(
        "parse_datetime",
        |s: &str, pattern: &str| -> rhai::Map {
            // `chrono::NaiveDateTime` 不带时区的日期时间
            let naive_dt = if pattern.contains("HH") {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M"))
                    .ok()
            } else {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                    .ok()
            };

            // 构建返回给脚本的 Map
            let mut map = rhai::Map::new();
            if let Some(dt) = naive_dt {
                // `dt.year()` 等是 chrono 提供的方法
                // `.into()` 将值转为 Rhai 的 Dynamic 类型
                map.insert("year".into(), (dt.year() as i64).into());
                map.insert("month".into(), (dt.month() as i64).into());
                map.insert("day".into(), (dt.day() as i64).into());
                map.insert("hour".into(), (dt.hour() as i64).into());
                map.insert("minute".into(), (dt.minute() as i64).into());
                map.insert("second".into(), (dt.second() as i64).into());
                // ISO 周数（周一为起始，与 Java WeekFields.of(MONDAY, 1) 一致）
                map.insert("week_of_year".into(), (dt.iso_week().week() as i64).into());
            }
            map
        },
    );
}

/// 从 Rhai 作用域中提取变量值并转为字符串
///
/// ## Rust 知识点: 泛型 + trait 限定
/// `scope.get_value::<T>()` 中的 `::<T>` 是 turbofish 语法，
/// 用于显式指定泛型参数。
fn get_scope_value_as_string(scope: &Scope, name: &str, label: &str) -> Result<String> {
    // 尝试按不同 Rust 类型读取，然后转为 String
    if let Some(v) = scope.get_value::<String>(name) { return Ok(v); }
    if let Some(v) = scope.get_value::<i64>(name) { return Ok(v.to_string()); }
    if let Some(v) = scope.get_value::<f64>(name) { return Ok(v.to_string()); }
    if let Some(v) = scope.get_value::<bool>(name) { return Ok(v.to_string()); }
    Err(anyhow!("脚本未设置 {label} 变量"))
}

/// 执行路由脚本
///
/// ## 执行流程
/// 1. 创建 Rhai 引擎并设置安全限制
/// 2. 注册自定义函数
/// 3. 将编号 (code) 注入脚本作用域
/// 4. 执行用户编写的路由脚本
/// 5. 从作用域中提取 database 和 table_suffix
///
/// ## 安全限制
/// - `set_max_operations(100_000)`: 最多执行 10 万次操作（防止死循环）
/// - `set_max_modules(0)`: 禁止加载外部模块（沙箱安全）
/// - `set_max_call_levels(32)`: 最大调用深度（防止栈溢出）
pub fn execute_script(code: &str, rule: &RouteRule) -> Result<(String, String)> {
    let mut engine = Engine::new();
    // 安全限制
    engine.set_max_operations(100_000);
    engine.set_max_modules(0);
    engine.set_max_call_levels(32);

    // 注册所有自定义函数
    register_functions(&mut engine);

    // 创建作用域并注入编号变量
    let mut scope = Scope::new();
    scope.push("code", code.to_string()); // 脚本中可通过 `code` 变量访问

    // 执行脚本
    engine
        .run_with_scope(&mut scope, &rule.route_script)
        .map_err(|e| anyhow!("脚本执行错误: {e}"))?;

    // 从作用域中提取结果
    let database = get_scope_value_as_string(&scope, "database", "database")?;
    let table_suffix = get_scope_value_as_string(&scope, "table_suffix", "table_suffix")?;

    Ok((database, table_suffix))
}

/// 获取路由规则模板列表
/// 预置的常用路由模式，方便用户快速上手
pub fn get_templates() -> Vec<RouteRule> {
    vec![
        RouteRule {
            id: String::new(),
            name: "按位置截取".to_string(),
            description: "根据编号的固定位置截取库名和表名后缀".to_string(),
            code_length: 0,
            code_prefix: String::new(),
            route_script: r#"let database = "db_order_" + code[3..7];
let table_suffix = "_" + code[15..18];"#.to_string(),
            tables: vec![],
        },
        RouteRule {
            id: String::new(),
            name: "取模分片".to_string(),
            description: "根据编号长度对分片数取模".to_string(),
            code_length: 0,
            code_prefix: String::new(),
            route_script: r#"let n = code.len();
let shard = (n % 16).to_string();
let database = "db_order_" + shard;
let table_suffix = "_" + shard;"#.to_string(),
            tables: vec![],
        },
        RouteRule {
            id: String::new(),
            name: "日期分表".to_string(),
            description: "从编号中提取日期部分路由到对应月份表".to_string(),
            code_length: 0,
            code_prefix: String::new(),
            route_script: r#"let year = code[3..7];
let month = code[7..9];
let database = "db_log";
let table_suffix = "_" + year + "_" + month;"#.to_string(),
            tables: vec![],
        },
        RouteRule {
            id: String::new(),
            name: "一致性哈希分片".to_string(),
            description: "基于一致性哈希算法路由，适合节点动态增减场景".to_string(),
            code_length: 0,
            code_prefix: String::new(),
            route_script: r#"let nodes = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
let sharding = consistent_hash(32, nodes, code);
let database = "0";
let table_suffix = "_" + pad_left(sharding.to_string(), 2, "0");"#.to_string(),
            tables: vec![],
        },
    ]
}
