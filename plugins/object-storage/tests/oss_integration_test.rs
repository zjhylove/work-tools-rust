// 真实 OSS 集成测试
// 运行方式:
//   cargo test -p object-storage --test oss_integration_test -- --ignored --nocapture

use object_storage::cos::CosClient;
use object_storage::crypto;
use object_storage::models::ConnectionConfig;
use object_storage::oss::OssClient;
use object_storage::provider::ObjectStoreProvider;

fn load_connections() -> Vec<ConnectionConfig> {
    let home = dirs::home_dir().unwrap();
    let path = home
        .join(".worktools")
        .join("history")
        .join("plugins")
        .join("object-storage.json");
    if !path.exists() {
        eprintln!("数据文件不存在: {}", path.display());
        return vec![];
    }
    let json = std::fs::read_to_string(&path).unwrap();
    let data: serde_json::Value = serde_json::from_str(&json).unwrap();
    let arr = data.get("connections").and_then(|v| v.as_array());
    match arr {
        Some(list) => list
            .iter()
            .map(|v| serde_json::from_value::<ConnectionConfig>(v.clone()).unwrap())
            .collect(),
        None => vec![],
    }
}

#[test]
#[ignore]
fn test_list_buckets_aliyun() {
    let connections = load_connections();
    let conn = connections
        .iter()
        .find(|c| c.provider == "aliyun")
        .expect("未找到阿里云连接，请先在前端添加连接");

    println!("=== 测试阿里云 OSS list_buckets ===");
    println!("连接名称: {}", conn.name);
    println!("Region: {}", conn.region);

    let ak = crypto::decrypt(&conn.access_key);
    let sk = crypto::decrypt(&conn.secret_key);
    let endpoint = conn.endpoint.as_deref().unwrap_or("");

    let client = OssClient::new(ak, sk, endpoint.to_string());
    let result = client.list_buckets(&conn.region);

    match result {
        Ok(buckets) => {
            println!("\n✅ 成功，共 {} 个 bucket:", buckets.len());
            for b in &buckets {
                println!("  📁 {}  region={:?}", b.name, b.region);
            }
            assert!(!buckets.is_empty(), "至少应有一个 bucket");
        }
        Err(e) => {
            println!("\n❌ 失败: {}", e);
            panic!("{}", e);
        }
    }
}

#[test]
#[ignore]
fn test_list_buckets_tencent() {
    let connections = load_connections();
    let conn = connections
        .iter()
        .find(|c| c.provider == "tencent")
        .expect("未找到腾讯云连接");

    println!("=== 测试腾讯云 COS list_buckets ===");

    let ak = crypto::decrypt(&conn.access_key);
    let sk = crypto::decrypt(&conn.secret_key);

    let client = if let Some(ref ep) = conn.endpoint {
        CosClient::new_with_endpoint(ak, sk, ep.clone())
    } else {
        CosClient::new(ak, sk, conn.region.clone())
    };

    let result = client.list_buckets(&conn.region);
    match result {
        Ok(buckets) => {
            println!("\n✅ 成功，共 {} 个 bucket:", buckets.len());
            for b in &buckets {
                println!("  📁 {}  region={:?}", b.name, b.region);
            }
        }
        Err(e) => {
            println!("\n❌ 失败: {}", e);
            panic!("{}", e);
        }
    }
}
