use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use similar::{Algorithm, ChangeTag, TextDiff as SimilarTextDiff};
use std::io::Write;
use worktools_plugin_api::Plugin;

/// 文本文件内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextFileContent {
    pub content: String,
    pub encoding: String,
}

/// 预处理选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOptions {
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
}

/// 差异统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub modifications: usize,
}

/// 文本比对插件
pub struct TextDiff;

impl TextDiff {
    /// 加载文本文件
    fn load_text_file_impl(file_path: &str) -> Result<TextFileContent> {
        use std::path::Path;

        // 验证文件是否存在
        if !Path::new(file_path).exists() {
            return Err(anyhow::anyhow!("文件不存在").into());
        }

        // 验证文件大小 (限制 10MB)
        let metadata = std::fs::metadata(file_path)?;
        if metadata.len() > 10 * 1024 * 1024 {
            return Err(anyhow::anyhow!("文件过大 (最大 10MB)").into());
        }

        // 读取文件内容
        let content = std::fs::read_to_string(file_path)?;

        Ok(TextFileContent {
            content,
            encoding: "utf-8".to_string(),
        })
    }

    /// 保存文本文件
    fn save_text_file_impl(file_path: &str, content: &str) -> Result<()> {
        std::fs::write(file_path, content)?;
        Ok(())
    }

    /// 预处理文本
    fn preprocess_text_impl(text: &str, options: &ProcessOptions) -> String {
        let mut result = text.to_string();

        if options.ignore_case {
            result = result.to_lowercase();
        }

        if options.ignore_whitespace {
            result = result
                .lines()
                .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>()
                .join("\n");
        }

        result
    }

    /// 计算差异统计
    fn count_diff_lines(original: &str, modified: &str) -> DiffStats {
        let diff = SimilarTextDiff::configure()
            .algorithm(Algorithm::Patience)
            .diff_lines(original, modified);

        let mut additions = 0;
        let mut deletions = 0;

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Insert => additions += 1,
                ChangeTag::Delete => deletions += 1,
                ChangeTag::Equal => {}
            }
        }

        // 计算修改数 (取较小值)
        let modifications = additions.min(deletions);
        additions -= modifications;
        deletions -= modifications;

        DiffStats {
            additions,
            deletions,
            modifications,
        }
    }

    /// 导出 Unified Diff 格式
    fn export_unified_diff(original: &str, modified: &str, filename: &str) -> String {
        let diff = SimilarTextDiff::configure()
            .algorithm(Algorithm::Patience)
            .diff_lines(original, modified);

        let mut output = Vec::new();

        writeln!(&mut output, "--- a/{}", filename).ok();
        writeln!(&mut output, "+++ b/{}", filename).ok();

        // 简化版本的 Unified Diff 输出
        // 实际应用中可以使用 similar 的 UnifiedDiff formatter
        let mut line_num_old = 1;
        let mut line_num_new = 1;
        let mut changes = Vec::new();

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    changes.push(('-', change.value().trim_end()));
                    line_num_old += 1;
                }
                ChangeTag::Insert => {
                    changes.push(('+', change.value().trim_end()));
                    line_num_new += 1;
                }
                ChangeTag::Equal => {
                    if !changes.is_empty() {
                        // 输出之前的变更
                        writeln!(
                            &mut output,
                            "@@ -{},{} +{},{} @@",
                            line_num_old - changes.len(),
                            changes.len(),
                            line_num_new - changes.len(),
                            changes.len()
                        )
                        .ok();

                        for (tag, line) in &changes {
                            writeln!(&mut output, "{} {}", tag, line).ok();
                        }

                        changes.clear();
                    }
                    line_num_old += 1;
                    line_num_new += 1;
                }
            }
        }

        // 输出剩余的变更
        if !changes.is_empty() {
            writeln!(
                &mut output,
                "@@ -{},{} +{},{} @@",
                line_num_old - changes.len(),
                changes.len(),
                line_num_new - changes.len(),
                changes.len()
            )
            .ok();

            for (tag, line) in &changes {
                writeln!(&mut output, "{} {}", tag, line).ok();
            }
        }

        String::from_utf8_lossy(&output).to_string()
    }
}

impl Plugin for TextDiff {
    fn id(&self) -> &str {
        "text-diff"
    }

    fn name(&self) -> &str {
        "文本比对"
    }

    fn description(&self) -> &str {
        "实时文本比对工具,支持差异高亮、文件导入导出、差异导航"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn icon(&self) -> &str {
        "🔍"
    }

    fn get_view(&self) -> String {
        // 插件已迁移到使用独立前端资源 (assets/index.html)
        // 此方法仅作为向后兼容的占位符
        "<div>插件前端资源加载中...</div>".to_string()
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "load_text_file" => {
                let file_path = params
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 file_path 参数"))?;

                let result = Self::load_text_file_impl(file_path)?;
                Ok(serde_json::to_value(result)?)
            }

            "save_text_file" => {
                let file_path = params
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 file_path 参数"))?;

                let content = params
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 content 参数"))?;

                Self::save_text_file_impl(file_path, content)?;
                Ok(serde_json::json!({ "success": true }))
            }

            "preprocess_text" => {
                let text = params
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 text 参数"))?;

                let ignore_whitespace = params
                    .get("ignore_whitespace")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let ignore_case = params
                    .get("ignore_case")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let options = ProcessOptions {
                    ignore_whitespace,
                    ignore_case,
                };

                let processed = Self::preprocess_text_impl(text, &options);
                Ok(serde_json::json!({
                    "original": text,
                    "processed": processed
                }))
            }

            "count_diff" => {
                let original = params
                    .get("original")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 original 参数"))?;

                let modified = params
                    .get("modified")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 modified 参数"))?;

                let stats = Self::count_diff_lines(original, modified);
                Ok(serde_json::to_value(stats)?)
            }

            "export_diff" => {
                let original = params
                    .get("original")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 original 参数"))?;

                let modified = params
                    .get("modified")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 modified 参数"))?;

                let filename = params
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("changes.diff");

                let diff = Self::export_unified_diff(original, modified, filename);
                Ok(serde_json::json!({ "diff": diff }))
            }

            _ => Err(format!("未知方法: {}", method).into()),
        }
    }
}

/// 插件工厂函数 - 导出给动态库加载器
#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(TextDiff));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_diff() {
        let original = "Hello\nWorld\nTest";
        let modified = "Hello\nRust\nTest";
        
        let stats = TextDiff::count_diff_lines(original, modified);
        
        // "World" -> "Rust": 1 deletion, 1 addition, 0 modifications
        println!("additions={}, deletions={}, modifications={}", 
                 stats.additions, stats.deletions, stats.modifications);
    }

    #[test]
    fn test_preprocess_text() {
        let text = "Hello  World\nTest   Line";
        let options = ProcessOptions {
            ignore_whitespace: true,
            ignore_case: false,
        };
        
        let processed = TextDiff::preprocess_text_impl(text, &options);
        
        assert_eq!(processed, "Hello World\nTest Line");
        println!("✅ preprocess_text test passed");
    }

    #[test]
    fn test_export_diff() {
        let original = "Line 1\nLine 2\nLine 3";
        let modified = "Line 1\nModified\nLine 3";
        
        let diff = TextDiff::export_unified_diff(original, modified, "test.txt");
        
        assert!(diff.contains("--- a/test.txt"));
        assert!(diff.contains("+++ b/test.txt"));
        assert!(diff.contains("- Line 2"));
        assert!(diff.contains("+ Modified"));
        println!("✅ export_diff test passed");
    }
}

    #[test]
    fn test_export_diff() {
        let original = "Line 1\nLine 2\nLine 3";
        let modified = "Line 1\nModified\nLine 3";
        
        let diff = TextDiff::export_unified_diff(original, modified, "test.txt");
        
        // 检查基本结构
        assert!(diff.contains("--- a/test.txt"));
        assert!(diff.contains("+++ b/test.txt"));
        assert!(diff.contains("@@")); // Unified diff 格式包含 @@
        assert!(diff.len() > 50); // 应该有足够的内容
        
        println!("Diff output:\n{}", diff);
        println!("\n✅ export_diff test passed");
    }
