import { useState, useCallback, useEffect, useRef } from 'react';
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
  const [fields, setFields] = useState(['*/5', '*', '*', '*', '*']);
  const parseTimer = useRef<ReturnType<typeof setTimeout>>();

  // Auto-parse after typing stops
  const autoParse = useCallback((expression: string) => {
    if (parseTimer.current) clearTimeout(parseTimer.current);
    parseTimer.current = setTimeout(async () => {
      try {
        const [parseR, execR] = await Promise.all([
          window.pluginAPI?.call('cron-tools', 'parse_cron', { expr: expression.trim() }),
          window.pluginAPI?.call('cron-tools', 'next_executions', { expr: expression.trim(), count: 5 }),
        ]);
        if (parseR && typeof parseR === 'object') {
          const r = parseR as { valid: boolean; description: string; error: string | null };
          setValid(r.valid);
          setDescription(r.description);
        }
        if (execR && typeof execR === 'object' && 'times' in execR) {
          setExecTimes((execR as { times: string[] }).times);
        }
      } catch { /* ignore */ }
    }, 300);
  }, []);

  // Initial parse on mount
  useEffect(() => {
    autoParse(expr);
    window.pluginAPI?.call('cron-tools', 'get_presets', {}).then(result => {
      if (result && typeof result === 'object' && 'presets' in result) {
        setPresets((result as { presets: Preset[] }).presets);
      }
    });
    return () => { if (parseTimer.current) clearTimeout(parseTimer.current); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleExprChange = useCallback((newExpr: string) => {
    setExpr(newExpr);
    const parts = newExpr.trim().split(/\s+/);
    if (parts.length === 5) {
      setFields(parts);
      autoParse(newExpr);
    }
  }, [autoParse]);

  const handleFieldChange = useCallback((index: number, value: string) => {
    const newFields = [...fields];
    newFields[index] = value;
    setFields(newFields);
    const newExpr = newFields.join(' ');
    setExpr(newExpr);
    autoParse(newExpr);
  }, [fields, autoParse]);

  const handlePresetClick = useCallback((preset: Preset) => {
    handleExprChange(preset.expr);
  }, [handleExprChange]);

  const formatExecTime = (iso: string) => {
    try {
      const d = new Date(iso);
      return d.toLocaleString('zh-CN', {
        month: '2-digit', day: '2-digit',
        hour: '2-digit', minute: '2-digit', second: '2-digit',
      });
    } catch {
      return iso;
    }
  };

  return (
    <div className="cron-tools">
      {/* Expression Input */}
      <div className="expr-section">
        <label className="section-label">Cron 表达式</label>
        <div className={`expr-input-wrapper ${!valid ? 'invalid' : ''}`}>
          <span className="expr-prefix">$</span>
          <input
            type="text"
            value={expr}
            onChange={e => handleExprChange(e.target.value)}
            placeholder="*/5 * * * *"
            className="expr-input"
            spellCheck={false}
          />
        </div>
      </div>

      {/* Description Banner */}
      {description && (
        <div className={`desc-banner ${valid ? 'valid' : 'invalid'}`}>
          <span className="desc-icon">{valid ? '✓' : '✗'}</span>
          <span className="desc-text">{description}</span>
        </div>
      )}

      {/* Execution Timeline */}
      {execTimes.length > 0 && (
        <div className="timeline-section">
          <label className="section-label">下次执行</label>
          <div className="timeline">
            {execTimes.map((t, i) => (
              <div key={i} className="timeline-item">
                <div className="timeline-dot" />
                {i < execTimes.length - 1 && <div className="timeline-line" />}
                <span className="timeline-index">{i + 1}</span>
                <code className="timeline-time">{formatExecTime(t)}</code>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Presets */}
      {presets.length > 0 && (
        <div className="presets-section">
          <label className="section-label">快速模板</label>
          <div className="preset-chips">
            {presets.map(p => (
              <button
                key={p.label}
                className={`chip ${expr.trim() === p.expr ? 'active' : ''}`}
                onClick={() => handlePresetClick(p)}
              >
                {p.label}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Visual Builder */}
      <div className="builder-section">
        <button
          className={`builder-toggle ${showBuilder ? 'open' : ''}`}
          onClick={() => setShowBuilder(!showBuilder)}
        >
          可视化构建
          <span className="toggle-arrow">▾</span>
        </button>
        <div className={`builder-panel ${showBuilder ? 'open' : ''}`}>
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
      </div>

    </div>
  );
}

export default App;
