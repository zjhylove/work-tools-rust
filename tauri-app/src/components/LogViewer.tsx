import React, { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./Dialog.css";

export interface LogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
}

interface LogViewerProps {
  onClose: () => void;
}

const LEVEL_COLORS: Record<string, string> = {
  ERROR: "#e74c3c",
  WARN: "#f39c12",
  INFO: "#2ecc71",
  DEBUG: "#95a5a6",
  TRACE: "#7f8c8d",
};

const LogViewer: React.FC<LogViewerProps> = ({ onClose }) => {
  const [allLogs, setAllLogs] = useState<LogEntry[]>([]);
  const [levelFilter, setLevelFilter] = useState("ALL");
  const [pluginFilter, setPluginFilter] = useState("");
  const [autoRefresh, setAutoRefresh] = useState(true);
  const containerRef = useRef<HTMLDivElement>(null);

  const fetchLogs = useCallback(async () => {
    try {
      const query: Record<string, unknown> = {};
      const logs = await invoke<LogEntry[]>("get_logs", { query });
      setAllLogs(logs);
    } catch (e) {
      console.error("获取日志失败:", e);
    }
  }, []);

  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  useEffect(() => {
    if (!autoRefresh) return;
    const interval = setInterval(fetchLogs, 3000);
    return () => clearInterval(interval);
  }, [fetchLogs, autoRefresh]);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = 0;
    }
  }, [allLogs]);

  const filteredLogs = allLogs.filter((entry) => {
    if (levelFilter !== "ALL" && entry.level !== levelFilter) return false;
    if (
      pluginFilter &&
      !entry.target.toLowerCase().includes(pluginFilter.toLowerCase())
    )
      return false;
    return true;
  });

  const formatTime = (timestamp: string) => {
    try {
      const d = new Date(timestamp);
      return d.toLocaleTimeString("zh-CN", { hour12: false });
    } catch {
      return timestamp;
    }
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div
        className="dialog-content dialog-large"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-header">
          <h2>系统日志</h2>
          <button className="dialog-close" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="log-filter-bar">
          <select
            className="log-filter-select"
            value={levelFilter}
            onChange={(e) => setLevelFilter(e.target.value)}
          >
            <option value="ALL">所有级别</option>
            <option value="ERROR">ERROR</option>
            <option value="WARN">WARN</option>
            <option value="INFO">INFO</option>
            <option value="DEBUG">DEBUG</option>
            <option value="TRACE">TRACE</option>
          </select>

          <input
            className="log-filter-input"
            type="text"
            placeholder="过滤模块…"
            value={pluginFilter}
            onChange={(e) => setPluginFilter(e.target.value)}
          />

          <label className="log-auto-refresh">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
            />
            自动刷新
          </label>

          <button className="button-secondary" onClick={fetchLogs}>
            刷新
          </button>
        </div>

        <div className="dialog-body">
          <div className="log-viewer" ref={containerRef}>
            {filteredLogs.length === 0 ? (
              <div className="log-empty">暂无日志记录</div>
            ) : (
              filteredLogs.map((entry, i) => (
                <div
                  key={`${entry.timestamp}-${i}`}
                  className="log-entry"
                >
                  <span className="log-time">
                    {formatTime(entry.timestamp)}
                  </span>
                  <span
                    className="log-level"
                    style={{
                      color: LEVEL_COLORS[entry.level] || "#bdc3c7",
                    }}
                  >
                    [{entry.level}]
                  </span>
                  {entry.target && (
                    <span className="log-target" title={entry.target}>
                      [{entry.target}]
                    </span>
                  )}
                  <span className="log-message">{entry.message}</span>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default LogViewer;
