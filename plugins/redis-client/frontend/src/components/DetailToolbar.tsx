interface Props {
  viewerMode: 'text' | 'hex';
  showSearch: boolean;
  searchQuery: string;
  onViewerModeChange: (m: 'text' | 'hex') => void;
  onSearchChange: (q: string) => void;
  onSearchToggle: () => void;
}

export function DetailToolbar({ viewerMode, showSearch, searchQuery, onViewerModeChange, onSearchChange, onSearchToggle }: Props) {
  return (
    <div className="detail-toolbar">
      <div className="toolbar-left">
        <button className={viewerMode === 'text' ? 'active' : ''} onClick={() => onViewerModeChange('text')}>Text</button>
        <button className={viewerMode === 'hex' ? 'active' : ''} onClick={() => onViewerModeChange('hex')}>HEX</button>
      </div>
      <div className="toolbar-right">
        <button className={showSearch ? 'active' : ''} onClick={onSearchToggle}>🔍 搜索</button>
        {showSearch && (
          <input type="text" value={searchQuery} onChange={e => onSearchChange(e.target.value)}
            placeholder="搜索值…" autoFocus />
        )}
      </div>
    </div>
  );
}
