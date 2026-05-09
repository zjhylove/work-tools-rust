import { useState } from 'react';
import { TreeNode } from '../types';
import { KeyTree } from './KeyTree';

interface Props {
  tree: TreeNode[];
  selectedKey: string | null;
  expandedPaths: Set<string>;
  multiSelect: Set<string>;
  scanLoading: boolean;
  hasScanned: boolean;
  nextCursor: number;
  onToggle: (p: string) => void;
  onSelect: (k: string) => void;
  onMultiToggle: (k: string) => void;
  onScan: (pattern: string) => void;
  onLoadMore: () => void;
  onDeleteSelected: () => void;
}

export function KeyPanel({ tree, selectedKey, expandedPaths, multiSelect, scanLoading, hasScanned, nextCursor,
  onToggle, onSelect, onMultiToggle, onScan, onLoadMore, onDeleteSelected }: Props) {
  const [search, setSearch] = useState('*');

  return (
    <div className="key-panel">
      <div className="panel-header">
        <input type="text" value={search} onChange={e => setSearch(e.target.value)}
          placeholder="搜索 key (* 通配)" onKeyDown={e => e.key === 'Enter' && onScan(search)} />
        <button onClick={() => onScan(search)} disabled={scanLoading}>🔍</button>
        {multiSelect.size > 0 && (
          <button onClick={onDeleteSelected} title="删除选中">🗑</button>
        )}
      </div>

      {scanLoading && !tree.length ? (
        <div className="list-status"><span className="spinner" />扫描中…</div>
      ) : tree.length > 0 ? (
        <>
          <KeyTree tree={tree} selectedKey={selectedKey} expandedPaths={expandedPaths}
            multiSelect={multiSelect} onToggle={onToggle} onSelect={onSelect} onMultiToggle={onMultiToggle} />
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
    </div>
  );
}
