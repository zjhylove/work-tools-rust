import { useState, useCallback } from 'react';

interface ToolbarProps {
  onOpenLeft: () => void;
  onOpenRight: () => void;
  onNextDiff: () => void;
  onPreviousDiff: () => void;
  onExport: () => void;
  onToggleIgnoreWhitespace: (value: boolean) => void;
  onToggleIgnoreCase: (value: boolean) => void;
  diffStats: {
    additions: number;
    deletions: number;
    modifications: number;
  };
}

export function Toolbar({
  onOpenLeft,
  onOpenRight,
  onNextDiff,
  onPreviousDiff,
  onExport,
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
      <button onClick={onOpenLeft} title="打开左侧文件 (Ctrl+O)">
        📂 打开左侧
      </button>
      <button onClick={onOpenRight} title="打开右侧文件 (Ctrl+Shift+O)">
        📂 打开右侧
      </button>

      <div className="separator" />

      <button onClick={onPreviousDiff} title="上一个差异 (Shift+F8)">
        ↑ 上一个
      </button>
      <button onClick={onNextDiff} title="下一个差异 (F8)">
        ↓ 下一个
      </button>

      <div className="separator" />

      <label className="checkbox-label">
        <input
          type="checkbox"
          checked={ignoreWhitespace}
          onChange={handleWhitespaceChange}
        />
        忽略空白
      </label>

      <label className="checkbox-label">
        <input
          type="checkbox"
          checked={ignoreCase}
          onChange={handleCaseChange}
        />
        忽略大小写
      </label>

      <div className="separator" />

      <button onClick={onExport} title="导出差异 (Ctrl+S)">
        💾 导出差异
      </button>

      <div className="stats">
        ➕ {diffStats.additions} | ➖ {diffStats.deletions} | ✏️ {diffStats.modifications}
      </div>
    </div>
  );
}
