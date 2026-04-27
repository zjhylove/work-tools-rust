use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use chrono::{Datelike, Timelike};
use rhai::{Engine, Scope};

use super::model::RouteRule;

/// FNV-1a 32-bit hash + avalanche，与 Java 版 HashUtil.fnvHash 完全一致
fn fnv1a_hash(s: &str) -> i32 {
    let p: i32 = 16777619;
    let mut hash: i32 = 2166136261_u32 as i32; // (int) 2166136261L
    for &b in s.as_bytes() {
        hash = (hash ^ b as i32).wrapping_mul(p);
    }
    // avalanche (Java >> 为算术右移，与 i32.wrapping_shr 一致)
    hash = hash.wrapping_add(hash << 13);
    hash ^= hash >> 7;
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 17;
    hash = hash.wrapping_add(hash << 5);
    hash.abs()
}

fn register_functions(engine: &mut Engine) {
    // ── 字符串方法 ───────────────────────────────────
    // bytes(): 按字节迭代 (返回 Vec<i64>)
    engine.register_fn(
        "bytes",
        |s: &str| s.as_bytes().iter().map(|&b| b as i64).collect::<Vec<i64>>(),
    );
    // to_upper() / to_lower()
    engine.register_fn("to_upper", |s: &str| s.to_uppercase());
    engine.register_fn("to_lower", |s: &str| s.to_lowercase());
    // trim() / trim_start() / trim_end()
    engine.register_fn("trim", |s: &str| s.trim().to_string());
    engine.register_fn("trim_start", |s: &str| s.trim_start().to_string());
    engine.register_fn("trim_end", |s: &str| s.trim_end().to_string());
    // contains() / starts_with() / ends_with()
    engine.register_fn("contains", |s: &str, sub: &str| s.contains(sub));
    engine.register_fn("starts_with", |s: &str, prefix: &str| s.starts_with(prefix));
    engine.register_fn("ends_with", |s: &str, suffix: &str| s.ends_with(suffix));
    // replace()
    engine.register_fn("replace", |s: &str, from: &str, to: &str| s.replace(from, to));
    // split(): 字符串分割，返回 Vec<String>
    engine.register_fn("split", |s: &str, sep: &str| -> Vec<String> {
        s.split(sep).map(String::from).collect()
    });
    // find(): 查找子串位置，未找到返回 -1
    engine.register_fn("find", |s: &str, sub: &str| -> i64 {
        s.find(sub).map(|i| i as i64).unwrap_or(-1)
    });
    // repeat()
    engine.register_fn("repeat", |s: &str, n: i64| s.repeat(n as usize));
    // pad_left() / pad_right()
    engine.register_fn("pad_left", |s: &str, n: i64, ch: &str| {
        let fill = ch.chars().next().unwrap_or(' ');
        let width = n as usize;
        if s.len() >= width {
            s.to_string()
        } else {
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
    // substring()
    engine.register_fn("substring", |s: &str, start: i64, end: i64| -> String {
        let start = start.max(0) as usize;
        let end = end.max(start as i64) as usize;
        s.get(start..end.min(s.len())).unwrap_or("").to_string()
    });
    // is_empty()
    engine.register_fn("is_empty", |s: &str| s.is_empty());

    // ── 数学函数 ─────────────────────────────────────
    engine.register_fn("abs", |n: i64| n.abs());
    engine.register_fn("min", |a: i64, b: i64| a.min(b));
    engine.register_fn("max", |a: i64, b: i64| a.max(b));
    engine.register_fn("pow", |base: i64, exp: i64| -> i64 {
        base.saturating_pow(exp as u32)
    });
    engine.register_fn("sqrt", |n: i64| -> i64 { (n as f64).sqrt() as i64 });
    engine.register_fn("floor", |n: f64| n.floor() as i64);
    engine.register_fn("ceil", |n: f64| n.ceil() as i64);
    engine.register_fn("round", |n: f64| n.round() as i64);

    // ── 类型转换 ─────────────────────────────────────
    engine.register_fn("to_int", |s: &str| -> i64 {
        s.trim().parse().unwrap_or(0)
    });
    engine.register_fn("to_float", |s: &str| -> f64 {
        s.trim().parse().unwrap_or(0.0)
    });
    engine.register_fn("to_string", |n: i64| n.to_string());
    engine.register_fn("to_string", |n: f64| n.to_string());
    engine.register_fn("to_string", |b: bool| b.to_string());

    // ── 哈希函数 ─────────────────────────────────────
    engine.register_fn("hash_code", |s: &str| -> i64 {
        let mut h: i64 = 0;
        for &b in s.as_bytes() {
            h = h.wrapping_mul(31).wrapping_add(b as i64);
        }
        h
    });
    // murmur32: 与 Java 版一致的 murmur3_32 哈希算法
    engine.register_fn("murmur32", |s: &str| -> i64 {
        let bytes = s.as_bytes();
        let c1: u32 = 0xcc9e2d51;
        let c2: u32 = 0x1b873593;
        let mut h1: u32 = 0;
        let n = bytes.len();
        let rounded_end = n & !3;

        let mut i = 0;
        while i < rounded_end {
            let mut k1 = u32::from_le_bytes([
                bytes[i],
                bytes[i + 1],
                bytes[i + 2],
                bytes[i + 3],
            ]);
            k1 = k1.wrapping_mul(c1);
            k1 = k1.rotate_left(15);
            k1 = k1.wrapping_mul(c2);
            h1 ^= k1;
            h1 = h1.rotate_left(13);
            h1 = h1.wrapping_mul(5).wrapping_add(0xe6546b64);
            i += 4;
        }

        let mut k1: u32 = 0;
        let tail = n & 3;
        if tail >= 3 {
            k1 ^= (bytes[rounded_end + 2] as u32) << 16;
        }
        if tail >= 2 {
            k1 ^= (bytes[rounded_end + 1] as u32) << 8;
        }
        if tail >= 1 {
            k1 ^= bytes[rounded_end] as u32;
            k1 = k1.wrapping_mul(c1);
            k1 = k1.rotate_left(15);
            k1 = k1.wrapping_mul(c2);
            h1 ^= k1;
        }

        h1 ^= n as u32;
        h1 ^= h1 >> 16;
        h1 = h1.wrapping_mul(0x85ebca6b);
        h1 ^= h1 >> 13;
        h1 = h1.wrapping_mul(0xc2b2ae35);
        h1 ^= h1 >> 16;
        // 先转 i32 再转 i64 取绝对值，与 JS 的 Math.abs(int32) 行为一致
        (h1 as i32 as i64).abs()
    });
    // fnv_hash: FNV-1a 32-bit 哈希，与 Hutool HashUtil.fnvHash 一致
    engine.register_fn("fnv_hash", |s: &str| -> i64 { fnv1a_hash(s) as i64 });
    // bkdr_hash: BKDR 哈希，与 Hutool HashUtil.bkdrHash 一致
    engine.register_fn("bkdr_hash", |s: &str| -> i64 {
        let seed: i32 = 131;
        let mut hash: i32 = 0;
        for &b in s.as_bytes() {
            hash = hash.wrapping_mul(seed) + b as i32;
        }
        (hash & 0x7FFFFFFF) as i64
    });
    // consistent_hash: 一致性哈希路由 (replicas=虚拟节点数, nodes=节点数组, key=路由键)
    engine.register_fn(
        "consistent_hash",
        |replicas: i64, nodes: rhai::Array, key: &str| -> i64 {
            let replicas = replicas as usize;
            let real_nodes: Vec<i64> = nodes
                .iter()
                .filter_map(|v| v.as_int().ok())
                .collect();
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

            let hash = fnv1a_hash(key);
            circle
                .range(hash..)
                .next()
                .or_else(|| circle.first_key_value())
                .map(|(_, &node)| node)
                .unwrap_or(-1)
        },
    );
    // parse_datetime: 解析日期时间字符串，返回含 year/month/day/hour/minute/second/week_of_year 的 map
    // pattern 仅支持 yyyy-MM-dd / yyyy-MM-dd HH:mm:ss 两种格式
    engine.register_fn(
        "parse_datetime",
        |s: &str, pattern: &str| -> rhai::Map {
            let naive_dt = if pattern.contains("HH") {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M"))
                    .ok()
            } else {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                    .ok()
            };
            let mut map = rhai::Map::new();
            if let Some(dt) = naive_dt {
                map.insert("year".into(), (dt.year() as i64).into());
                map.insert("month".into(), (dt.month() as i64).into());
                map.insert("day".into(), (dt.day() as i64).into());
                map.insert("hour".into(), (dt.hour() as i64).into());
                map.insert("minute".into(), (dt.minute() as i64).into());
                map.insert("second".into(), (dt.second() as i64).into());
                // ISO 周数: 周一为起始，最小天数=1，与 Java WeekFields.of(MONDAY, 1) 一致
                map.insert(
                    "week_of_year".into(),
                    (dt.iso_week().week() as i64).into(),
                );
            }
            map
        },
    );
}

fn get_scope_value_as_string(scope: &Scope, name: &str, label: &str) -> Result<String> {
    if let Some(v) = scope.get_value::<String>(name) {
        return Ok(v);
    }
    if let Some(v) = scope.get_value::<i64>(name) {
        return Ok(v.to_string());
    }
    if let Some(v) = scope.get_value::<f64>(name) {
        return Ok(v.to_string());
    }
    if let Some(v) = scope.get_value::<bool>(name) {
        return Ok(v.to_string());
    }
    Err(anyhow!("脚本未设置 {label} 变量"))
}

pub fn execute_script(code: &str, rule: &RouteRule) -> Result<(String, String)> {
    let mut engine = Engine::new();
    engine.set_max_operations(100_000);
    engine.set_max_modules(0);
    engine.set_max_call_levels(32);
    register_functions(&mut engine);

    let mut scope = Scope::new();
    scope.push("code", code.to_string());

    engine
        .run_with_scope(&mut scope, &rule.route_script)
        .map_err(|e| anyhow!("脚本执行错误: {e}"))?;

    let database = get_scope_value_as_string(&scope, "database", "database")?;

    let table_suffix = get_scope_value_as_string(&scope, "table_suffix", "table_suffix")?;

    Ok((database, table_suffix))
}

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
            description: "基于一致性哈希算法路由，使用 FNV-1a 哈希，适合节点动态增减场景".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rule(script: &str) -> RouteRule {
        RouteRule {
            id: "test".to_string(),
            name: "test".to_string(),
            description: String::new(),
            code_length: 0,
            code_prefix: String::new(),
            route_script: script.to_string(),
            tables: vec!["t_order".to_string(), "t_order_item".to_string()],
        }
    }

    #[test]
    fn test_execute_positional() {
        let rule = make_rule(r#"let database = "db_" + code[3..7];
let table_suffix = "_" + code[7..10];"#);
        let (db, suffix) = execute_script("ORD2024001", &rule).unwrap();
        assert_eq!(db, "db_2024");
        assert_eq!(suffix, "_001");
    }

    #[test]
    fn test_execute_modulo() {
        let rule = make_rule(r#"let n = code.len();
let shard = (n % 4).to_string();
let database = "db_" + shard;
let table_suffix = "_" + shard;"#);
        let (db, suffix) = execute_script("ABC123", &rule).unwrap();
        assert_eq!(db, "db_2");
        assert_eq!(suffix, "_2");
    }

    #[test]
    fn test_execute_missing_variable() {
        let rule = make_rule("let database = \"db\";");
        let result = execute_script("test", &rule);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("table_suffix"));
    }

    #[test]
    fn test_execute_script_error() {
        let rule = make_rule("let x = 1 / 0;");
        let result = execute_script("test", &rule);
        assert!(result.is_err());
    }

    #[test]
    fn test_murmur32_sharding() {
        let rule = make_rule(r#"let ds_num = 10;
let tb_num = 100;

let hash_str = murmur32(code).to_string();
let hash_len = hash_str.len();
let hash;
if hash_len < 3 {
    hash = "000"[0..(3 - hash_len)] + hash_str;
} else {
    hash = substring(hash_str, hash_len - 3, hash_len);
}

let ds = to_int(hash[0..1]) % ds_num;
let database = pad_left(ds.to_string(), 2, "0");

let tb = to_int(hash[1..3]) % tb_num;
let table_suffix = "_" + pad_left(tb.to_string(), 3, "0");"#);
        let (db, suffix) = execute_script("1000000129126011", &rule).unwrap();
        assert_eq!(db, "00");
        assert_eq!(suffix, "_076");
    }

    #[test]
    fn test_get_templates() {
        let templates = get_templates();
        assert_eq!(templates.len(), 4);
        assert_eq!(templates[0].name, "按位置截取");
        assert_eq!(templates[1].name, "取模分片");
        assert_eq!(templates[2].name, "日期分表");
        assert_eq!(templates[3].name, "一致性哈希分片");
    }

    #[test]
    fn test_consistent_hash_routing() {
        let rule = make_rule(r#"let nodes = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
let sharding = consistent_hash(32, nodes, code);
let database = "0";
let table_suffix = "_" + pad_left(sharding.to_string(), 2, "0");"#);
        let (db, suffix) = execute_script("1000000129126011", &rule).unwrap();
        assert_eq!(db, "0");
        // 验证 sharding 在 0-9 范围内
        assert!(suffix.starts_with('_'));
        let num: i64 = suffix[1..].parse().unwrap();
        assert!((0..=9).contains(&num));
    }

    #[test]
    fn test_consistent_hash_deterministic() {
        let rule = make_rule(r#"let nodes = [0, 1, 2, 3, 4];
let sharding = consistent_hash(16, nodes, code);
let database = sharding.to_string();
let table_suffix = "_" + pad_left(sharding.to_string(), 2, "0");"#);
        // 相同输入必须产生相同结果
        let r1 = execute_script("test_key_123", &rule).unwrap();
        let r2 = execute_script("test_key_123", &rule).unwrap();
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_fnv_hash_basic() {
        let rule = make_rule("let h = fnv_hash(code);\nlet database = h.to_string();\nlet table_suffix = \"_0\";");
        let (db, _) = execute_script("hello", &rule).unwrap();
        let h: i64 = db.parse().unwrap();
        assert_ne!(h, 0);
    }

    #[test]
    fn test_bkdr_hash_routing() {
        let rule = make_rule(r#"let sharding = bkdr_hash(code) % 10;
let database = "0";
let table_suffix = "_" + pad_left(sharding.to_string(), 2, "0");"#);
        let (db, suffix) = execute_script("1000000131783927", &rule).unwrap();
        assert_eq!(db, "0");
        let num: i64 = suffix[1..].parse().unwrap();
        assert!((0..=9).contains(&num));
    }

    #[test]
    fn test_parse_datetime_week_routing() {
        let rule = make_rule(r#"let dt = parse_datetime(code, "yyyy-MM-dd HH:mm:ss");
let database = "0";
let table_suffix = "_" + dt["year"].to_string() + "_" + dt["week_of_year"].to_string();"#);
        let (db, suffix) = execute_script("2025-02-21 10:10:10", &rule).unwrap();
        assert_eq!(db, "0");
        assert_eq!(suffix, "_2025_8");
    }

    #[test]
    fn test_parse_datetime_date_only() {
        let rule = make_rule(r#"let dt = parse_datetime(code, "yyyy-MM-dd");
let database = "0";
let table_suffix = "_" + dt["year"].to_string() + "_" + dt["week_of_year"].to_string();"#);
        let (db, suffix) = execute_script("2025-12-31", &rule).unwrap();
        assert_eq!(db, "0");
        // 2025-12-31 是周三，属于 2026 年第 1 周 (ISO)
        assert_eq!(suffix, "_2025_1");
    }
}
