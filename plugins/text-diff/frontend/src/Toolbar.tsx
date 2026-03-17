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
  // 提取短文件名用于显示
  const getShortFileName = (fullName: string) => {
    if (!fullName || fullName === '原始文件' || fullName === '修改后的文件') {
      return fullName;
    }
    // 从完整路径中提取文件名
    const parts = fullName.split(/[/\\]/);
    const fileName = parts[parts.length - 1];
    // 如果文件名超过15个字符，截断并添加省略号
    return fileName.length > 15 ? fileName.substring(0, 12) + '...' : fileName;
  };
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
          📂 {getShortFileName(originalFileName)}
        </button>
        <button
          onClick={onOpenModified}
          title="打开右侧文件 (Ctrl+Shift+O)"
          className="icon-btn file-btn"
        >
          📂 {getShortFileName(modifiedFileName)}
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
        <span className="stats-label">差异:</span>
        <span className="stats-value additions" title={`新增 ${diffStats.additions} 行`}>
          +{diffStats.additions}
        </span>
        <span className="stats-value deletions" title={`删除 ${diffStats.deletions} 行`}>
          -{diffStats.deletions}
        </span>
        {diffStats.modifications > 0 && (
          <span className="stats-value modifications" title={`修改 ${diffStats.modifications} 处`}>
            ±{diffStats.modifications}
          </span>
        )}
      </div>
    </div>
  );
}
