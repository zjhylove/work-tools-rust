use crate::models::{BucketInfo, ObjectInfo};
use anyhow::Result;
use hmac::Hmac;
use sha1::Sha1;

pub type HmacSha1 = Hmac<Sha1>;

/// 云服务商统一接口
pub trait ObjectStoreProvider {
    fn list_buckets(&self, region: &str) -> Result<Vec<BucketInfo>>;
    fn list_objects(&self, bucket: &str, region: &str, prefix: &str, delimiter: Option<&str>, max_keys: Option<u32>) -> Result<(Vec<ObjectInfo>, Vec<String>)>;
    fn get_object(&self, bucket: &str, region: &str, key: &str) -> Result<Vec<u8>>;
    fn head_object(&self, bucket: &str, region: &str, key: &str) -> Result<ObjectInfo>;
    fn put_object(&self, bucket: &str, region: &str, key: &str, data: &[u8], content_type: &str) -> Result<()>;
    fn delete_object(&self, bucket: &str, region: &str, key: &str) -> Result<()>;
}

// -- shared helpers --

pub fn strip_tag(s: &str, tag: &str) -> String {
    s.trim()
        .replace(&format!("<{}>", tag), "")
        .replace(&format!("</{}>", tag), "")
        .trim()
        .to_string()
}

pub fn urlenc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

pub fn pct_encode(s: &str) -> String {
    s.split('/').map(urlenc).collect::<Vec<_>>().join("/")
}
