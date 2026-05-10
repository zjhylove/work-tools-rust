import { useState, useEffect, useCallback, useRef } from 'react';
import './App.css';

declare global {
  interface Window {
    pluginAPI?: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
    WorkTools: {
      toast: {
        success: (message: string) => void;
        error: (message: string) => void;
        info: (message: string) => void;
        warning: (message: string) => void;
      };
    };
  }
}
const WorkTools = window.WorkTools;

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
  const [pulse, setPulse] = useState(false);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  useEffect(() => {
    const tick = async () => {
      try {
        const result = await window.pluginAPI?.call('timestamp-converter', 'current_time', { timezone });
        if (mountedRef.current && result) {
          setCurrentTime(result as typeof currentTime);
          setPulse(true);
          setTimeout(() => setPulse(false), 400);
        }
      } catch { /* ignore */ }
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [timezone]);

  const handleTsToDt = useCallback(async () => {
    try {
      const result = await window.pluginAPI?.call('timestamp-converter', 'timestamp_to_datetime', { ts: tsInput, timezone });
      setTsResult((result as Record<string, string>) || null);
    } catch (e) {
      WorkTools.toast.error(String(e));
      setTsResult(null);
    }
  }, [tsInput, timezone]);

  const handleDtToTs = useCallback(async () => {
    try {
      const result = await window.pluginAPI?.call('timestamp-converter', 'datetime_to_timestamp', { datetime: dtInput, timezone });
      setDtResult((result as Record<string, number>) || null);
    } catch (e) {
      WorkTools.toast.error(String(e));
      setDtResult(null);
    }
  }, [dtInput, timezone]);

  const handleBatchConvert = useCallback(async () => {
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
      WorkTools.toast.error(String(e));
      setBatchResults([]);
    }
  }, [batchInput, timezone]);

  return (
    <div className="ts-converter">
      {/* Hero Clock */}
      <div className="hero-clock">
        <div className="clock-display">
          <span className={`clock-datetime ${pulse ? 'pulse' : ''}`}>{currentTime.datetime}</span>
          <span className="clock-unix">Unix {currentTime.ts_sec}</span>
        </div>
        <div className="clock-meta">
          <span className="clock-utc">UTC {currentTime.utc}</span>
          <div className="tz-select-wrapper">
            <select className="tz-select" value={timezone} onChange={e => setTimezone(e.target.value)}>
              {TIMEZONES.map(tz => (
                <option key={tz.value} value={tz.value}>{tz.label}</option>
              ))}
            </select>
          </div>
        </div>
      </div>

      {/* Tab Bar */}
      <nav className="tab-bar">
        <button
          className={`tab ${activeTab === 'ts2dt' ? 'active' : ''}`}
          onClick={() => setActiveTab('ts2dt')}
        >
          <span className="tab-icon">↓</span>
          <span className="tab-label">时间戳 → 日期</span>
        </button>
        <button
          className={`tab ${activeTab === 'dt2ts' ? 'active' : ''}`}
          onClick={() => setActiveTab('dt2ts')}
        >
          <span className="tab-icon">↑</span>
          <span className="tab-label">日期 → 时间戳</span>
        </button>
        <button
          className={`tab ${activeTab === 'batch' ? 'active' : ''}`}
          onClick={() => setActiveTab('batch')}
        >
          <span className="tab-icon">⇉</span>
          <span className="tab-label">批量转换</span>
        </button>
        <div className="tab-indicator" style={{ '--tab-index': activeTab === 'ts2dt' ? 0 : activeTab === 'dt2ts' ? 1 : 2 } as React.CSSProperties} />
      </nav>

      {/* Content */}
      <div className="tab-content">
        {activeTab === 'ts2dt' && (
          <div className="convert-panel">
            <div className="input-group">
              <span className="input-prefix">#</span>
              <input
                type="text"
                value={tsInput}
                onChange={e => setTsInput(e.target.value)}
                placeholder="1756193728 (支持秒/毫秒/微秒)"
                onKeyDown={e => e.key === 'Enter' && handleTsToDt()}
                autoFocus
              />
              <button className="btn-convert" onClick={handleTsToDt}>转换</button>
            </div>
            {tsResult && (
              <div className="result-card">
                <div className="result-item">
                  <span className="result-key">ISO 8601</span>
                  <code className="result-value">{tsResult.format_iso}</code>
                </div>
                <div className="result-item">
                  <span className="result-key">RFC 2822</span>
                  <code className="result-value">{tsResult.format_rfc2822}</code>
                </div>
                <div className="result-item">
                  <span className="result-key">{tsResult.timezone}</span>
                  <code className="result-value">{tsResult.datetime}</code>
                </div>
                <div className="result-item">
                  <span className="result-key">UTC</span>
                  <code className="result-value">{tsResult.utc}</code>
                </div>
              </div>
            )}
          </div>
        )}

        {activeTab === 'dt2ts' && (
          <div className="convert-panel">
            <div className="input-group">
              <span className="input-prefix">@</span>
              <input
                type="text"
                value={dtInput}
                onChange={e => setDtInput(e.target.value)}
                placeholder="2026-05-02T17:35:28+08:00"
                onKeyDown={e => e.key === 'Enter' && handleDtToTs()}
                autoFocus
              />
              <button className="btn-convert" onClick={handleDtToTs}>转换</button>
            </div>
            {dtResult && (
              <div className="result-card">
                <div className="result-stat-row">
                  <div className="stat">
                    <span className="stat-value">{dtResult.ts_sec}</span>
                    <span className="stat-label">秒</span>
                  </div>
                  <div className="stat">
                    <span className="stat-value">{dtResult.ts_ms}</span>
                    <span className="stat-label">毫秒</span>
                  </div>
                  <div className="stat">
                    <span className="stat-value">{dtResult.ts_us}</span>
                    <span className="stat-label">微秒</span>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}

        {activeTab === 'batch' && (
          <div className="convert-panel">
            <div className="batch-input-group">
              <textarea
                value={batchInput}
                onChange={e => setBatchInput(e.target.value)}
                placeholder={'每行一个值，自动识别类型\n\n1756193728\n2026-05-02T17:35:28+08:00\n1714608000000'}
                rows={6}
              />
              <button className="btn-convert" onClick={handleBatchConvert}>批量转换</button>
            </div>
            {batchResults.length > 0 && (
              <div className="batch-table-wrapper">
                <table className="batch-table">
                  <thead>
                    <tr><th>输入</th><th>结果</th></tr>
                  </thead>
                  <tbody>
                    {batchResults.map((r, i) => (
                      <tr key={i} className={r.error ? 'row-error' : ''}>
                        <td className="batch-input-cell">{r.input}</td>
                        <td className="batch-result-cell">{r.error || r.datetime || `${r.ts_sec} (秒) / ${r.ts_ms} (毫秒)`}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        )}
      </div>

    </div>
  );
}

export default App;
