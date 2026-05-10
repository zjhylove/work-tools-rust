pub mod html;
pub mod markdown;

use crate::models::ApiInfo;
use anyhow::Result;

pub trait DocumentExporter {
    fn export(&self, apis: &[ApiInfo], output_dir: &str, service_name: &str)
        -> Result<Vec<String>>;
}

pub(crate) fn sanitize_filename(name: &str) -> String {
    name.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")
}
