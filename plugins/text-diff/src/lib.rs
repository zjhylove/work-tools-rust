//! # 文本比对插件
//!
//! 提供文本差异比对功能，支持：
//! - 文件加载/保存
//! - 文本预处理（忽略空白、忽略大小写）
//! - 差异统计（新增、删除、修改行数）
//! - Unified Diff 格式导出
//!
//! ## 使用 similar crate
//! `similar` 是 Rust 的高性能文本比对库，支持多种算法（Patience、Myers、LCS）。
//! Patience 算法在处理代码差异时通常给出更可读的结果。
//!
//! ## Rust 知识点
//! - `similar::TextDiff`: 文本差异引擎
//! - `writeln!` 宏: 写入格式化字符串到 writer（此处是 Vec<u8>）
//! - `String::from_utf8_lossy`: 从字节创建字符串，无效 UTF-8 用替换字符处理

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
    pub encoding: String, // 编码格式（目前仅支持 UTF-8）
}

/// 预处理选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOptions {
    pub ignore_whitespace: bool, // 忽略空白字符差异
    pub ignore_case: bool,       // 忽略大小写差异
}

/// 差异统计结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub additions: usize,     // 新增行数
    pub deletions: usize,     // 删除行数
    pub modifications: usize, // 修改行数（新增和删除的重叠部分）
}

/// 文本比对插件
pub struct TextDiff;

impl TextDiff {
    /// 加载文本文件（最大 10MB）
    fn load_text_file_impl(file_path: &str) -> Result<TextFileContent> {
        use std::path::Path;

        if !Path::new(file_path).exists() {
            return Err(anyhow::anyhow!("文件不存在").into());
        }

        // 检查文件大小限制（防止读取超大文件导致内存不足）
        let metadata = std::fs::metadata(file_path)?;
        if metadata.len() > 10 * 1024 * 1024 {
            return Err(anyhow::anyhow!("文件过大 (最大 10MB)").into());
        }

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
    ///
    /// ## Rust 知识点: 迭代器链
    /// `text.lines()` — 按行分割的迭代器
    /// `.map(|line| line.split_whitespace()...)` — 对每行：分割空白 → 合并
    /// `.collect::<Vec<_>>()` — 收集到 Vec（类型由 `.join("\n")` 推断）
    fn preprocess_text_impl(text: &str, options: &ProcessOptions) -> String {
        let mut result = text.to_string();

        if options.ignore_case {
            result = result.to_lowercase();
        }

        if options.ignore_whitespace {
            // 对每行：将连续的空白符合并为单个空格
            result = result
                .lines()
                .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>()
                .join("\n");
        }

        result
    }

    /// 计算差异统计
    ///
    /// `similar::TextDiff::configure()` 使用建造者模式配置差异算法。
    /// `.algorithm(Algorithm::Patience)` 选择 Patience 算法（擅长处理代码差异）。
    fn count_diff_lines(original: &str, modified: &str) -> DiffStats {
        let diff = SimilarTextDiff::configure()
            .algorithm(Algorithm::Patience)
            .diff_lines(original, modified); // 按行比较

        let mut additions = 0;
        let mut deletions = 0;

        // `iter_all_changes()` 遍历所有变更块
        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Insert => additions += 1, // 新增的行
                ChangeTag::Delete => deletions += 1, // 删除的行
                ChangeTag::Equal => {}               // 未变更的行
            }
        }

        // 修改的行 = min(新增, 删除)
        // 将重叠部分记为"修改"，剩余的记为纯粹的新增/删除
        let modifications = additions.min(deletions);
        additions -= modifications;
        deletions -= modifications;

        DiffStats { additions, deletions, modifications }
    }

    /// 导出 Unified Diff 格式
    ///
    /// Unified Diff 是 Git diff 使用的标准格式：
    /// ```diff
    /// --- a/file.txt
    /// +++ b/file.txt
    /// @@ -1,3 +1,3 @@
    ///  未修改的行
    /// -删除的行
    /// +新增的行
    ///  未修改的行
    /// ```
    fn export_unified_diff(original: &str, modified: &str, filename: &str) -> String {
        let diff = SimilarTextDiff::configure()
            .algorithm(Algorithm::Patience)
            .diff_lines(original, modified);

        // 使用 Vec<u8> 作为 writer（比 String 更适合二进制安全的场景）
        let mut output = Vec::new();

        // 写入文件头
        writeln!(&mut output, "--- a/{}", filename).ok();
        writeln!(&mut output, "+++ b/{}", filename).ok();

        let mut line_num_old = 1; // 原文件行号
        let mut line_num_new = 1; // 新文件行号
        let mut changes = Vec::new(); // 当前变更块的缓存

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
                    // 遇到未变更行时，先输出之前积累的变更块
                    if !changes.is_empty() {
                        writeln!(
                            &mut output,
                            "@@ -{},{} +{},{} @@",
                            line_num_old - changes.len(), changes.len(),
                            line_num_new - changes.len(), changes.len()
                        ).ok();

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

        // 输出文件末尾的剩余变更
        if !changes.is_empty() {
            writeln!(
                &mut output,
                "@@ -{},{} +{},{} @@",
                line_num_old - changes.len(), changes.len(),
                line_num_new - changes.len(), changes.len()
            ).ok();

            for (tag, line) in &changes {
                writeln!(&mut output, "{} {}", tag, line).ok();
            }
        }

        // `String::from_utf8_lossy` 将字节转为字符串
        // 无效的 UTF-8 字节会被替换为 � (U+FFFD)
        String::from_utf8_lossy(&output).to_string()
    }
}

impl Plugin for TextDiff {
    fn id(&self) -> &str { "text-diff" }
    fn name(&self) -> &str { "文本比对" }
    fn description(&self) -> &str { "实时文本比对工具，支持差异高亮、文件导入导出、差异导航" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "📝" }
    fn get_view(&self) -> String { "<div>插件前端资源加载中...</div>".to_string() }

    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "load_text_file" => {
                let file_path = params.get("file_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 file_path 参数"))?;
                let result = Self::load_text_file_impl(file_path)?;
                Ok(serde_json::to_value(result)?)
            }

            "save_text_file" => {
                let file_path = params.get("file_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 file_path 参数"))?;
                let content = params.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 content 参数"))?;
                Self::save_text_file_impl(file_path, content)?;
                Ok(serde_json::json!({ "success": true }))
            }

            "preprocess_text" => {
                let text = params.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 text 参数"))?;
                // `unwrap_or(false)` 提供默认值
                let ignore_whitespace = params.get("ignore_whitespace")
                    .and_then(|v| v.as_bool()).unwrap_or(false);
                let ignore_case = params.get("ignore_case")
                    .and_then(|v| v.as_bool()).unwrap_or(false);

                let options = ProcessOptions { ignore_whitespace, ignore_case };
                let processed = Self::preprocess_text_impl(text, &options);
                Ok(serde_json::json!({ "original": text, "processed": processed }))
            }

            "count_diff" => {
                let original = params.get("original")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 original 参数"))?;
                let modified = params.get("modified")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 modified 参数"))?;
                let stats = Self::count_diff_lines(original, modified);
                Ok(serde_json::to_value(stats)?)
            }

            "export_diff" => {
                let original = params.get("original")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 original 参数"))?;
                let modified = params.get("modified")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 modified 参数"))?;
                let filename = params.get("filename")
                    .and_then(|v| v.as_str()).unwrap_or("changes.diff");
                let diff = Self::export_unified_diff(original, modified, filename);
                Ok(serde_json::json!({ "diff": diff }))
            }

            _ => Err(format!("未知方法: {}", method).into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(TextDiff));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
