//! # 日志系统
//!
//! 三层日志架构，使用 `tracing` 生态：
//!
//! | 层 | 输出位置 | 用途 | 特性 |
//! |---|---|---|---|
//! | fmt::layer (stdout) | 控制台 | 开发调试 | ANSI 颜色 |
//! | fmt::layer (file) | `~/.worktools/logs/` | 持久化 | 按天滚动，无颜色 |
//! | LogRingLayer | `LOG_RING` 内存缓冲 | 前端查询 | 最多 1000 条，环形覆盖 |
//!
//! ## Rust 知识点: tracing 架构
//! - `tracing`: Rust 的结构化日志框架，类似于 Java 的 SLF4J
//! - `tracing_subscriber`: 日志的"订阅者"，决定日志输出到哪里
//! - `Layer`: tracing_subscriber 的核心抽象，每个 Layer 处理日志的一种方式
//! - `registry()`: 注册中心，管理所有 Layer
//! - `Subscriber` vs `Layer`: Subscriber 是最终接收者，Layer 是中间处理器
//!
//! ## 为什么用 tracing 而不是 log？
//! 1. **结构化日志**: 可以记录键值对（如 `plugin_id = "xxx"`），而不仅是字符串
//! 2. **异步友好**: 与 tokio 集成，支持 span（追踪跨异步任务的请求）
//! 3. **分层处理**: 同一份日志可以输出到多个目标，各层独立配置

use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Mutex;
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

/// 单条日志条目，可序列化为 JSON 返回给前端
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String, // RFC 3339 格式时间戳，精确到毫秒
    pub level: String,     // 日志级别: TRACE, DEBUG, INFO, WARN, ERROR
    pub target: String,    // 日志来源模块，如 "work_tools::plugin_manager"
    pub message: String,   // 日志内容（结构化字段已拼接）
}

/// 环形缓冲区最大容量
const MAX_LOG_ENTRIES: usize = 1000;

/// 全局日志环形缓冲区
///
/// ## Rust 知识点: 静态变量
/// `pub static LOG_RING: Mutex<VecDeque<LogEntry>>`:
/// - `static`: 全局静态变量，整个程序生命周期内有效
/// - `Mutex<VecDeque>`: 线程安全的双向队列（VecDeque 支持高效的首尾操作）
/// - `Mutex::new(VecDeque::new())`: const 初始化（必须是常量表达式）
///
/// ## 为什么用 VecDeque 而不是 Vec？
/// 环形缓冲区需要从头部移除旧元素（`pop_front`），
/// VecDeque 的两端操作都是 O(1)，而 Vec 的 `remove(0)` 是 O(n)。
pub static LOG_RING: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());

/// 自定义日志层 — 将日志写入内存环形缓冲区
///
/// ## Rust 知识点: 自定义 Layer
/// `impl<S> Layer<S> for LogRingLayer where S: Subscriber + ...`
/// 为所有满足约束的 Subscriber 实现 Layer trait。
/// 泛型参数 `S` 是 Subscriber 类型（因为我们用了 `registry()`）。
pub struct LogRingLayer;

impl<S> Layer<S> for LogRingLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    /// 当有日志事件发生时被调用
    ///
    /// ## Rust 知识点: Visitor 模式
    /// tracing 使用 Visitor 模式来收集结构化字段。
    /// `event.record(&mut visitor)` 逐个访问事件的字段并调用 visitor 的方法。
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        // `metadata()` 获取事件的元数据：级别、目标模块等
        let metadata = event.metadata();
        let mut visitor = StringVisitor(String::new());
        // `record` 将事件的所有字段传递给 visitor
        event.record(&mut visitor);

        let entry = LogEntry {
            timestamp: Utc::now()
                // 生成 RFC 3339 格式的时间戳，例如 "2024-01-15T10:30:45.123Z"
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level: metadata.level().to_string(),
            target: metadata.target().to_string(),
            message: visitor.0, // 拼接后的消息文本
        };

        // 写入环形缓冲区
        if let Ok(mut ring) = LOG_RING.lock() {
            // 达到容量上限时移除最旧的条目
            if ring.len() >= MAX_LOG_ENTRIES {
                ring.pop_front(); // 移除最旧的
            }
            ring.push_back(entry); // 添加最新的
        }
    }
}

/// 字段访问器 — 将 tracing 的结构化字段拼接为字符串
///
/// ## Rust 知识点: 新类型模式 (Newtype Pattern)
/// `struct StringVisitor(String)` 是对 String 的包装。
/// 这不是继承，而是组合。编译器会将这个包装优化掉（零成本抽象）。
///
/// 使用新类型的原因：我们需要为外部 crate（tracing）的 trait 提供实现。
/// Rust 的孤儿规则（orphan rule）规定：不能为外部类型实现外部 trait。
/// 通过包装 String 创建自己的类型，我们可以自由地为其实现 `tracing::field::Visit`。
struct StringVisitor(String);

/// 辅助宏：记录字段值到 StringVisitor
///
/// ## Rust 知识点: 声明宏
/// `macro_rules!` 是 Rust 的声明宏，在编译时展开。
/// 这里用它来避免重复的字段记录代码。
macro_rules! record_field {
    ($self:expr, $field:expr, $val:expr) => {
        // 跳过 tracing 内部字段（以 "log." 开头）
        if !$field.name().starts_with("log.") {
            // 在已有内容后添加空格分隔
            if !$self.0.is_empty() {
                $self.0.push(' ');
            }
            // 追加字段值
            $self.0.push_str(&$val.to_string());
        }
    };
}

/// 实现 Visit trait — 使 StringVisitor 能够接收 tracing 的字段
///
/// ## Rust 知识点: trait 实现
/// `impl tracing::field::Visit for StringVisitor`
/// Visit trait 有多个方法，对应不同的字段类型（str, Debug, u64, i64, bool）。
/// 每个方法提供了访问一个字段值的机会。
impl tracing::field::Visit for StringVisitor {
    /// 字符串字段（最常见）
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        record_field!(self, field, value);
    }

    /// Debug 格式字段（其他类型都走这个）
    /// `dyn std::fmt::Debug` 是 trait 对象，表示任何实现了 Debug 的类型
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if !field.name().starts_with("log.") {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            // `use std::fmt::Write` 导入 trait，使 String 支持 `write!` 宏
            // 这与 `std::io::Write` 不同 — 这是格式化字符串专用的 Write
            use std::fmt::Write;
            write!(self.0, "{:?}", value).ok();
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        record_field!(self, field, value);
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        record_field!(self, field, value);
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        record_field!(self, field, value);
    }
}

/// 初始化日志系统
///
/// 配置三层日志架构：
/// 1. 控制台输出（带 ANSI 颜色，无 target 前缀）
/// 2. 文件输出（无颜色，带 target 前缀，按天滚动）
/// 3. 环形缓冲区（供前端查询）
///
/// ## Rust 知识点: tracing_subscriber 的流式 API
/// `registry().with(a).with(b).with(c).init()` 是建造者模式的变体：
/// - `registry()` 创建注册中心
/// - `with(layer)` 添加一个层
/// - `init()` 完成初始化，设置全局订阅者
///
/// ## 关于 `Box::leak`
/// `Box::leak(Box::new(guard))` 故意"泄漏"内存。
/// `guard` 是 `tracing_appender` 的 WorkerGuard，如果不泄漏它会被 drop，
/// 导致文件写入 worker 被停止。因为我们希望它在整个程序生命周期内运行，所以故意泄漏。
///
/// 这不是真正的内存泄漏（只泄一次，可以忽略），是 tracing_appender 的常见用法。
pub fn init_logging() -> Result<()> {
    let log_dir = crate::paths::logs_dir()?;
    std::fs::create_dir_all(&log_dir)?;

    // 创建按天滚动的文件 appender
    // `tracing_appender::rolling::daily(dir, prefix)` 每天创建新文件
    // 文件名格式: prefix.YYYY-MM-DD
    let file_appender = tracing_appender::rolling::daily(&log_dir, "work-tools.log");

    // 使用非阻塞写入：日志不会阻塞主线程
    // `non_blocking` 返回 (writer, guard)
    // guard 必须保持存活（被泄漏），否则后台线程会停止
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);
    Box::leak(Box::new(guard)); // 故意泄漏 guard，保持后台线程运行

    tracing_subscriber::registry()
        // ── 层1: 控制台输出 ──
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)   // 输出到标准输出
                .with_target(false)              // 不显示模块路径（减少噪音）
                .with_level(true)                // 显示日志级别
                .with_ansi(true),                // ANSI 颜色（终端支持时）
        )
        // ── 层2: 文件输出 ──
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking_file)  // 输出到文件
                .with_ansi(false)                // 文件中不需要颜色
                .with_target(true)               // 显示模块路径（便于排查）
                .with_level(true),               // 显示日志级别
        )
        // ── 层3: 环形缓冲区 ──
        .with(LogRingLayer)
        // ── 过滤器 ──
        .with(
            tracing_subscriber::filter::Targets::new()
                .with_default(tracing::Level::DEBUG)         // 默认级别: DEBUG
                .with_target("winit", tracing::Level::ERROR) // winit 太吵，只显示错误
                .with_target("tao", tracing::Level::ERROR),  // tao 同理
        )
        .init(); // 设置全局订阅者

    Ok(())
}
