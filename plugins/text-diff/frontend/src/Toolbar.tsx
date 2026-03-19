import './Toolbar.css';

interface ToolbarProps {
  originalFileName: string;
  modifiedFileName: string;
  onOpenOriginal: () => void;
  onOpenModified: () => void;
}

export function Toolbar({
  originalFileName,
  modifiedFileName,
  onOpenOriginal,
  onOpenModified
}: ToolbarProps) {
  return (
    <div className="toolbar">
      {/* 左侧文件选择 */}
      <button
        onClick={onOpenOriginal}
        className="file-select-btn"
      >
        <span>📄</span>
        <span>打开文件</span>
        <span style={{ color: '#666', fontSize: '12px', marginLeft: '8px' }}>
          {originalFileName || '未选择'}
        </span>
      </button>

      {/* 右侧文件选择 */}
      <button
        onClick={onOpenModified}
        className="file-select-btn"
      >
        <span>📝</span>
        <span>打开文件</span>
        <span style={{ color: '#666', fontSize: '12px', marginLeft: '8px' }}>
          {modifiedFileName || '未选择'}
        </span>
      </button>
    </div>
  );
}
