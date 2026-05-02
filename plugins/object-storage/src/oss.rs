use crate::models::{BucketInfo, ObjectInfo};
use crate::provider::{self, HmacSha1, ObjectStoreProvider};
use anyhow::Result;
use base64::Engine;
use hmac::Mac;

pub struct OssClient {
    access_key: String,
    secret_key: String,
    endpoint: String,
    client: reqwest::blocking::Client,
}

impl OssClient {
    pub fn new(access_key: String, secret_key: String, endpoint: String) -> Self {
        let endpoint = endpoint
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string();
        Self {
            access_key,
            secret_key,
            endpoint,
            client: reqwest::blocking::Client::new(),
        }
    }

    fn bucket_host(&self, bucket: &str, region: &str) -> String {
        if !self.endpoint.is_empty() {
            format!("{}.{}", bucket, self.endpoint)
        } else {
            let r = region.strip_prefix("oss-").unwrap_or(region);
            format!("{}.oss-{}.aliyuncs.com", bucket, r)
        }
    }

    fn region_host(&self, region: &str) -> String {
        if !self.endpoint.is_empty() {
            self.endpoint.clone()
        } else {
            let r = region.strip_prefix("oss-").unwrap_or(region);
            format!("oss-{}.aliyuncs.com", r)
        }
    }

    fn sign(
        &self,
        verb: &str,
        date: &str,
        resource: &str,
        content_md5: &str,
        content_type: &str,
    ) -> String {
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}\n{}",
            verb, content_md5, content_type, date, resource
        );
        let mut mac = HmacSha1::new_from_slice(self.secret_key.as_bytes()).expect("HMAC");
        mac.update(string_to_sign.as_bytes());
        base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
    }

    fn auth_header(&self, verb: &str, date: &str, resource: &str) -> String {
        format!(
            "OSS {}:{}",
            self.access_key,
            self.sign(verb, date, resource, "", "")
        )
    }

    fn date_rfc2822() -> String {
        let now = std::time::SystemTime::now();
        let dt: chrono::DateTime<chrono::Utc> = now.into();
        dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
    }
}

impl ObjectStoreProvider for OssClient {
    fn list_buckets(&self, region: &str) -> Result<Vec<BucketInfo>> {
        let host = self.region_host(region);
        let date = Self::date_rfc2822();
        let auth = self.auth_header("GET", &date, "/");

        let resp = self
            .client
            .get(&format!("https://{}", host))
            .header("Authorization", &auth)
            .header("Date", &date)
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
        region: &str,
        prefix: &str,
        delimiter: Option<&str>,
        max_keys: Option<u32>,
    ) -> Result<(Vec<ObjectInfo>, Vec<String>)> {
        let host = self.bucket_host(bucket, region);
        let date = Self::date_rfc2822();
        let mut query = format!(
            "prefix={}&max-keys={}",
            provider::urlenc(prefix),
            max_keys.unwrap_or(1000)
        );
        if let Some(d) = delimiter {
            query.push_str(&format!("&delimiter={}", provider::urlenc(d)));
        }
        let resource = format!("/{}/", bucket);
        let auth = self.auth_header("GET", &date, &resource);

        let resp = self
            .client
            .get(&format!("https://{}?{}", host, query))
            .header("Authorization", &auth)
            .header("Date", &date)
            .header("Host", &host)
            .send()?;
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("列举对象 HTTP {}: {}", status, body);
        }
        parse_list_objects(&body)
    }

    fn get_object(&self, bucket: &str, region: &str, key: &str) -> Result<Vec<u8>> {
        let host = self.bucket_host(bucket, region);
        let date = Self::date_rfc2822();
        let resource = format!("/{}/{}", bucket, key);
        let auth = self.auth_header("GET", &date, &resource);

        let resp = self
            .client
            .get(&format!("https://{}/{}", host, provider::pct_encode(key)))
            .header("Authorization", &auth)
            .header("Date", &date)
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

    fn head_object(&self, bucket: &str, region: &str, key: &str) -> Result<ObjectInfo> {
        let host = self.bucket_host(bucket, region);
        let date = Self::date_rfc2822();
        let resource = format!("/{}/{}", bucket, key);
        let auth = self.auth_header("HEAD", &date, &resource);

        let resp = self
            .client
            .head(&format!("https://{}/{}", host, provider::pct_encode(key)))
            .header("Authorization", &auth)
            .header("Date", &date)
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
        region: &str,
        key: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<()> {
        let host = self.bucket_host(bucket, region);
        let date = Self::date_rfc2822();
        let resource = format!("/{}/{}", bucket, key);
        let content_md5 = {
            let d = md5::compute(data);
            base64::engine::general_purpose::STANDARD.encode(d.as_ref())
        };
        let sig = self.sign("PUT", &date, &resource, &content_md5, content_type);
        let auth = format!("OSS {}:{}", self.access_key, sig);

        let resp = self
            .client
            .put(&format!("https://{}/{}", host, provider::pct_encode(key)))
            .header("Authorization", &auth)
            .header("Date", &date)
            .header("Host", &host)
            .header("Content-Type", content_type)
            .header("Content-MD5", &content_md5)
            .body(data.to_vec())
            .send()?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("上传失败 HTTP {}: {}", status, body);
        }
        Ok(())
    }

    fn delete_object(&self, bucket: &str, region: &str, key: &str) -> Result<()> {
        let host = self.bucket_host(bucket, region);
        let date = Self::date_rfc2822();
        let resource = format!("/{}/{}", bucket, key);
        let auth = self.auth_header("DELETE", &date, &resource);

        let resp = self
            .client
            .delete(&format!("https://{}/{}", host, provider::pct_encode(key)))
            .header("Authorization", &auth)
            .header("Date", &date)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_list_buckets() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListAllMyBucketsResult>
  <Buckets>
    <Bucket>
      <Name>bucket-a</Name>
      <Location>oss-cn-hangzhou</Location>
      <CreationDate>2024-01-01T00:00:00Z</CreationDate>
    </Bucket>
    <Bucket>
      <Name>bucket-b</Name>
      <Location>oss-cn-beijing</Location>
      <CreationDate>2024-06-15T00:00:00Z</CreationDate>
    </Bucket>
  </Buckets>
</ListAllMyBucketsResult>"#;
        let buckets = parse_list_buckets(xml).unwrap();
        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets[0].name, "bucket-a");
        assert_eq!(buckets[0].region.as_deref().unwrap(), "oss-cn-hangzhou");
    }

    #[test]
    fn test_parse_list_objects() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult>
  <Contents>
    <Key>file1.txt</Key>
    <Size>1024</Size>
    <LastModified>2024-01-15T10:00:00Z</LastModified>
    <ETag>"abc123"</ETag>
  </Contents>
  <CommonPrefixes>
    <Prefix>dir1/</Prefix>
  </CommonPrefixes>
</ListBucketResult>"#;
        let (objects, prefixes) = parse_list_objects(xml).unwrap();
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].key, "file1.txt");
        assert_eq!(objects[0].size, 1024);
        assert_eq!(prefixes.len(), 1);
        assert_eq!(prefixes[0], "dir1/");
    }

    #[test]
    fn test_empty_list() {
        let xml = r#"<ListAllMyBucketsResult><Buckets></Buckets></ListAllMyBucketsResult>"#;
        assert!(parse_list_buckets(xml).unwrap().is_empty());
    }

    #[test]
    fn test_sign_deterministic() {
        let c = OssClient::new("ak".into(), "sk".into(), "".into());
        let s1 = c.auth_header("GET", "Thu, 30 Apr 2026 09:23:43 GMT", "/b/");
        let s2 = c.auth_header("GET", "Thu, 30 Apr 2026 09:23:43 GMT", "/b/");
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_sign_differs_by_verb() {
        let c = OssClient::new("ak".into(), "sk".into(), "".into());
        assert_ne!(
            c.auth_header("GET", "Thu, 30 Apr 2026 09:23:43 GMT", "/b/"),
            c.auth_header("PUT", "Thu, 30 Apr 2026 09:23:43 GMT", "/b/")
        );
    }

    #[test]
    fn test_endpoint_sanitization() {
        let c = OssClient::new(
            "ak".into(),
            "sk".into(),
            "https://oss-cn-hangzhou.aliyuncs.com/".into(),
        );
        assert_eq!(
            c.bucket_host("b", "cn-hangzhou"),
            "b.oss-cn-hangzhou.aliyuncs.com"
        );
    }

    #[test]
    fn test_no_endpoint_uses_region() {
        let c = OssClient::new("ak".into(), "sk".into(), "".into());
        assert_eq!(
            c.bucket_host("b", "cn-hangzhou"),
            "b.oss-cn-hangzhou.aliyuncs.com"
        );
    }

    #[test]
    fn test_region_strips_oss_prefix() {
        let c = OssClient::new("ak".into(), "sk".into(), "".into());
        assert_eq!(
            c.bucket_host("b", "oss-cn-hangzhou"),
            c.bucket_host("b", "cn-hangzhou")
        );
    }
}
