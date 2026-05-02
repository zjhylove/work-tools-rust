# Developer Tools Plugins 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** 创建 3 个独立 cdylib 插件 — timestamp-converter、cron-tools、redis-client

**Architecture:** 每个插件复用现有模式：Rust `Plugin` trait + `plugin_create` FFI 导出 + React/Vite 前端构建到 assets/。前端通过 `window.pluginAPI.call()` 调用 Rust 后端方法。

**Tech Stack:** Rust (serde_json, chrono, chrono-tz, cron, redis crates) | React 19 + TypeScript + Vite 6

**执行顺序:** timestamp-converter → cron-tools → redis-client (复杂度递增)

---

## 插件 1: timestamp-converter

### Task 1.1: 创建插件骨架 (Cargo + manifest)

**Files:**
- Create: `plugins/timestamp-converter/Cargo.toml`
- Create: `plugins/timestamp-converter/manifest.json`

- [ ] **Step 1: Write Cargo.toml**

File: `plugins/timestamp-converter/Cargo.toml`
```toml
[package]
name = "timestamp-converter"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
tracing = "0.1"
chrono = { version = "0.4", features = ["serde"] }
chrono-tz = "0.10"
```

- [ ] **Step 2: Write manifest.json**

File: `plugins/timestamp-converter/manifest.json`
```json
{
  "id": "timestamp-converter",
  "name": "时间戳转换",
  "description": "Unix时间戳与日期时间互相转换，支持多时区、批量处理",
  "version": "1.0.0",
  "icon": "⏰",
  "author": "Work Tools Team",
  "files": {
    "macos": "libtimestamp_converter.dylib",
    "linux": "libtimestamp_converter.so",
    "windows": "timestamp_converter.dll"
  },
  "assets": {
    "entry": "index.html"
  },
  "permissions": []
}
```

- [ ] **Step 3: Verify**

Run: `cargo check -p timestamp-converter`
Expected: compilation succeeds (will fail until lib.rs exists, this is fine as a skeleton check)

---

### Task 1.2: 实现 Rust 后端

**Files:**
- Create: `plugins/timestamp-converter/src/lib.rs`

- [ ] **Step 1: Write lib.rs**

File: `plugins/timestamp-converter/src/lib.rs`
```rust
use anyhow::Context;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::Value;
use worktools_plugin_api::Plugin;

pub struct TimestampConverter;

/// 自动识别时间戳位数：10位=秒, 13位=毫秒, 16位=微秒
fn parse_timestamp(ts_str: &str) -> anyhow::Result<(i64, &str)> {
    let ts_str = ts_str.trim();
    let (ts, unit) = match ts_str.len() {
        10 => (ts_str.parse::<i64>().context("时间戳解析失败")?, "秒"),
        13 => (ts_str.parse::<i64>().context("时间戳解析失败")? / 1000, "毫秒"),
        16 => (ts_str.parse::<i64>().context("时间戳解析失败")? / 1_000_000, "微秒"),
        _ => return Err(anyhow::anyhow!("无法识别时间戳格式，请输入10/13/16位数字")),
    };
    Ok((ts, unit))
}

fn parse_timezone(tz_str: Option<&str>) -> Tz {
    tz_str
        .and_then(|s| s.parse::<Tz>().ok())
        .unwrap_or_else(|| chrono_tz::Asia::Shanghai)
}

fn format_datetimes(ts_sec: i64, tz: Tz) -> Value {
    let utc_dt = match Utc.timestamp_opt(ts_sec, 0) {
        chrono::LocalResult::Single(dt) => dt,
        _ => return serde_json::json!({ "error": "时间戳超出范围" }),
    };
    let local_dt = utc_dt.with_timezone(&tz);
    serde_json::json!({
        "utc": utc_dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        "datetime": local_dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        "timezone": tz.name(),
        "format_iso": local_dt.to_rfc3339(),
        "format_rfc2822": local_dt.to_rfc2822(),
    })
}

impl Plugin for TimestampConverter {
    fn id(&self) -> &str { "timestamp-converter" }
    fn name(&self) -> &str { "时间戳转换" }
    fn description(&self) -> &str { "Unix时间戳与日期时间互相转换，多时区、批量处理" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "⏰" }
    fn get_view(&self) -> String {
        "<div>插件资源加载中...</div>".to_string()
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "timestamp_to_datetime" => {
                let ts_str = params.get("ts")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 ts 参数")?;
                let tz_str = params.get("timezone").and_then(|v| v.as_str());
                let (ts_sec, _unit) = parse_timestamp(ts_str)?;
                let tz = parse_timezone(tz_str);
                Ok(format_datetimes(ts_sec, tz))
            }

            "datetime_to_timestamp" => {
                let dt_str = params.get("datetime")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 datetime 参数")?;
                let tz_str = params.get("timezone").and_then(|v| v.as_str());
                let tz = parse_timezone(tz_str);

                // 尝试多种格式解析
                let ts_sec = if let Ok(dt) = DateTime::parse_from_rfc3339(dt_str) {
                    dt.timestamp()
                } else if let Ok(dt) = DateTime::parse_from_rfc2822(dt_str) {
                    dt.timestamp()
                } else if let Ok(naive) = NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%d %H:%M:%S") {
                    tz.from_local_datetime(&naive).single()
                        .ok_or("无效日期时间")?
                        .timestamp()
                } else if let Ok(naive) = NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%d") {
                    tz.from_local_datetime(&naive).single()
                        .ok_or("无效日期")?
                        .timestamp()
                } else {
                    return Err("无法解析日期格式，支持: ISO 8601 / RFC 2822 / YYYY-MM-DD HH:MM:SS / YYYY-MM-DD".into());
                };

                Ok(serde_json::json!({
                    "ts_sec": ts_sec,
                    "ts_ms": ts_sec * 1000i64,
                    "ts_us": ts_sec * 1_000_000i64,
                }))
            }

            "current_time" => {
                let tz_str = params.get("timezone").and_then(|v| v.as_str());
                let tz = parse_timezone(tz_str);
                let now = Utc::now();
                let ts_sec = now.timestamp();
                let ts_ms = now.timestamp_millis();
                let mut result = format_datetimes(ts_sec, tz);
                if let Some(obj) = result.as_object_mut() {
                    obj.insert("ts_sec".into(), serde_json::json!(ts_sec));
                    obj.insert("ts_ms".into(), serde_json::json!(ts_ms));
                }
                Ok(result)
            }

            "batch_convert" => {
                let items = params.get("items")
                    .and_then(|v| v.as_array())
                    .ok_or("缺少 items 数组")?;
                let tz_str = params.get("timezone").and_then(|v| v.as_str());
                let tz = parse_timezone(tz_str);

                let results: Vec<Value> = items.iter().map(|item| {
                    let value = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
                    let direction = item.get("direction").and_then(|v| v.as_str()).unwrap_or("to_datetime");

                    if direction == "to_datetime" {
                        match parse_timestamp(value) {
                            Ok((ts_sec, _)) => format_datetimes(ts_sec, tz),
                            Err(e) => serde_json::json!({ "input": value, "error": e.to_string() }),
                        }
                    } else {
                        // reuse datetime_to_timestamp logic inline for batch
                        let ts_sec = if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
                            dt.timestamp()
                        } else if let Ok(dt) = DateTime::parse_from_rfc2822(value) {
                            dt.timestamp()
                        } else if let Ok(naive) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
                            tz.from_local_datetime(&naive).single().map(|d| d.timestamp())
                        } else {
                            None
                        };

                        match ts_sec {
                            Some(ts) => serde_json::json!({ "input": value, "ts_sec": ts, "ts_ms": ts * 1000i64 }),
                            None => serde_json::json!({ "input": value, "error": "无法解析" }),
                        }
                    }
                }).collect();

                Ok(serde_json::json!({ "results": results }))
            }

            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(TimestampConverter));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
```

- [ ] **Step 2: Verify Rust compilation**

Run: `cargo check -p timestamp-converter`
Expected: success (green, no errors)

---

### Task 1.3: 创建前端项目骨架

**Files:**
- Create: `plugins/timestamp-converter/frontend/package.json`
- Create: `plugins/timestamp-converter/frontend/tsconfig.json`
- Create: `plugins/timestamp-converter/frontend/tsconfig.node.json`
- Create: `plugins/timestamp-converter/frontend/vite.config.ts`
- Create: `plugins/timestamp-converter/frontend/index.html`

- [ ] **Step 1: Write package.json**

File: `plugins/timestamp-converter/frontend/package.json`
```json
{
  "name": "timestamp-converter-frontend",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "npx tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@vitejs/plugin-react": "^4.0.0",
    "typescript": "^5.0.0",
    "vite": "^4.3.0"
  }
}
```

- [ ] **Step 2: Write tsconfig.json**

File: `plugins/timestamp-converter/frontend/tsconfig.json`
```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": false,
    "noUnusedParameters": false,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

- [ ] **Step 3: Write tsconfig.node.json**

File: `plugins/timestamp-converter/frontend/tsconfig.node.json`
```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 4: Write vite.config.ts**

File: `plugins/timestamp-converter/frontend/vite.config.ts`
```ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  base: './',
  build: {
    outDir: '../assets',
    emptyOutDir: true,
    minify: 'esbuild',
    sourcemap: false,
    rollupOptions: {
      output: {
        entryFileNames: 'main.js',
        chunkFileNames: 'chunks/[name].js',
        assetFileNames: (assetInfo) => {
          if (assetInfo.name === 'index.html') return 'index.html';
          if (assetInfo.name?.endsWith('.css')) return 'styles.css';
          return 'assets/[name][extname]';
        }
      }
    }
  },
  esbuild: {
    logOverride: { 'this-is-undefined-in-esm': 'silent' }
  },
  optimizeDeps: {
    include: ['react', 'react-dom']
  }
});
```

- [ ] **Step 5: Write index.html**

File: `plugins/timestamp-converter/frontend/index.html`
```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>时间戳转换</title>
    <style>
      html, body { height: 100%; margin: 0; padding: 0; overflow: hidden; }
      #root { height: 100%; overflow: hidden; }
    </style>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 6: Install dependencies**

Run: `cd plugins/timestamp-converter/frontend && npm install`

---

### Task 1.4: 实现前端 (React)

**Files:**
- Create: `plugins/timestamp-converter/frontend/src/main.tsx`
- Create: `plugins/timestamp-converter/frontend/src/App.tsx`
- Create: `plugins/timestamp-converter/frontend/src/App.css`

- [ ] **Step 1: Write main.tsx**

File: `plugins/timestamp-converter/frontend/src/main.tsx`
```tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './App.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

- [ ] **Step 2: Write App.tsx**

File: `plugins/timestamp-converter/frontend/src/App.tsx`
```tsx
import { useState, useEffect, useCallback, useRef } from 'react';
import './App.css';

declare global {
  interface Window {
    pluginAPI?: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

const TIMEZONES = [
  { label: 'UTC+8 上海', value: 'Asia/Shanghai' },
  { label: 'UTC+9 东京', value: 'Asia/Tokyo' },
  { label: 'UTC+0 伦敦', value: 'Europe/London' },
  { label: 'UTC-5 纽约', value: 'America/New_York' },
  { label: 'UTC-8 洛杉矶', value: 'America/Los_Angeles' },
  { label: 'UTC', value: 'UTC' },
];

function App() {
  const [currentTime, setCurrentTime] = useState({ ts_sec: 0, ts_ms: 0, datetime: '', utc: '' });
  const [timezone, setTimezone] = useState('Asia/Shanghai');
  const [tsInput, setTsInput] = useState('');
  const [tsResult, setTsResult] = useState<Record<string, string> | null>(null);
  const [dtInput, setDtInput] = useState('');
  const [dtResult, setDtResult] = useState<Record<string, number> | null>(null);
  const [batchInput, setBatchInput] = useState('');
  const [batchResults, setBatchResults] = useState<Array<Record<string, string>>>([]);
  const [activeTab, setActiveTab] = useState<'ts2dt' | 'dt2ts' | 'batch'>('ts2dt');
  const [error, setError] = useState('');
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  const clearError = () => setError('');

  // 每秒刷新当前时间
  useEffect(() => {
    const tick = async () => {
      try {
        const result = await window.pluginAPI?.call('timestamp-converter', 'current_time', { timezone });
        if (mountedRef.current && result) {
          setCurrentTime(result as typeof currentTime);
        }
      } catch { /* ignore */ }
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [timezone]);

  const handleTsToDt = useCallback(async () => {
    clearError();
    try {
      const result = await window.pluginAPI?.call('timestamp-converter', 'timestamp_to_datetime', { ts: tsInput, timezone });
      setTsResult((result as Record<string, string>) || null);
    } catch (e) {
      setError(String(e));
      setTsResult(null);
    }
  }, [tsInput, timezone]);

  const handleDtToTs = useCallback(async () => {
    clearError();
    try {
      const result = await window.pluginAPI?.call('timestamp-converter', 'datetime_to_timestamp', { datetime: dtInput, timezone });
      setDtResult((result as Record<string, number>) || null);
    } catch (e) {
      setError(String(e));
      setDtResult(null);
    }
  }, [dtInput, timezone]);

  const handleBatchConvert = useCallback(async () => {
    clearError();
    const lines = batchInput.split('\n').filter(l => l.trim());
    const items = lines.map(line => {
      const isNumeric = /^\d+$/.test(line.trim());
      return { value: line.trim(), direction: isNumeric ? 'to_datetime' : 'to_timestamp' };
    });
    try {
      const result = await window.pluginAPI?.call('timestamp-converter', 'batch_convert', { items, timezone });
      if (result && typeof result === 'object' && 'results' in result) {
        setBatchResults((result as { results: Array<Record<string, string>> }).results);
      }
    } catch (e) {
      setError(String(e));
      setBatchResults([]);
    }
  }, [batchInput, timezone]);

  return (
    <div className="ts-converter">
      {/* 当前时间 */}
      <div className="current-time-bar">
        <span className="time-label">当前时间</span>
        <span className="time-datetime">{currentTime.datetime}</span>
        <span className="time-ts">Unix: {currentTime.ts_sec}</span>
      </div>

      {/* 时区选择 */}
      <div className="timezone-row">
        <label>时区:</label>
        <select value={timezone} onChange={e => setTimezone(e.target.value)}>
          {TIMEZONES.map(tz => (
            <option key={tz.value} value={tz.value}>{tz.label}</option>
          ))}
        </select>
      </div>

      {/* Tab 切换 */}
      <div className="tabs">
        <button className={`tab ${activeTab === 'ts2dt' ? 'active' : ''}`} onClick={() => setActiveTab('ts2dt')}>时间戳 → 日期</button>
        <button className={`tab ${activeTab === 'dt2ts' ? 'active' : ''}`} onClick={() => setActiveTab('dt2ts')}>日期 → 时间戳</button>
        <button className={`tab ${activeTab === 'batch' ? 'active' : ''}`} onClick={() => setActiveTab('batch')}>批量转换</button>
      </div>

      <div className="tab-content">
        {activeTab === 'ts2dt' && (
          <div className="convert-panel">
            <div className="input-row">
              <input
                type="text"
                value={tsInput}
                onChange={e => setTsInput(e.target.value)}
                placeholder="输入时间戳 (10位秒 / 13位毫秒 / 16位微秒)"
                onKeyDown={e => e.key === 'Enter' && handleTsToDt()}
              />
              <button className="btn-primary" onClick={handleTsToDt}>转换</button>
            </div>
            {tsResult && (
              <div className="result-box">
                <div className="result-row"><span>ISO 8601:</span><code>{tsResult.format_iso}</code></div>
                <div className="result-row"><span>RFC 2822:</span><code>{tsResult.format_rfc2822}</code></div>
                <div className="result-row"><span>{tsResult.timezone}:</span><code>{tsResult.datetime}</code></div>
                <div className="result-row"><span>UTC:</span><code>{tsResult.utc}</code></div>
              </div>
            )}
          </div>
        )}

        {activeTab === 'dt2ts' && (
          <div className="convert-panel">
            <div className="input-row">
              <input
                type="text"
                value={dtInput}
                onChange={e => setDtInput(e.target.value)}
                placeholder="输入日期 (ISO 8601 / RFC 2822 / YYYY-MM-DD HH:MM:SS)"
                onKeyDown={e => e.key === 'Enter' && handleDtToTs()}
              />
              <button className="btn-primary" onClick={handleDtToTs}>转换</button>
            </div>
            {dtResult && (
              <div className="result-box">
                <div className="result-row"><span>秒:</span><code>{dtResult.ts_sec}</code></div>
                <div className="result-row"><span>毫秒:</span><code>{dtResult.ts_ms}</code></div>
                <div className="result-row"><span>微秒:</span><code>{dtResult.ts_us}</code></div>
              </div>
            )}
          </div>
        )}

        {activeTab === 'batch' && (
          <div className="convert-panel">
            <textarea
              value={batchInput}
              onChange={e => setBatchInput(e.target.value)}
              placeholder={'每行一个值，自动识别格式:\n1756193728\n2026-05-02T17:35:28+08:00\n1714608000000'}
              rows={8}
            />
            <button className="btn-primary" onClick={handleBatchConvert} style={{ marginTop: 12 }}>批量转换</button>
            {batchResults.length > 0 && (
              <div className="batch-results">
                <table>
                  <thead><tr><th>输入</th><th>结果</th></tr></thead>
                  <tbody>
                    {batchResults.map((r, i) => (
                      <tr key={i}>
                        <td>{r.input}</td>
                        <td>{r.error || r.datetime || `${r.ts_sec} (秒) / ${r.ts_ms} (毫秒)`}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        )}
      </div>

      {error && <div className="error-toast">{error}</div>}
    </div>
  );
}

export default App;
```

- [ ] **Step 3: Write App.css**

File: `plugins/timestamp-converter/frontend/src/App.css`
```css
.ts-converter {
  flex: 1;
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 20px;
  background: var(--bg-primary);
}

.current-time-bar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 16px;
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
  margin-bottom: 16px;
  font-size: 14px;
  flex-shrink: 0;
}

.time-label {
  font-weight: 600;
  color: var(--text-secondary);
}

.time-datetime {
  font-family: 'Monaco', 'Menlo', monospace;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.time-ts {
  font-family: 'Monaco', 'Menlo', monospace;
  color: var(--text-secondary);
  margin-left: auto;
}

.timezone-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
  flex-shrink: 0;
}

.timezone-row label {
  font-size: 13px;
  color: var(--text-secondary);
}

.timezone-row select {
  padding: 6px 12px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
}

.tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 16px;
  flex-shrink: 0;
}

.tab {
  padding: 8px 16px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
}

.tab:hover { background: var(--hover-bg); }

.tab.active {
  background: var(--accent);
  color: var(--text-inverse);
  border-color: var(--accent);
}

.tab-content {
  flex: 1;
  overflow: auto;
}

.convert-panel {
  display: flex;
  flex-direction: column;
}

.input-row {
  display: flex;
  gap: 12px;
}

.input-row input {
  flex: 1;
  padding: 10px 14px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-size: 14px;
  font-family: 'Monaco', 'Menlo', monospace;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.input-row input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-ring);
}

.btn-primary {
  padding: 10px 20px;
  background: var(--accent);
  color: var(--text-inverse);
  border: none;
  border-radius: var(--radius-sm);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.2s;
}

.btn-primary:hover {
  background: var(--accent-hover);
}

textarea {
  width: 100%;
  padding: 10px 14px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-size: 14px;
  font-family: 'Monaco', 'Menlo', monospace;
  background: var(--bg-primary);
  color: var(--text-primary);
  resize: vertical;
  box-sizing: border-box;
}

textarea:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-ring);
}

.result-box {
  margin-top: 16px;
  padding: 16px;
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
}

.result-row {
  display: flex;
  gap: 12px;
  padding: 6px 0;
  font-size: 13px;
}

.result-row span {
  color: var(--text-secondary);
  min-width: 80px;
  flex-shrink: 0;
}

.result-row code {
  font-family: 'Monaco', 'Menlo', monospace;
  font-size: 13px;
  color: var(--text-primary);
}

.batch-results {
  margin-top: 16px;
  overflow: auto;
}

.batch-results table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.batch-results th {
  text-align: left;
  padding: 8px 12px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
  font-weight: 600;
  border-bottom: 2px solid var(--border-color);
}

.batch-results td {
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-light);
  font-family: 'Monaco', 'Menlo', monospace;
  color: var(--text-primary);
}

.error-toast {
  position: fixed;
  top: 16px;
  left: 50%;
  transform: translateX(-50%);
  padding: 10px 20px;
  background: var(--error-light);
  border: 1px solid var(--error-border);
  border-radius: var(--radius-md);
  color: var(--error-text);
  font-size: 13px;
  z-index: 1000;
  animation: slideDown 0.3s ease;
}

@keyframes slideDown {
  from { opacity: 0; transform: translateX(-50%) translateY(-10px); }
  to { opacity: 1; transform: translateX(-50%) translateY(0); }
}
```

- [ ] **Step 4: Build 前端**

Run: `cd plugins/timestamp-converter/frontend && npm run build`
Expected: build succeeds, `assets/` populated with index.html/main.js/styles.css

- [ ] **Step 5: Verify full compilation**

Run: `cargo check -p timestamp-converter`
Expected: success

---

### Task 1.5: 功能验证

- [ ] **Step 1: 验证 Rust 时间戳转换**

Run: `cargo test -p timestamp-converter 2>/dev/null || echo "No tests yet — will verify via integration"`
Manual check: This plugin has no unit tests initially; verify via cargo check success + frontend build.

---

## 插件 2: cron-tools

### Task 2.1: 创建插件骨架

**Files:**
- Create: `plugins/cron-tools/Cargo.toml`
- Create: `plugins/cron-tools/manifest.json`

- [ ] **Step 1: Write Cargo.toml**

File: `plugins/cron-tools/Cargo.toml`
```toml
[package]
name = "cron-tools"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
tracing = "0.1"
cron = "0.14"
chrono = "0.4"
```

- [ ] **Step 2: Write manifest.json**

File: `plugins/cron-tools/manifest.json`
```json
{
  "id": "cron-tools",
  "name": "Cron 表达式",
  "description": "Cron表达式解析、人类可读描述、下次执行时间预览、可视化构建",
  "version": "1.0.0",
  "icon": "⏱",
  "author": "Work Tools Team",
  "files": {
    "macos": "libcron_tools.dylib",
    "linux": "libcron_tools.so",
    "windows": "cron_tools.dll"
  },
  "assets": {
    "entry": "index.html"
  },
  "permissions": []
}
```

---

### Task 2.2: 实现 Rust 后端

**Files:**
- Create: `plugins/cron-tools/src/lib.rs`

- [ ] **Step 1: Write lib.rs**

File: `plugins/cron-tools/src/lib.rs`
```rust
use anyhow::Context;
use chrono::{DateTime, Utc};
use cron::Schedule;
use serde_json::Value;
use std::str::FromStr;
use worktools_plugin_api::Plugin;

pub struct CronTools;

const STANDARD_FIELDS: [(&str, &str); 5] = [
    ("minute", "分钟"),
    ("hour", "小时"),
    ("day_of_month", "日"),
    ("month", "月"),
    ("day_of_week", "周"),
];

/// 生成表达式中单个字段的中文描述
fn describe_field(value: &str, field_name: &str) -> String {
    if value == "*" {
        return format!("每{}", field_name);
    }
    if value.contains('/') {
        let parts: Vec<&str> = value.split('/').collect();
        if parts.len() == 2 {
            let base = if parts[0] == "*" { "每".to_string() } else { format!("从第{}", parts[0]) };
            return format!("{}{}{}执行", base, field_name, match parts[1] {
                "1" => "".to_string(),
                n => format!("间隔{}", n),
            });
        }
    }
    if value.contains(',') {
        let nums: Vec<&str> = value.split(',').collect();
        return format!("{}的第{}", field_name, nums.join("、"));
    }
    if value.contains('-') {
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() == 2 {
            return format!("{}从{}到{}", field_name, parts[0], parts[1]);
        }
    }
    format!("{}为{}", field_name, value)
}

/// 生成完整 cron 表达式的人类可读描述
fn describe_cron(expr: &str) -> String {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return "无效的 cron 表达式（需要5个字段）".to_string();
    }

    let parts: Vec<String> = fields.iter().enumerate().map(|(i, f)| {
        describe_field(f, STANDARD_FIELDS[i].1)
    }).collect();

    parts.join("，")
}

impl Plugin for CronTools {
    fn id(&self) -> &str { "cron-tools" }
    fn name(&self) -> &str { "Cron 表达式" }
    fn description(&self) -> &str { "Cron表达式解析、人类可读描述、下次执行时间预览" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "⏱" }
    fn get_view(&self) -> String {
        "<div>插件资源加载中...</div>".to_string()
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "parse_cron" => {
                let expr = params.get("expr")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 expr 参数")?;
                let expr = expr.trim();

                if expr.split_whitespace().count() != 5 {
                    return Ok(serde_json::json!({
                        "valid": false,
                        "description": "无效的 cron 表达式（需要5个字段）",
                        "error": "表达式需要5个空格分隔的字段"
                    }));
                }

                match Schedule::from_str(expr) {
                    Ok(_) => Ok(serde_json::json!({
                        "valid": true,
                        "description": describe_cron(expr),
                        "error": null,
                    })),
                    Err(e) => Ok(serde_json::json!({
                        "valid": false,
                        "description": format!("无效表达式: {}", e),
                        "error": e.to_string(),
                    })),
                }
            }

            "next_executions" => {
                let expr = params.get("expr")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 expr 参数")?;
                let count = params.get("count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5) as usize;
                let count = count.min(20);

                let start_time = params.get("start_time")
                    .and_then(|v| v.as_str())
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

                let schedule = Schedule::from_str(expr.trim())
                    .context("cron 表达式解析失败")?;

                let times: Vec<String> = schedule.upcoming(Utc)
                    .take(count)
                    .map(|dt| dt.to_rfc3339())
                    .collect();

                // 如果 upcoming 返回空 (表达式可能已经过期), 使用 after
                let times = if times.is_empty() {
                    schedule.after(&start_time)
                        .take(count)
                        .map(|dt| dt.to_rfc3339())
                        .collect()
                } else {
                    times
                };

                Ok(serde_json::json!({ "times": times }))
            }

            "get_presets" => Ok(serde_json::json!({
                "presets": [
                    { "label": "每分钟", "expr": "* * * * *" },
                    { "label": "每5分钟", "expr": "*/5 * * * *" },
                    { "label": "每15分钟", "expr": "*/15 * * * *" },
                    { "label": "每小时", "expr": "0 * * * *" },
                    { "label": "每天凌晨", "expr": "0 0 * * *" },
                    { "label": "每天上午9点", "expr": "0 9 * * *" },
                    { "label": "工作日上午9点", "expr": "0 9 * * 1-5" },
                    { "label": "每月1号凌晨", "expr": "0 0 1 * *" },
                    { "label": "每周一凌晨", "expr": "0 0 * * 1" },
                ]
            })),

            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(CronTools));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p cron-tools`
Expected: success

---

### Task 2.3: 创建前端项目

**Files:**
- Create: `plugins/cron-tools/frontend/package.json`
- Create: `plugins/cron-tools/frontend/tsconfig.json`
- Create: `plugins/cron-tools/frontend/tsconfig.node.json`
- Create: `plugins/cron-tools/frontend/vite.config.ts`
- Create: `plugins/cron-tools/frontend/index.html`

Prepare these 5 files with the same structure as timestamp-converter but with:
- name: `cron-tools-frontend` in package.json
- title: `Cron 表达式` in index.html

- [ ] **Step 1: Write all 5 skeleton files**

Write these 5 files with the following differences from Task 1.3:
- `package.json`: `"name": "cron-tools-frontend"`
- `index.html`: `<title>Cron 表达式</title>`
- `tsconfig.json`, `tsconfig.node.json`, `vite.config.ts`: identical to Task 1.3, copy verbatim

- [ ] **Step 2: Install dependencies**

Run: `cd plugins/cron-tools/frontend && npm install`

---

### Task 2.4: 实现前端

**Files:**
- Create: `plugins/cron-tools/frontend/src/main.tsx`
- Create: `plugins/cron-tools/frontend/src/App.tsx`
- Create: `plugins/cron-tools/frontend/src/App.css`

- [ ] **Step 1: Write main.tsx**

- [ ] **Step 1: Write main.tsx** (identical to timestamp-converter Task 1.4 Step 1 — copies the standard React entry point)

- [ ] **Step 2: Write App.tsx**

File: `plugins/cron-tools/frontend/src/App.tsx`
```tsx
import { useState, useCallback } from 'react';
import './App.css';

declare global {
  interface Window {
    pluginAPI?: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

interface Preset { label: string; expr: string; }

const FIELD_LABELS = ['分钟', '小时', '日', '月', '周'];

function App() {
  const [expr, setExpr] = useState('*/5 * * * *');
  const [description, setDescription] = useState('');
  const [valid, setValid] = useState(true);
  const [execTimes, setExecTimes] = useState<string[]>([]);
  const [presets, setPresets] = useState<Preset[]>([]);
  const [showBuilder, setShowBuilder] = useState(false);
  const [fields, setFields] = useState(['*', '*', '*', '*', '*']);
  const [error, setError] = useState('');

  const clearError = () => setError('');

  const handleParse = useCallback(async () => {
    clearError();
    try {
      const result = await window.pluginAPI?.call('cron-tools', 'parse_cron', { expr: expr.trim() });
      if (result && typeof result === 'object') {
        const r = result as { valid: boolean; description: string; error: string | null };
        setValid(r.valid);
        setDescription(r.description);
      }
    } catch (e) { setError(String(e)); }
  }, [expr]);

  const handleNextExec = useCallback(async () => {
    try {
      const result = await window.pluginAPI?.call('cron-tools', 'next_executions', { expr: expr.trim(), count: 5 });
      if (result && typeof result === 'object' && 'times' in result) {
        setExecTimes((result as { times: string[] }).times);
      }
    } catch (e) { setError(String(e)); }
  }, [expr]);

  const loadPresets = useCallback(async () => {
    try {
      const result = await window.pluginAPI?.call('cron-tools', 'get_presets', {});
      if (result && typeof result === 'object' && 'presets' in result) {
        setPresets((result as { presets: Preset[] }).presets);
      }
    } catch { /* ignore */ }
  }, []);

  // 当表达式改变时自动解析和获取执行时间
  const handleExprChange = useCallback((newExpr: string) => {
    setExpr(newExpr);
    const parts = newExpr.trim().split(/\s+/);
    if (parts.length === 5) setFields(parts);
  }, []);

  const handleFieldChange = useCallback((index: number, value: string) => {
    const newFields = [...fields];
    newFields[index] = value;
    setFields(newFields);
    setExpr(newFields.join(' '));
  }, [fields]);

  const handlePresetClick = useCallback((preset: Preset) => {
    handleExprChange(preset.expr);
    setTimeout(() => {
      window.pluginAPI?.call('cron-tools', 'parse_cron', { expr: preset.expr }).then(result => {
        if (result && typeof result === 'object') {
          const r = result as { valid: boolean; description: string; error: string | null };
          setValid(r.valid);
          setDescription(r.description);
        }
      });
      window.pluginAPI?.call('cron-tools', 'next_executions', { expr: preset.expr, count: 5 }).then(result => {
        if (result && typeof result === 'object' && 'times' in result) {
          setExecTimes((result as { times: string[] }).times);
        }
      });
    }, 50);
  }, []);

  return (
    <div className="cron-tools">
      {/* 输入区 */}
      <div className="input-section">
        <div className="input-row">
          <input
            type="text"
            value={expr}
            onChange={e => handleExprChange(e.target.value)}
            placeholder="输入 cron 表达式，如 */5 * * * *"
            className={`cron-input ${!valid ? 'invalid' : ''}`}
            onKeyDown={e => e.key === 'Enter' && handleParse()}
          />
          <button className="btn-primary" onClick={() => { handleParse(); handleNextExec(); }}>解析</button>
        </div>

        {description && (
          <div className={`description ${valid ? 'valid' : 'invalid'}`}>
            {description}
          </div>
        )}
      </div>

      {/* 执行时间预览 */}
      {execTimes.length > 0 && (
        <div className="exec-section">
          <h4>下次执行时间</h4>
          <ul>
            {execTimes.map((t, i) => (
              <li key={i}><code>{t}</code></li>
            ))}
          </ul>
        </div>
      )}

      {/* 可视化构建器 */}
      <div className="builder-section">
        <button className="btn-secondary" onClick={() => { setShowBuilder(!showBuilder); if (!showBuilder && presets.length === 0) loadPresets(); }}>
          {showBuilder ? '收起构建器 ▲' : '可视化构建 ▼'}
        </button>

        {showBuilder && (
          <div className="builder-panel">
            <div className="field-grid">
              {fields.map((value, i) => (
                <div key={i} className="field-item">
                  <label>{FIELD_LABELS[i]}</label>
                  <select value={value} onChange={e => handleFieldChange(i, e.target.value)}>
                    <option value="*">* (每{FIELD_LABELS[i]})</option>
                    {i === 0 && [0,5,10,15,20,25,30,35,40,45,50,55].map(n => (
                      <option key={n} value={String(n)}>{n}</option>
                    ))}
                    {i === 1 && Array.from({length: 24}, (_, n) => (
                      <option key={n} value={String(n)}>{n}</option>
                    ))}
                    {i === 2 && Array.from({length: 31}, (_, n) => (
                      <option key={n+1} value={String(n+1)}>{n+1}</option>
                    ))}
                    {i === 3 && Array.from({length: 12}, (_, n) => (
                      <option key={n+1} value={String(n+1)}>{n+1}</option>
                    ))}
                    {i === 4 && [
                      {v: '0', l: '0 (周日)'}, {v: '1', l: '1 (周一)'},
                      {v: '2', l: '2 (周二)'}, {v: '3', l: '3 (周三)'},
                      {v: '4', l: '4 (周四)'}, {v: '5', l: '5 (周五)'},
                      {v: '6', l: '6 (周六)'},
                    ].map(o => <option key={o.v} value={o.v}>{o.l}</option>)}
                  </select>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* 常用预设 */}
      {presets.length > 0 && (
        <div className="presets-section">
          <h4>常用模板</h4>
          <div className="preset-chips">
            {presets.map(p => (
              <button key={p.label} className="chip" onClick={() => handlePresetClick(p)}>
                {p.label}
                <span className="chip-expr">{p.expr}</span>
              </button>
            ))}
          </div>
        </div>
      )}

      {error && <div className="error-toast">{error}</div>}
    </div>
  );
}

export default App;
```

- [ ] **Step 3: Write App.css**

```css
.cron-tools {
  flex: 1;
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  overflow: auto;
  padding: 20px;
  background: var(--bg-primary);
}

.input-section {
  margin-bottom: 20px;
}

.input-row {
  display: flex;
  gap: 12px;
}

.cron-input {
  flex: 1;
  padding: 10px 14px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-size: 18px;
  font-family: 'Monaco', 'Menlo', monospace;
  background: var(--bg-primary);
  color: var(--text-primary);
  letter-spacing: 1px;
}

.cron-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-ring);
}

.cron-input.invalid {
  border-color: var(--error);
}

.btn-primary, .btn-secondary {
  padding: 10px 20px;
  border-radius: var(--radius-sm);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  border: none;
}

.btn-primary {
  background: var(--accent);
  color: var(--text-inverse);
}

.btn-primary:hover { background: var(--accent-hover); }

.btn-secondary {
  background: var(--bg-secondary);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
}

.btn-secondary:hover { background: var(--hover-bg); }

.description {
  margin-top: 12px;
  padding: 12px 16px;
  border-radius: var(--radius-sm);
  font-size: 14px;
  font-weight: 500;
}

.description.valid {
  background: var(--success-light);
  border: 1px solid var(--success-border);
  color: var(--success-text);
}

.description.invalid {
  background: var(--error-light);
  border: 1px solid var(--error-border);
  color: var(--error-text);
}

.exec-section {
  margin-bottom: 20px;
}

.exec-section h4 {
  font-size: 13px;
  color: var(--text-secondary);
  margin: 0 0 8px 0;
}

.exec-section ul {
  list-style: none;
  padding: 0;
  margin: 0;
}

.exec-section li {
  padding: 6px 12px;
  font-size: 13px;
}

.exec-section code {
  font-family: 'Monaco', 'Menlo', monospace;
  color: var(--text-primary);
}

.builder-section {
  margin-bottom: 20px;
}

.builder-panel {
  margin-top: 12px;
  padding: 16px;
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
}

.field-grid {
  display: flex;
  gap: 12px;
}

.field-item {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-item label {
  font-size: 12px;
  color: var(--text-secondary);
  font-weight: 600;
}

.field-item select {
  padding: 8px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
}

.presets-section h4 {
  font-size: 13px;
  color: var(--text-secondary);
  margin: 0 0 8px 0;
}

.preset-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.chip {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding: 8px 14px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.2s;
  font-size: 13px;
  color: var(--text-primary);
}

.chip:hover {
  border-color: var(--accent);
  background: var(--accent-light);
}

.chip-expr {
  font-family: 'Monaco', 'Menlo', monospace;
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

.error-toast {
  position: fixed;
  top: 16px;
  left: 50%;
  transform: translateX(-50%);
  padding: 10px 20px;
  background: var(--error-light);
  border: 1px solid var(--error-border);
  border-radius: var(--radius-md);
  color: var(--error-text);
  font-size: 13px;
  z-index: 1000;
}
```

- [ ] **Step 4: Build frontend**

Run: `cd plugins/cron-tools/frontend && npm run build`
Expected: build succeeds

- [ ] **Step 5: Verify full compilation**

Run: `cargo check -p cron-tools`
Expected: success

---

## 插件 3: redis-client

### Task 3.1: 创建插件骨架

**Files:**
- Create: `plugins/redis-client/Cargo.toml`
- Create: `plugins/redis-client/manifest.json`

- [ ] **Step 1: Write Cargo.toml**

File: `plugins/redis-client/Cargo.toml`
```toml
[package]
name = "redis-client"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
tracing = "0.1"
redis = "0.27"
uuid = { version = "1", features = ["v4"] }
```

- [ ] **Step 2: Write manifest.json**

File: `plugins/redis-client/manifest.json`
```json
{
  "id": "redis-client",
  "name": "Redis 客户端",
  "description": "Redis数据库管理工具，支持Key浏览、String/Hash/List/Set/ZSet操作",
  "version": "1.0.0",
  "icon": "🔴",
  "author": "Work Tools Team",
  "files": {
    "macos": "libredis_client.dylib",
    "linux": "libredis_client.so",
    "windows": "redis_client.dll"
  },
  "assets": {
    "entry": "index.html"
  },
  "permissions": ["network"]
}
```

---

### Task 3.2: 实现 Rust 后端

**Files:**
- Create: `plugins/redis-client/src/lib.rs`

`lib.rs` 分为: 连接管理、Key操作、数据类型操作、工厂函数。

- [ ] **Step 1: Write lib.rs**

File: `plugins/redis-client/src/lib.rs`
```rust
use anyhow::Context;
use redis::{Client, Commands, Connection, ConnectionLike};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use worktools_plugin_api::{Plugin, PluginStorage};

// ── 数据结构 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedConnection {
    id: String,
    name: String,
    host: String,
    port: u16,
    db: i64,
    password_obfuscated: String,
}

#[derive(Debug, Clone)]
struct ConnectionConfig {
    host: String,
    port: u16,
    db: i64,
}

struct StorageData {
    saved_connections: Vec<SavedConnection>,
}

// ── 简单混淆 ──

const XOR_KEY: &[u8] = b"worktools-redis-2026";

fn obfuscate(s: &str) -> String {
    let bytes: Vec<u8> = s.bytes().zip(XOR_KEY.iter().cycle()).map(|(a, b)| a ^ b).collect();
    hex::encode(bytes)
}

fn deobfuscate(s: &str) -> Option<String> {
    let bytes: Vec<u8> = hex::decode(s).ok()?;
    let decoded: Vec<u8> = bytes.iter().zip(XOR_KEY.iter().cycle()).map(|(a, b)| a ^ b).collect();
    String::from_utf8(decoded).ok()
}

// ── 插件主体 ──

pub struct RedisClientPlugin {
    client: Option<Client>,
    current_config: Option<ConnectionConfig>,
    storage: PluginStorage,
    saved_connections: Vec<SavedConnection>,
}

impl RedisClientPlugin {
    fn new() -> Self {
        let mut storage = PluginStorage::new("redis-client");
        let saved_connections = storage
            .get::<Vec<SavedConnection>>("saved_connections")
            .unwrap_or_default();

        Self {
            client: None,
            current_config: None,
            storage,
            saved_connections,
        }
    }

    fn get_conn(&self) -> Result<Connection, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .as_ref()
            .ok_or("未连接到 Redis")?
            .get_connection()
            .map_err(|e| e.into())
    }

    fn persist_connections(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.storage.set("saved_connections", &self.saved_connections)?;
        Ok(())
    }
}

fn key_info(key: &str, conn: &mut Connection) -> Value {
    let key_type: String = redis::cmd("TYPE").arg(key).query(conn).unwrap_or_else(|_| "unknown".into());
    let ttl: i64 = redis::cmd("TTL").arg(key).query(conn).unwrap_or(-2);
    serde_json::json!({
        "key": key,
        "type": key_type,
        "ttl": ttl,
    })
}

impl Plugin for RedisClientPlugin {
    fn id(&self) -> &str { "redis-client" }
    fn name(&self) -> &str { "Redis 客户端" }
    fn description(&self) -> &str { "Redis数据库管理工具" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "🔴" }
    fn get_view(&self) -> String { "<div>插件资源加载中...</div>".to_string() }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            // ── 连接管理 ──
            "connect" => {
                let host = params.get("host").and_then(|v| v.as_str()).unwrap_or("127.0.0.1");
                let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(6379) as u16;
                let db = params.get("db").and_then(|v| v.as_i64()).unwrap_or(0);
                let password = params.get("password").and_then(|v| v.as_str()).filter(|s| !s.is_empty());

                let url = if let Some(pass) = password {
                    format!("redis://:{}@{}:{}/{}", pass, host, port, db)
                } else {
                    format!("redis://{}:{}/{}", host, port, db)
                };

                let client = Client::open(url.as_str())
                    .context("Redis 连接失败")?;

                // 验证连接
                let _: String = client.get_connection()?.ping()?;

                self.client = Some(client);
                self.current_config = Some(ConnectionConfig {
                    host: host.to_string(),
                    port,
                    db,
                });

                tracing::info!(host, port, db, "Redis 连接成功");
                Ok(serde_json::json!({ "ok": true, "host": host, "port": port, "db": db }))
            }

            "disconnect" => {
                self.client = None;
                self.current_config = None;
                Ok(serde_json::json!({ "ok": true }))
            }

            "get_connection_info" => {
                if let Some(ref cfg) = self.current_config {
                    Ok(serde_json::json!({
                        "connected": true,
                        "host": cfg.host,
                        "port": cfg.port,
                        "db": cfg.db,
                    }))
                } else {
                    Ok(serde_json::json!({ "connected": false }))
                }
            }

            "save_connection" => {
                let name = params.get("name").and_then(|v| v.as_str()).ok_or("缺少 name")?;
                let host = params.get("host").and_then(|v| v.as_str()).unwrap_or("127.0.0.1");
                let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(6379) as u16;
                let db = params.get("db").and_then(|v| v.as_i64()).unwrap_or(0);
                let password = params.get("password").and_then(|v| v.as_str()).unwrap_or("");

                let conn = SavedConnection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: name.to_string(),
                    host: host.to_string(),
                    port,
                    db,
                    password_obfuscated: if password.is_empty() { String::new() } else { obfuscate(password) },
                };

                self.saved_connections.push(conn);
                self.persist_connections()?;
                Ok(serde_json::json!({ "ok": true }))
            }

            "list_saved_connections" => {
                let list: Vec<Value> = self.saved_connections.iter().map(|c| {
                    serde_json::json!({
                        "id": c.id,
                        "name": c.name,
                        "host": c.host,
                        "port": c.port,
                        "db": c.db,
                        "has_password": !c.password_obfuscated.is_empty(),
                    })
                }).collect();
                Ok(serde_json::json!({ "connections": list }))
            }

            "delete_saved_connection" => {
                let id = params.get("id").and_then(|v| v.as_str()).ok_or("缺少 id")?;
                self.saved_connections.retain(|c| c.id != id);
                self.persist_connections()?;
                Ok(serde_json::json!({ "ok": true }))
            }

            "get_saved_password" => {
                let id = params.get("id").and_then(|v| v.as_str()).ok_or("缺少 id")?;
                let conn = self.saved_connections.iter().find(|c| c.id == id)
                    .ok_or("连接配置不存在")?;
                if conn.password_obfuscated.is_empty() {
                    Ok(serde_json::json!({ "password": "" }))
                } else {
                    let pass = deobfuscate(&conn.password_obfuscated).unwrap_or_default();
                    Ok(serde_json::json!({ "password": pass }))
                }
            }

            // ── Key 操作 ──
            "scan_keys" => {
                let mut conn = self.get_conn()?;
                let cursor: u64 = params.get("cursor").and_then(|v| v.as_u64()).unwrap_or(0);
                let pattern = params.get("pattern").and_then(|v| v.as_str()).unwrap_or("*");
                let count: usize = params.get("count").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

                let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH").arg(pattern)
                    .arg("COUNT").arg(count)
                    .query(&mut conn)?;

                let key_infos: Vec<Value> = keys.iter().map(|k| key_info(k, &mut conn)).collect();

                Ok(serde_json::json!({
                    "cursor": next_cursor,
                    "keys": key_infos,
                }))
            }

            "get_key_info" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let mut conn = self.get_conn()?;
                let key_type: String = redis::cmd("TYPE").arg(key).query(&mut conn)?;
                let ttl: i64 = redis::cmd("TTL").arg(key).query(&mut conn)?;
                let length: Option<usize> = match key_type.as_str() {
                    "string" => None,
                    "hash" => Some(redis::cmd("HLEN").arg(key).query(&mut conn)?),
                    "list" => Some(redis::cmd("LLEN").arg(key).query(&mut conn)?),
                    "set" => Some(redis::cmd("SCARD").arg(key).query(&mut conn)?),
                    "zset" => Some(redis::cmd("ZCARD").arg(key).query(&mut conn)?),
                    _ => None,
                };
                Ok(serde_json::json!({ "key": key, "type": key_type, "ttl": ttl, "length": length }))
            }

            "delete_key" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let mut conn = self.get_conn()?;
                let deleted: i32 = conn.del(key)?;
                Ok(serde_json::json!({ "deleted": deleted }))
            }

            "rename_key" => {
                let old = params.get("old").and_then(|v| v.as_str()).ok_or("缺少 old")?;
                let new = params.get("new").and_then(|v| v.as_str()).ok_or("缺少 new")?;
                let mut conn = self.get_conn()?;
                redis::cmd("RENAME").arg(old).arg(new).query::<()>(&mut conn)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            "set_ttl" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let seconds = params.get("seconds").and_then(|v| v.as_i64()).ok_or("缺少 seconds")?;
                let mut conn = self.get_conn()?;
                let result: i32 = conn.expire(key, seconds as usize)?;
                Ok(serde_json::json!({ "ok": result == 1 }))
            }

            // ── String ──
            "get_string" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let mut conn = self.get_conn()?;
                let value: String = conn.get(key)?;
                Ok(serde_json::json!({ "value": value }))
            }

            "set_string" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let value = params.get("value").and_then(|v| v.as_str()).ok_or("缺少 value")?;
                let mut conn = self.get_conn()?;
                let _: () = conn.set(key, value)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            // ── Hash ──
            "get_hash" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let mut conn = self.get_conn()?;
                let fields: HashMap<String, String> = conn.hgetall(key)?;
                Ok(serde_json::json!({ "fields": fields }))
            }

            "set_hash_field" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let field = params.get("field").and_then(|v| v.as_str()).ok_or("缺少 field")?;
                let value = params.get("value").and_then(|v| v.as_str()).ok_or("缺少 value")?;
                let mut conn = self.get_conn()?;
                let _: () = conn.hset(key, field, value)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            "del_hash_field" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let field = params.get("field").and_then(|v| v.as_str()).ok_or("缺少 field")?;
                let mut conn = self.get_conn()?;
                let _: () = conn.hdel(key, field)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            // ── List ──
            "get_list" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let start: isize = params.get("start").and_then(|v| v.as_i64()).unwrap_or(0) as isize;
                let stop: isize = params.get("stop").and_then(|v| v.as_i64()).unwrap_or(-1) as isize;
                let mut conn = self.get_conn()?;
                let items: Vec<String> = conn.lrange(key, start, stop)?;
                Ok(serde_json::json!({ "items": items }))
            }

            "lpush" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let value = params.get("value").and_then(|v| v.as_str()).ok_or("缺少 value")?;
                let mut conn = self.get_conn()?;
                let len: i32 = conn.lpush(key, value)?;
                Ok(serde_json::json!({ "length": len }))
            }

            "rpush" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let value = params.get("value").and_then(|v| v.as_str()).ok_or("缺少 value")?;
                let mut conn = self.get_conn()?;
                let len: i32 = conn.rpush(key, value)?;
                Ok(serde_json::json!({ "length": len }))
            }

            "lrem" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let index = params.get("index").and_then(|v| v.as_i64()).ok_or("缺少 index")?;
                let mut conn = self.get_conn()?;
                // LSET index value → use LREM with value at index
                let value: String = conn.lindex(key, index)?;
                let _: i32 = conn.lrem(key, 1, &value)?;
                Ok(serde_json::json!({ "ok": true }))
            }

            // ── Set ──
            "get_set" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let mut conn = self.get_conn()?;
                let members: Vec<String> = conn.smembers(key)?;
                Ok(serde_json::json!({ "members": members }))
            }

            "sadd" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let member = params.get("member").and_then(|v| v.as_str()).ok_or("缺少 member")?;
                let mut conn = self.get_conn()?;
                let added: i32 = conn.sadd(key, member)?;
                Ok(serde_json::json!({ "added": added }))
            }

            "srem" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let member = params.get("member").and_then(|v| v.as_str()).ok_or("缺少 member")?;
                let mut conn = self.get_conn()?;
                let removed: i32 = conn.srem(key, member)?;
                Ok(serde_json::json!({ "removed": removed }))
            }

            // ── ZSet ──
            "get_zset" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let mut conn = self.get_conn()?;
                let members: Vec<(String, f64)> = conn.zrange_withscores(key, 0, -1)?;
                let result: Vec<Value> = members.into_iter().map(|(m, s)| {
                    serde_json::json!({ "member": m, "score": s })
                }).collect();
                Ok(serde_json::json!({ "members": result }))
            }

            "zadd" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let score = params.get("score").and_then(|v| v.as_f64()).ok_or("缺少 score")?;
                let member = params.get("member").and_then(|v| v.as_str()).ok_or("缺少 member")?;
                let mut conn = self.get_conn()?;
                let added: i32 = conn.zadd(key, member, score)?;
                Ok(serde_json::json!({ "added": added }))
            }

            "zrem" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or("缺少 key")?;
                let member = params.get("member").and_then(|v| v.as_str()).ok_or("缺少 member")?;
                let mut conn = self.get_conn()?;
                let removed: i32 = conn.zrem(key, member)?;
                Ok(serde_json::json!({ "removed": removed }))
            }

            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(RedisClientPlugin::new()));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}

// ── hex 编解码 (minimal internal impl to avoid extra dep) ──
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        if s.len() % 2 != 0 { return Err(()); }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i+2], 16).map_err(|_| ()))
            .collect()
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p redis-client`
Expected: success

---

### Task 3.3: 创建前端项目骨架

**Files:**
- Create: `plugins/redis-client/frontend/package.json`
- Create: `plugins/redis-client/frontend/tsconfig.json`
- Create: `plugins/redis-client/frontend/tsconfig.node.json`
- Create: `plugins/redis-client/frontend/vite.config.ts`
- Create: `plugins/redis-client/frontend/index.html`

- [ ] **Step 1: Write skeleton files**

Write these 5 files with the following differences from Task 1.3:
- `package.json`: `"name": "redis-client-frontend"`
- `index.html`: `<title>Redis 客户端</title>`
- `tsconfig.json`, `tsconfig.node.json`, `vite.config.ts`: identical to Task 1.3, copy verbatim

- [ ] **Step 2: Install dependencies**

Run: `cd plugins/redis-client/frontend && npm install`

---

### Task 3.4: 实现前端

**Files:**
- Create: `plugins/redis-client/frontend/src/main.tsx`
- Create: `plugins/redis-client/frontend/src/App.tsx`
- Create: `plugins/redis-client/frontend/src/App.css`

redis-client 前端最复杂：左侧连接面板 + 右侧 key 浏览/值编辑。分为两个组件。

- [ ] **Step 1: Write main.tsx** — same pattern

- [ ] **Step 2: Write App.tsx**

File: `plugins/redis-client/frontend/src/App.tsx`
```tsx
import { useState, useEffect, useCallback, useRef } from 'react';
import './App.css';

declare global {
  interface Window {
    pluginAPI?: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

interface KeyInfo { key: string; type: string; ttl: number; }
interface SavedConn { id: string; name: string; host: string; port: number; db: number; has_password: boolean; }

function App() {
  const [connected, setConnected] = useState(false);
  const [connForm, setConnForm] = useState({ host: '127.0.0.1', port: 6379, db: 0, password: '' });
  const [savedConns, setSavedConns] = useState<SavedConn[]>([]);
  const [keys, setKeys] = useState<KeyInfo[]>([]);
  const [cursor, setCursor] = useState(0);
  const [search, setSearch] = useState('*');
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [keyDetail, setKeyDetail] = useState<Record<string, unknown> | null>(null);
  const [valueData, setValueData] = useState<unknown>(null);
  const [error, setError] = useState('');
  const [editingField, setEditingField] = useState({ field: '', value: '' });
  const [newMember, setNewMember] = useState('');

  const clearError = () => setError('');
  const showError = (e: unknown) => setError(String(e));

  // 加载已保存连接
  const loadSavedConns = useCallback(async () => {
    try {
      const r = await window.pluginAPI?.call('redis-client', 'list_saved_connections', {});
      if (r && typeof r === 'object' && 'connections' in r) {
        setSavedConns((r as { connections: SavedConn[] }).connections);
      }
    } catch { /* ignore */ }
  }, []);

  useEffect(() => { loadSavedConns(); }, [loadSavedConns]);

  // 连接
  const handleConnect = useCallback(async (host: string, port: number, db: number, password: string) => {
    clearError();
    try {
      await window.pluginAPI?.call('redis-client', 'connect', { host, port, db, password });
      setConnected(true);
      setConnForm({ host, port, db, password: '' });
      // 连接后扫描 key
      const r = await window.pluginAPI?.call('redis-client', 'scan_keys', { cursor: 0, pattern: '*', count: 50 });
      if (r && typeof r === 'object' && 'keys' in r) {
        const data = r as { cursor: number; keys: KeyInfo[] };
        setCursor(data.cursor);
        setKeys(data.keys);
      }
    } catch (e) { showError(e); }
  }, []);

  const handleDisconnect = useCallback(async () => {
    await window.pluginAPI?.call('redis-client', 'disconnect', {});
    setConnected(false);
    setKeys([]);
    setSelectedKey(null);
    setKeyDetail(null);
    setValueData(null);
  }, []);

  // 扫描 key
  const handleScan = useCallback(async (pattern?: string) => {
    try {
      const r = await window.pluginAPI?.call('redis-client', 'scan_keys', {
        cursor: 0,
        pattern: pattern || search,
        count: 50,
      });
      if (r && typeof r === 'object' && 'keys' in r) {
        const data = r as { cursor: number; keys: KeyInfo[] };
        setCursor(data.cursor);
        setKeys(data.keys);
      }
    } catch (e) { showError(e); }
  }, [search]);

  // 选中 key
  const handleSelectKey = useCallback(async (key: string) => {
    setSelectedKey(key);
    clearError();
    try {
      const info = await window.pluginAPI?.call('redis-client', 'get_key_info', { key });
      setKeyDetail(info as Record<string, unknown>);

      const kType = (info as Record<string, string>).type;
      if (kType === 'string') {
        const v = await window.pluginAPI?.call('redis-client', 'get_string', { key });
        setValueData(v);
      } else if (kType === 'hash') {
        const v = await window.pluginAPI?.call('redis-client', 'get_hash', { key });
        setValueData(v);
      } else if (kType === 'list') {
        const v = await window.pluginAPI?.call('redis-client', 'get_list', { key, start: 0, stop: -1 });
        setValueData(v);
      } else if (kType === 'set') {
        const v = await window.pluginAPI?.call('redis-client', 'get_set', { key });
        setValueData(v);
      } else if (kType === 'zset') {
        const v = await window.pluginAPI?.call('redis-client', 'get_zset', { key });
        setValueData(v);
      }
    } catch (e) { showError(e); }
  }, []);

  // 删除 key
  const handleDeleteKey = useCallback(async () => {
    if (!selectedKey) return;
    try {
      await window.pluginAPI?.call('redis-client', 'delete_key', { key: selectedKey });
      setSelectedKey(null);
      setKeyDetail(null);
      setValueData(null);
      handleScan();
    } catch (e) { showError(e); }
  }, [selectedKey, handleScan]);

  // Hash 字段操作
  const handleSetHashField = useCallback(async (field: string, value: string) => {
    if (!selectedKey) return;
    try {
      await window.pluginAPI?.call('redis-client', 'set_hash_field', { key: selectedKey, field, value });
      setEditingField({ field: '', value: '' });
      handleSelectKey(selectedKey);
    } catch (e) { showError(e); }
  }, [selectedKey, handleSelectKey]);

  const handleDelHashField = useCallback(async (field: string) => {
    if (!selectedKey) return;
    try {
      await window.pluginAPI?.call('redis-client', 'del_hash_field', { key: selectedKey, field });
      handleSelectKey(selectedKey);
    } catch (e) { showError(e); }
  }, [selectedKey, handleSelectKey]);

  // String 编辑
  const [editingStringValue, setEditingStringValue] = useState('');
  const handleSaveString = useCallback(async () => {
    if (!selectedKey) return;
    try {
      await window.pluginAPI?.call('redis-client', 'set_string', { key: selectedKey, value: editingStringValue });
      handleSelectKey(selectedKey);
    } catch (e) { showError(e); }
  }, [selectedKey, editingStringValue, handleSelectKey]);

  return (
    <div className="redis-client">
      {!connected ? (
        /* ── 连接面板 ── */
        <div className="connect-panel">
          <h3>连接 Redis</h3>
          <div className="form-group">
            <label>Host</label>
            <input type="text" value={connForm.host} onChange={e => setConnForm(p => ({ ...p, host: e.target.value }))} />
          </div>
          <div className="form-group">
            <label>Port</label>
            <input type="number" value={connForm.port} onChange={e => setConnForm(p => ({ ...p, port: Number(e.target.value) }))} />
          </div>
          <div className="form-group">
            <label>DB</label>
            <input type="number" value={connForm.db} onChange={e => setConnForm(p => ({ ...p, db: Number(e.target.value) }))} />
          </div>
          <div className="form-group">
            <label>Password</label>
            <input type="password" value={connForm.password} onChange={e => setConnForm(p => ({ ...p, password: e.target.value }))}
              onKeyDown={e => e.key === 'Enter' && handleConnect(connForm.host, connForm.port, connForm.db, connForm.password)} />
          </div>
          <button className="btn-primary" onClick={() => handleConnect(connForm.host, connForm.port, connForm.db, connForm.password)}>连接</button>

          {savedConns.length > 0 && (
            <div className="saved-connections">
              <h4>已保存连接</h4>
              {savedConns.map(c => (
                <div key={c.id} className="saved-conn-item" onClick={async () => {
                  try {
                    const r = await window.pluginAPI?.call('redis-client', 'get_saved_password', { id: c.id });
                    const pass = (r as { password: string }).password || '';
                    handleConnect(c.host, c.port, c.db, pass);
                  } catch (e) { showError(e); }
                }}>
                  <span className="conn-name">{c.name}</span>
                  <span className="conn-info">{c.host}:{c.port} db{c.db}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      ) : (
        /* ── 主界面 ── */
        <div className="main-layout">
          {/* 左侧 Key 列表 */}
          <div className="key-panel">
            <div className="panel-header">
              <input
                type="text"
                value={search}
                onChange={e => setSearch(e.target.value)}
                placeholder="搜索 key (* 通配)"
                onKeyDown={e => e.key === 'Enter' && handleScan()}
              />
              <button onClick={() => handleScan()}>🔍</button>
              <button onClick={handleDisconnect} title="断开">✕</button>
            </div>
            <div className="key-list">
              {keys.map(k => (
                <div
                  key={k.key}
                  className={`key-item ${selectedKey === k.key ? 'selected' : ''}`}
                  onClick={() => handleSelectKey(k.key)}
                >
                  <span className="key-type-badge">{k.type}</span>
                  <span className="key-name">{k.key}</span>
                  {k.ttl > 0 && <span className="key-ttl">{k.ttl}s</span>}
                </div>
              ))}
            </div>
          </div>

          {/* 右侧详情 */}
          <div className="detail-panel">
            {selectedKey && keyDetail ? (
              <div className="detail-content">
                <div className="detail-header">
                  <h4>{selectedKey}</h4>
                  <span className="type-badge">{keyDetail.type as string}</span>
                  <span>TTL: {keyDetail.ttl as number}</span>
                  <button className="btn-danger" onClick={handleDeleteKey}>删除</button>
                </div>

                {/* String 值 */}
                {keyDetail.type === 'string' && valueData && (
                  <div className="value-editor">
                    <textarea
                      value={editingStringValue || (valueData as { value: string }).value}
                      onChange={e => setEditingStringValue(e.target.value)}
                      onFocus={() => setEditingStringValue((valueData as { value: string }).value)}
                      rows={12}
                    />
                    <button className="btn-primary" onClick={handleSaveString}>保存</button>
                  </div>
                )}

                {/* Hash 值 */}
                {keyDetail.type === 'hash' && valueData && (
                  <div className="hash-editor">
                    <table>
                      <thead><tr><th>Field</th><th>Value</th><th>操作</th></tr></thead>
                      <tbody>
                        {Object.entries((valueData as { fields: Record<string, string> }).fields).map(([f, v]) => (
                          <tr key={f}>
                            <td><code>{f}</code></td>
                            <td><code>{v}</code></td>
                            <td><button onClick={() => handleDelHashField(f)}>删除</button></td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                    <div className="add-field">
                      <input placeholder="field" value={editingField.field} onChange={e => setEditingField(p => ({ ...p, field: e.target.value }))} />
                      <input placeholder="value" value={editingField.value} onChange={e => setEditingField(p => ({ ...p, value: e.target.value }))} />
                      <button className="btn-primary" onClick={() => handleSetHashField(editingField.field, editingField.value)}>添加</button>
                    </div>
                  </div>
                )}

                {/* List 值 */}
                {keyDetail.type === 'list' && valueData && (
                  <div className="list-editor">
                    <ol>
                      {(valueData as { items: string[] }).items.map((item, i) => (
                        <li key={i}><code>{item}</code></li>
                      ))}
                    </ol>
                  </div>
                )}

                {/* Set 值 */}
                {keyDetail.type === 'set' && valueData && (
                  <div className="set-editor">
                    {(valueData as { members: string[] }).members.map(m => (
                      <span key={m} className="member-tag">{m}</span>
                    ))}
                  </div>
                )}

                {/* ZSet 值 */}
                {keyDetail.type === 'zset' && valueData && (
                  <div className="zset-editor">
                    <table>
                      <thead><tr><th>Member</th><th>Score</th></tr></thead>
                      <tbody>
                        {(valueData as { members: Array<{ member: string; score: number }> }).members.map(m => (
                          <tr key={m.member}>
                            <td><code>{m.member}</code></td>
                            <td>{m.score}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </div>
            ) : (
              <div className="empty-detail">选择一个 Key 查看详情</div>
            )}
          </div>
        </div>
      )}

      {error && <div className="error-toast" onClick={() => setError('')}>{error} (点击关闭)</div>}
    </div>
  );
}

export default App;
```

- [ ] **Step 3: Write App.css**

```css
.redis-client {
  flex: 1;
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-primary);
}

/* ── 连接面板 ── */
.connect-panel {
  max-width: 400px;
  margin: 40px auto;
  padding: 24px;
  background: var(--bg-secondary);
  border-radius: var(--radius-lg);
}

.connect-panel h3 {
  margin: 0 0 20px 0;
  color: var(--text-primary);
}

.form-group {
  margin-bottom: 12px;
}

.form-group label {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}

.form-group input, .form-group select {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-size: 14px;
  background: var(--bg-primary);
  color: var(--text-primary);
  box-sizing: border-box;
}

.saved-connections {
  margin-top: 20px;
}

.saved-connections h4 {
  font-size: 13px;
  color: var(--text-secondary);
  margin: 0 0 8px 0;
}

.saved-conn-item {
  display: flex;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  margin-bottom: 4px;
  cursor: pointer;
  font-size: 13px;
}

.saved-conn-item:hover { border-color: var(--accent); }

.conn-name { font-weight: 600; color: var(--text-primary); }
.conn-info { color: var(--text-secondary); font-family: monospace; font-size: 12px; }

/* ── 主布局 ── */
.main-layout {
  flex: 1;
  display: flex;
  overflow: hidden;
}

/* ── Key 面板 ── */
.key-panel {
  width: 260px;
  min-width: 260px;
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
}

.panel-header {
  display: flex;
  gap: 4px;
  padding: 8px;
  border-bottom: 1px solid var(--border-color);
}

.panel-header input {
  flex: 1;
  padding: 6px 10px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-size: 13px;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.panel-header button {
  padding: 6px 10px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  cursor: pointer;
  color: var(--text-primary);
}

.key-list {
  flex: 1;
  overflow: auto;
}

.key-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  font-size: 13px;
  border-bottom: 1px solid var(--border-light);
}

.key-item:hover { background: var(--hover-bg); }
.key-item.selected { background: var(--accent-light); border-left: 3px solid var(--accent); }

.key-type-badge {
  font-size: 10px;
  padding: 1px 6px;
  background: var(--accent-light);
  color: var(--accent);
  border-radius: var(--radius-xs);
  font-weight: 600;
}

.key-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-primary);
  font-family: monospace;
}

.key-ttl {
  font-size: 11px;
  color: var(--text-tertiary);
}

/* ── 详情面板 ── */
.detail-panel {
  flex: 1;
  overflow: auto;
  padding: 20px;
}

.detail-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border-color);
}

.detail-header h4 {
  margin: 0;
  font-family: monospace;
  color: var(--text-primary);
}

.type-badge {
  font-size: 12px;
  padding: 2px 10px;
  background: var(--accent-light);
  color: var(--accent);
  border-radius: var(--radius-sm);
  font-weight: 600;
}

.empty-detail {
  text-align: center;
  padding: 60px 20px;
  color: var(--text-tertiary);
}

.value-editor textarea {
  width: 100%;
  padding: 12px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-family: 'Monaco', 'Menlo', monospace;
  font-size: 13px;
  background: var(--bg-primary);
  color: var(--text-primary);
  resize: vertical;
  box-sizing: border-box;
}

.hash-editor table, .zset-editor table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.hash-editor th, .zset-editor th {
  text-align: left;
  padding: 6px 12px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
  font-weight: 600;
  border-bottom: 2px solid var(--border-color);
}

.hash-editor td, .zset-editor td {
  padding: 6px 12px;
  border-bottom: 1px solid var(--border-light);
}

.hash-editor code, .zset-editor code, .list-editor code {
  font-family: 'Monaco', 'Menlo', monospace;
  font-size: 12px;
  color: var(--text-primary);
}

.add-field {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

.add-field input {
  flex: 1;
  padding: 6px 10px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-size: 13px;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.list-editor ol {
  padding-left: 24px;
}

.list-editor li {
  padding: 4px 0;
  font-size: 13px;
}

.set-editor {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.member-tag {
  padding: 4px 12px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-family: monospace;
  font-size: 13px;
  color: var(--text-primary);
}

.btn-primary {
  padding: 8px 16px;
  background: var(--accent);
  color: var(--text-inverse);
  border: none;
  border-radius: var(--radius-sm);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
}

.btn-primary:hover { background: var(--accent-hover); }

.btn-danger {
  padding: 6px 14px;
  background: var(--error);
  color: white;
  border: none;
  border-radius: var(--radius-sm);
  font-size: 12px;
  cursor: pointer;
  margin-left: auto;
}

.error-toast {
  position: fixed;
  bottom: 16px;
  left: 50%;
  transform: translateX(-50%);
  padding: 10px 20px;
  background: var(--error-light);
  border: 1px solid var(--error-border);
  border-radius: var(--radius-md);
  color: var(--error-text);
  font-size: 13px;
  z-index: 1000;
  cursor: pointer;
}
```

- [ ] **Step 4: Build frontend**

Run: `cd plugins/redis-client/frontend && npm run build`
Expected: build succeeds

- [ ] **Step 5: Verify full compilation**

Run: `cargo check -p redis-client`
Expected: success

---

## 最终验证

- [ ] **验证全部 3 个插件编译通过**

Run: `cargo check -p timestamp-converter -p cron-tools -p redis-client`
Expected: all 3 succeed

- [ ] **运行 cargo fmt**

Run: `cargo fmt`
Expected: no format changes needed (or apply them)

- [ ] **运行 cargo clippy**

Run: `cargo clippy -p timestamp-converter -p cron-tools -p redis-client`
Expected: no warnings
