use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Mutex;
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

/// 日志条目（序列化给前端）
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

const MAX_LOG_ENTRIES: usize = 1000;

/// 全局环形缓冲区，存储最近 N 条日志
pub static LOG_RING: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());

// ── 自定义 Layer：将事件写入 LOG_RING ──

pub struct LogRingLayer;

impl<S> Layer<S> for LogRingLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = StringVisitor(String::new());
        event.record(&mut visitor);

        let entry = LogEntry {
            timestamp: Utc::now()
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level: metadata.level().to_string(),
            target: metadata.target().to_string(),
            message: visitor.0,
        };

        if let Ok(mut ring) = LOG_RING.lock() {
            if ring.len() >= MAX_LOG_ENTRIES {
                ring.pop_front();
            }
            ring.push_back(entry);
        }
    }
}

// ── Visitor：收集事件字段值 ──

struct StringVisitor(String);

impl tracing::field::Visit for StringVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if !field.name().starts_with("log.") {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            self.0.push_str(value);
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if !field.name().starts_with("log.") {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            use std::fmt::Write;
            write!(self.0, "{:?}", value).ok();
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if !field.name().starts_with("log.") {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            self.0.push_str(&value.to_string());
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if !field.name().starts_with("log.") {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            self.0.push_str(&value.to_string());
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if !field.name().starts_with("log.") {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            self.0.push_str(&value.to_string());
        }
    }
}

// ── 初始化 ──

/// 初始化日志系统：stdout + 文件滚动 + 内存环形缓冲
pub fn init_logging() -> Result<()> {
    let user_dirs =
        directories::UserDirs::new().ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;
    let log_dir = user_dirs.home_dir().join(".worktools/logs");

    std::fs::create_dir_all(&log_dir)?;

    // 按天滚动的文件 writer
    let file_appender = tracing_appender::rolling::daily(&log_dir, "work-tools.log");
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    // 泄漏 guard 以保持文件 writer 存活（程序生命周期内有效）
    Box::leak(Box::new(guard));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_target(false)
                .with_level(true)
                .with_ansi(true),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking_file)
                .with_ansi(false)
                .with_target(true)
                .with_level(true),
        )
        .with(LogRingLayer)
        .with(
            tracing_subscriber::filter::Targets::new()
                .with_default(tracing::Level::DEBUG)
                .with_target("winit", tracing::Level::ERROR)
                .with_target("tao", tracing::Level::ERROR),
        )
        .init();

    Ok(())
}
