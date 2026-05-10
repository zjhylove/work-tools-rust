use anyhow::Result;
use serde_json::Value;
use tracing::{info, warn};
use worktools_plugin_api::Plugin;

use crate::exporter::DocumentExporter;

pub mod exporter;
pub mod models;
pub mod parser;
pub mod storage;

pub struct ApiDocPlugin {
    storage: storage::ApiDocStorage,
}

impl ApiDocPlugin {
    pub fn new() -> Self {
        Self {
            storage: storage::ApiDocStorage::new(),
        }
    }

    fn handle_save_config(&self, params: Value) -> Result<Value> {
        let config: models::ApiDocConfig = serde_json::from_value(params)?;
        self.storage.save_config(&config)?;
        info!("API 文档配置已保存");
        Ok(serde_json::json!({"success": true}))
    }

    fn handle_load_config(&self) -> Result<Value> {
        let config = self.storage.load_config()?;
        Ok(serde_json::to_value(config)?)
    }

    fn handle_scan_controllers(&self, params: Value) -> Result<Value> {
        let jar_path = params["source_jar_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("缺少 source_jar_path 参数"))?;

        info!(jar_path = %jar_path, "开始扫描 Spring Boot JAR");
        let parser = parser::JarParser::new(jar_path)?;
        let controllers = parser.scan_controllers()?;
        info!(count = controllers.len(), "Controller 扫描完成");
        Ok(serde_json::to_value(controllers)?)
    }

    fn handle_parse_api_details(&self, params: Value) -> Result<Value> {
        let jar_path = params["source_jar_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("缺少 source_jar_path 参数"))?;
        let service_name = params["service_name"].as_str().unwrap_or("");
        let controllers: Vec<models::ControllerInfo> =
            serde_json::from_value(params["controllers"].clone())?;
        let selected: Vec<(String, String)> = serde_json::from_value(params["selected"].clone())?;

        info!(jar_path = %jar_path, api_count = selected.len(), "开始解析 API 详情");
        let mut parser = parser::JarParser::new(jar_path)?;

        if let Err(e) = parser.load_dependencies(jar_path, &[], true) {
            warn!(error = %e, "加载依赖 JAR 失败，继续解析");
        }

        let apis = parser.parse_api_details(&controllers, &selected, service_name)?;
        info!(count = apis.len(), "API 详情解析完成");
        Ok(serde_json::to_value(apis)?)
    }

    fn handle_export_docs(&self, params: Value) -> Result<Value> {
        let config: models::ExportConfig = serde_json::from_value(params.clone())?;
        let apis: Vec<models::ApiInfo> = serde_json::from_value(params["apis"].clone())?;
        let service_name = params["service_name"].as_str().unwrap_or("unknown");

        info!(count = apis.len(), formats = ?config.formats, output_dir = %config.output_dir, "开始导出文档");

        let mut output_files = Vec::new();

        for format in &config.formats {
            let files = match format {
                models::ExportFormat::Markdown => exporter::markdown::MarkdownExporter.export(
                    &apis,
                    &config.output_dir,
                    service_name,
                )?,
                models::ExportFormat::Html => {
                    exporter::html::HtmlExporter.export(&apis, &config.output_dir, service_name)?
                }
            };
            output_files.extend(files);
        }

        Ok(serde_json::to_value(output_files)?)
    }
}

impl Default for ApiDocPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ApiDocPlugin {
    fn id(&self) -> &str {
        "api-doc"
    }
    fn name(&self) -> &str {
        "API文档"
    }
    fn description(&self) -> &str {
        "解析Spring Boot JAR包,自动生成API接口文档"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn icon(&self) -> &str {
        "\u{1f4c4}"
    }

    fn get_view(&self) -> String {
        "<div>API文档生成器加载中...</div>".to_string()
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let result = match method {
            "save_config" => self.handle_save_config(params),
            "load_config" => self.handle_load_config(),
            "scan_controllers" => self.handle_scan_controllers(params),
            "parse_api_details" => self.handle_parse_api_details(params),
            "export_docs" => self.handle_export_docs(params),
            _ => Err(anyhow::anyhow!("未知方法: {}", method)),
        };
        result.map_err(|e| e.into())
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(ApiDocPlugin::new()));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
