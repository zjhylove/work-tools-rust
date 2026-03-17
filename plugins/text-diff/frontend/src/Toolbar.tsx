import { useState, useCallback } from 'react';
import './Toolbar.css';

interface ToolbarProps {
  originalFileName: string;
  modifiedFileName: string;
  viewMode: 'side-by-side' | 'inline';
  onOpenOriginal: () => void;
  onOpenModified: () => void;
  onNextDiff: () => void;
  onPreviousDiff: () => void;
  onExport: () => void;
  onToggleViewMode: () => void;
  onToggleIgnoreWhitespace: (value: boolean) => void;
  onToggleIgnoreCase: (value: boolean) => void;
  diffStats: {
    additions: number;
    deletions: number;
    modifications: number;
  };
}

export function Toolbar({
  originalFileName,
  modifiedFileName,
  viewMode,
  onOpenOriginal,
  onOpenModified,
  onNextDiff,
  onPreviousDiff,
  onExport,
  onToggleViewMode,
  onToggleIgnoreWhitespace,
  onToggleIgnoreCase,
  diffStats
}: ToolbarProps) {
  const [ignoreWhitespace, setIgnoreWhitespace] = useState(false);
  const [ignoreCase, setIgnoreCase] = useState(false);

  const handleWhitespaceChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.checked;
    setIgnoreWhitespace(value);
    onToggleIgnoreWhitespace(value);
  }, [onToggleIgnoreWhitespace]);

  const handleCaseChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.checked;
    setIgnoreCase(value);
    onToggleIgnoreCase(value);
  }, [onToggleIgnoreCase]);

  return (
    <div className="toolbar">
      {/* 文件操作组 */}
      <div className="toolbar-group">
        <button
          onClick={onOpenOriginal}
          title="打开左侧文件 (Ctrl+O)"
          className="icon-btn file-btn"
        >
          📂 {originalFileName.length > 20 ? originalFileName.substring(0, 20) + '...' : originalFileName}
        </button>
        <button
          onClick={onOpenModified}
          title="打开右侧文件 (Ctrl+Shift+O)"
          className="icon-btn file-btn"
        >
          📂 {modifiedFileName.length > 20 ? modifiedFileName.substring(0, 20) + '...' : modifiedFileName}
        </button>
      </div>

      {/* 选项组 */}
      <div className="toolbar-group">
        <label className="compact-checkbox">
          <input
            type="checkbox"
            checked={ignoreWhitespace}
            onChange={handleWhitespaceChange}
          />
          <span>忽略空白</span>
        </label>
        <label className="compact-checkbox">
          <input
            type="checkbox"
            checked={ignoreCase}
            onChange={handleCaseChange}
          />
          <span>忽略大小写</span>
        </label>
      </div>

      {/* 视图模式切换 */}
      <div className="toolbar-group">
        <button
          onClick={onToggleViewMode}
          title="切换视图模式"
          className="view-toggle-btn"
        >
          {viewMode === 'side-by-side' ? '📊 并排' : '📄 行内'}
        </button>
      </div>

      {/* 导出按钮 */}
      <div className="toolbar-group">
        <button
          onClick={onExport}
          title="导出差异 (Ctrl+S)"
          className="export-btn"
        >
          💾 导出
        </button>
      </div>

      {/* 导航组 - 放在右侧 */}
      <div className="toolbar-group nav-group">
        <button
          onClick={onPreviousDiff}
          title="上一个差异 (Shift+F8)"
          className="nav-btn"
        >
          ↑
        </button>
        <button
          onClick={onNextDiff}
          title="下一个差异 (F8)"
          className="nav-btn"
        >
          ↓
        </button>
      </div>

      {/* 统计信息 - 最右侧 */}
      <div className="toolbar-group stats-group">
        <span className="stats-label">统计:</span>
        <span className="stats-value additions">+{diffStats.additions}</span>
        <span className="stats-value deletions">-{diffStats.deletions}</span>
        <span className="stats-value modifications">~{diffStats.modifications}</span>
      </div>
    </div>
  );
}
