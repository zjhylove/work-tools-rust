import { useState, useEffect } from 'react';
import { TreeNode } from '../types';
import { KeyTree } from './KeyTree';
import { ContextMenu, ContextMenuAction } from './ContextMenu';

interface Props {
  tree: TreeNode[];
  selectedKey: string | null;
  expandedPaths: Set<string>;
  scanLoading: boolean;
  hasScanned: boolean;
  nextCursor: number;
  searchPattern: string;
  multiSelect: Set<string>;
  onToggle: (p: string) => void;
  onSelect: (k: string) => void;
  onMultiToggle: (k: string) => void;
  onScan: (pattern: string) => void;
  onLoadMore: () => void;
  onDeleteSelected: () => void;
  onDeleteKey: (key: string) => void;
  onDeleteFolder: (prefix: string) => void;
  onLoadFolder: (prefix: string) => void;
}

export function KeyPanel({ tree, selectedKey, expandedPaths, scanLoading, hasScanned, nextCursor,
  searchPattern, multiSelect, onToggle, onSelect, onMultiToggle, onScan, onLoadMore, onDeleteSelected,
  onDeleteKey, onDeleteFolder, onLoadFolder }: Props) {
  const [search, setSearch] = useState(searchPattern);
  const [batchMode, setBatchMode] = useState(false);
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number; node: TreeNode } | null>(null);

  useEffect(() => {
    setSearch(searchPattern);
  }, [searchPattern]);

  const handleSearch = () => {
    if (scanLoading) return;
    onScan(search || '*');
  };

  const handleContextMenu = (e: React.MouseEvent, node: TreeNode) => {
    e.preventDefault();
    setCtxMenu({ x: e.clientX, y: e.clientY, node });
  };

  const getCtxActions = (): ContextMenuAction[] => {
    if (!ctxMenu) return [];
    const node = ctxMenu.node;
    const isFolder = node.fullKey === null;
    if (isFolder) {
      const displayName = node.prefix.replace(/:$/, '');
      const prefix = node.prefix;
      return [
        { label: `仅加载 "${displayName}" 下的 key`, onClick: () => onLoadFolder(prefix) },
        { label: `删除 "${displayName}" 下所有 key`, danger: true, onClick: () => onDeleteFolder(prefix) },
      ];
    }
    return [
      { label: `删除 key`, danger: true, onClick: () => node.fullKey && onDeleteKey(node.fullKey) },
    ];
  };

  return (
    <div className="key-panel">
      <div className="panel-header">
        <input type="text" value={search} onChange={e => setSearch(e.target.value)}
          placeholder="搜索 key (* 通配)" onKeyDown={e => { if (e.key === 'Enter') handleSearch(); }} />
        <button onClick={handleSearch} disabled={scanLoading} title="搜索">🔍</button>
        <button className={batchMode ? 'active' : ''} onClick={() => { setBatchMode(!batchMode); }}
          title="批量选择">☐</button>
      </div>

      {batchMode && multiSelect.size > 0 && (
        <div className="batch-bar">
          <span>已选 {multiSelect.size} 项</span>
          <button className="btn-danger-text" onClick={onDeleteSelected}>删除选中</button>
          <button onClick={() => setBatchMode(false)}>取消</button>
        </div>
      )}

      {scanLoading && !tree.length ? (
        <div className="list-status"><span className="spinner" />扫描中…</div>
      ) : tree.length > 0 ? (
        <>
          <KeyTree tree={tree} selectedKey={selectedKey} expandedPaths={expandedPaths}
            multiSelect={batchMode} selectedSet={multiSelect}
            onToggle={onToggle} onSelect={onSelect} onMultiToggle={onMultiToggle}
            onContextMenu={handleContextMenu} />
          {nextCursor !== 0 && (
            <button className="btn-load-more" onClick={onLoadMore} disabled={scanLoading}>
              {scanLoading ? '加载中…' : '加载更多'}
            </button>
          )}
        </>
      ) : hasScanned ? (
        <div className="list-status">无匹配的 Key</div>
      ) : (
        <div className="list-status">输入 pattern 后搜索</div>
      )}

      {ctxMenu && (
        <ContextMenu x={ctxMenu.x} y={ctxMenu.y} actions={getCtxActions()} onClose={() => setCtxMenu(null)} />
      )}
    </div>
  );
}
