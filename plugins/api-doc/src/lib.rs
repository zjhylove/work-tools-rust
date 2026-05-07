use anyhow::Result;
use serde_json::Value;
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

        let parser = parser::JarParser::new(jar_path)?;
        let controllers = parser.scan_controllers()?;
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

        let mut parser = parser::JarParser::new(jar_path)?;

        // 加载依赖
        let dep_jars: Vec<String> = params["dependency_jars"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let auto_scan = params["auto_scan_dependencies"].as_bool().unwrap_or(false);

        if auto_scan || !dep_jars.is_empty() {
            let _ = parser.load_dependencies(jar_path, &dep_jars, auto_scan);
        }

        let apis = parser.parse_api_details(&controllers, &selected, service_name)?;
        Ok(serde_json::to_value(apis)?)
    }

    fn handle_export_docs(&self, params: Value) -> Result<Value> {
        let config: models::ExportConfig = serde_json::from_value(params.clone())?;
        let apis: Vec<models::ApiInfo> = serde_json::from_value(params["apis"].clone())?;
        let service_name = params["service_name"].as_str().unwrap_or("unknown");

        let mut output_files = Vec::new();

        for format in &config.formats {
            let files = match format {
                models::ExportFormat::Word => {
                    exporter::word::WordExporter.export(&apis, &config.output_dir, service_name)?
                }
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

        // 保存导出历史
        let history = models::ExportHistory {
            id: uuid::Uuid::new_v4().to_string(),
            service_name: service_name.to_string(),
            api_count: apis.len(),
            formats: config.formats.clone(),
            output_path: config.output_dir.clone(),
            exported_at: chrono::Local::now().to_rfc3339(),
        };
        self.storage.add_export_history(history)?;

        Ok(serde_json::to_value(output_files)?)
    }

    fn handle_get_export_history(&self) -> Result<Value> {
        let history = self.storage.get_export_history()?;
        Ok(serde_json::to_value(history)?)
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
            "get_export_history" => self.handle_get_export_history(),
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
