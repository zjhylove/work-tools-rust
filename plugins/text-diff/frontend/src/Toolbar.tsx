import './Toolbar.css';

interface DiffStats {
  additions: number;
  deletions: number;
  modifications: number;
}

interface ToolbarProps {
  originalFileName: string;
  modifiedFileName: string;
  onOpenOriginal: () => void;
  onOpenModified: () => void;
  stats?: DiffStats;
}

export function Toolbar({
  originalFileName,
  modifiedFileName,
  onOpenOriginal,
  onOpenModified,
  stats
}: ToolbarProps) {
  return (
    <div className="toolbar">
      {/* 左侧文件选择 */}
      <button
        onClick={onOpenOriginal}
        className="wt-btn wt-btn--secondary wt-btn--sm file-select-btn"
        title="打开原始文件"
      >
        <span className="icon">+</span>
        <span className="label">原始</span>
        <span className="filename">{originalFileName || '未选择'}</span>
      </button>

      {/* 中间统计信息 */}
      {stats && (stats.additions > 0 || stats.deletions > 0 || stats.modifications > 0) && (
        <div className="stats-info">
          {stats.additions > 0 && (
            <span className="stat-item added">
              <span className="count">+{stats.additions}</span>
              <span>新增</span>
            </span>
          )}
          {stats.deletions > 0 && (
            <span className="stat-item removed">
              <span className="count">-{stats.deletions}</span>
              <span>删除</span>
            </span>
          )}
          {stats.modifications > 0 && (
            <span className="stat-item modified">
              <span className="count">~{stats.modifications}</span>
              <span>修改</span>
            </span>
          )}
        </div>
      )}

      {/* 右侧文件选择 */}
      <button
        onClick={onOpenModified}
        className="wt-btn wt-btn--secondary wt-btn--sm file-select-btn"
        title="打开修改后的文件"
      >
        <span className="icon">~</span>
        <span className="label">修改</span>
        <span className="filename">{modifiedFileName || '未选择'}</span>
      </button>
    </div>
  );
}
