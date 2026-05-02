use crate::models::{BucketInfo, ObjectInfo};
use crate::provider::{self, HmacSha1, ObjectStoreProvider};
use anyhow::Result;
use hmac::Mac;

pub struct CosClient {
    access_key: String,
    secret_key: String,
    endpoint_suffix: String,
    client: reqwest::blocking::Client,
}

impl CosClient {
    pub fn new(access_key: String, secret_key: String, region: String) -> Self {
        Self {
            access_key,
            secret_key,
            endpoint_suffix: format!("cos.{}.myqcloud.com", region),
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn new_with_endpoint(access_key: String, secret_key: String, endpoint: String) -> Self {
        let ep = endpoint
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string();
        Self {
            access_key,
            secret_key,
            endpoint_suffix: ep,
            client: reqwest::blocking::Client::new(),
        }
    }

    fn sign(&self, verb: &str, path: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let key_time = format!("{};{}", now, now + 3600);

        let sign_key = {
            let mut mac = HmacSha1::new_from_slice(self.secret_key.as_bytes()).expect("HMAC");
            mac.update(key_time.as_bytes());
            mac.finalize().into_bytes()
        };

        use sha1::Digest;
        let http_string = format!("{}\n{}\n{}\n{}\n", verb.to_lowercase(), path, "", "");
        let sha1_http = format!("{:x}", sha1::Sha1::digest(http_string.as_bytes()));
        let string_to_sign = format!("sha1\n{}\n{}\n", key_time, sha1_http);

        let mut mac = HmacSha1::new_from_slice(&sign_key).expect("HMAC");
        mac.update(string_to_sign.as_bytes());
        let signature: String = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        format!("q-sign-algorithm=sha1&q-ak={}&q-sign-time={}&q-key-time={}&q-header-list=&q-url-param-list=&q-signature={}",
            self.access_key, key_time, key_time, signature)
    }
}

impl ObjectStoreProvider for CosClient {
    fn list_buckets(&self, _region: &str) -> Result<Vec<BucketInfo>> {
        let host = format!("service.{}", self.endpoint_suffix);
        let auth = self.sign("GET", "/");

        let resp = self
            .client
            .get(&format!("https://{}", host))
            .header("Authorization", &auth)
            .header("Host", &host)
            .send()?;
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("获取Bucket列表 HTTP {}: {}", status, body);
        }
        parse_list_buckets(&body)
    }

    fn list_objects(
        &self,
        bucket: &str,
        _region: &str,
        prefix: &str,
        delimiter: Option<&str>,
        max_keys: Option<u32>,
    ) -> Result<(Vec<ObjectInfo>, Vec<String>)> {
        let host = format!("{}.{}", bucket, self.endpoint_suffix);
        let mut query = format!(
            "prefix={}&max-keys={}",
            provider::urlenc(prefix),
            max_keys.unwrap_or(1000)
        );
        if let Some(d) = delimiter {
            query.push_str(&format!("&delimiter={}", provider::urlenc(d)));
        }
        let auth = self.sign("GET", "/");

        let resp = self
            .client
            .get(&format!("https://{}?{}", host, query))
            .header("Authorization", &auth)
            .header("Host", &host)
            .send()?;
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("列举对象 HTTP {}: {}", status, body);
        }
        parse_list_objects(&body)
    }

    fn get_object(&self, bucket: &str, _region: &str, key: &str) -> Result<Vec<u8>> {
        let host = format!("{}.{}", bucket, self.endpoint_suffix);
        let auth = self.sign("GET", &format!("/{}", key));

        let resp = self
            .client
            .get(&format!("https://{}/{}", host, provider::pct_encode(key)))
            .header("Authorization", &auth)
            .header("Host", &host)
            .send()?;
        let status = resp.status();
        let bytes = resp.bytes()?;
        if !status.is_success() {
            anyhow::bail!(
                "下载对象 HTTP {}: {}",
                status,
                String::from_utf8_lossy(&bytes)
            );
        }
        Ok(bytes.to_vec())
    }

    fn head_object(&self, bucket: &str, _region: &str, key: &str) -> Result<ObjectInfo> {
        let host = format!("{}.{}", bucket, self.endpoint_suffix);
        let auth = self.sign("HEAD", &format!("/{}", key));

        let resp = self
            .client
            .head(&format!("https://{}/{}", host, provider::pct_encode(key)))
            .header("Authorization", &auth)
            .header("Host", &host)
            .send()?;
        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("获取对象元数据 HTTP {}", status);
        }
        Ok(ObjectInfo {
            key: key.to_string(),
            size: resp
                .headers()
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            last_modified: resp
                .headers()
                .get("last-modified")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string(),
            etag: resp
                .headers()
                .get("etag")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .trim_matches('"')
                .to_string(),
            is_dir: key.ends_with('/'),
        })
    }

    fn put_object(
        &self,
        bucket: &str,
        _region: &str,
        key: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<()> {
        let host = format!("{}.{}", bucket, self.endpoint_suffix);
        let auth = self.sign("PUT", &format!("/{}", key));

        let resp = self
            .client
            .put(&format!("https://{}/{}", host, provider::pct_encode(key)))
            .header("Authorization", &auth)
            .header("Host", &host)
            .header("Content-Type", content_type)
            .body(data.to_vec())
            .send()?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("上传失败 HTTP {}: {}", status, body);
        }
        Ok(())
    }

    fn delete_object(&self, bucket: &str, _region: &str, key: &str) -> Result<()> {
        let host = format!("{}.{}", bucket, self.endpoint_suffix);
        let auth = self.sign("DELETE", &format!("/{}", key));

        let resp = self
            .client
            .delete(&format!("https://{}/{}", host, provider::pct_encode(key)))
            .header("Authorization", &auth)
            .header("Host", &host)
            .send()?;
        let status = resp.status();
        if status.is_success() || status.as_u16() == 204 {
            return Ok(());
        }
        let body = resp.text().unwrap_or_default();
        anyhow::bail!("删除失败 HTTP {}: {}", status, body)
    }
}

fn parse_list_buckets(xml: &str) -> Result<Vec<BucketInfo>> {
    let mut buckets = Vec::new();
    let mut name = String::new();
    let mut loc = String::new();
    let mut cdate = String::new();
    let mut in_bucket = false;

    for line in xml.lines() {
        let t = line.trim();
        if t == "<Bucket>" {
            in_bucket = true;
            name.clear();
            loc.clear();
            cdate.clear();
            continue;
        }
        if t == "</Bucket>" {
            if in_bucket {
                buckets.push(BucketInfo {
                    name: name.clone(),
                    region: Some(loc.clone()).filter(|s| !s.is_empty()),
                    creation_date: Some(cdate.clone()).filter(|s| !s.is_empty()),
                });
            }
            in_bucket = false;
            continue;
        }
        if in_bucket {
            if t.starts_with("<Name>") {
                name = provider::strip_tag(t, "Name");
            }
            if t.starts_with("<Location>") {
                loc = provider::strip_tag(t, "Location");
            }
            if t.starts_with("<CreationDate>") {
                cdate = provider::strip_tag(t, "CreationDate");
            }
        }
    }
    Ok(buckets)
}

fn parse_list_objects(xml: &str) -> Result<(Vec<ObjectInfo>, Vec<String>)> {
    let mut objects = Vec::new();
    let mut prefixes = Vec::new();
    let mut in_contents = false;
    let mut in_common = false;
    let mut key = String::new();
    let mut size: u64 = 0;
    let mut lm = String::new();
    let mut etag = String::new();

    for line in xml.lines() {
        let t = line.trim();
        if t == "<Contents>" {
            in_contents = true;
            key.clear();
            lm.clear();
            etag.clear();
            size = 0;
            continue;
        }
        if t == "</Contents>" && in_contents {
            in_contents = false;
            objects.push(ObjectInfo {
                key: key.clone(),
                size,
                last_modified: lm.clone(),
                etag: etag.trim_matches('"').to_string(),
                is_dir: key.ends_with('/'),
            });
            continue;
        }
        if t == "<CommonPrefixes>" {
            in_common = true;
            continue;
        }
        if t == "</CommonPrefixes>" {
            in_common = false;
            continue;
        }
        if in_contents {
            if t.starts_with("<Key>") {
                key = provider::strip_tag(t, "Key");
            }
            if t.starts_with("<Size>") {
                size = provider::strip_tag(t, "Size").parse().unwrap_or(0);
            }
            if t.starts_with("<LastModified>") {
                lm = provider::strip_tag(t, "LastModified");
            }
            if t.starts_with("<ETag>") {
                etag = provider::strip_tag(t, "ETag");
            }
        }
        if in_common && t.starts_with("<Prefix>") {
            prefixes.push(provider::strip_tag(t, "Prefix"));
        }
    }
    Ok((objects, prefixes))
}
