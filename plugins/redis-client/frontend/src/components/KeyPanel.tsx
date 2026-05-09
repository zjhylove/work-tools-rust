import { useState } from 'react';
import { TreeNode } from '../types';
import { KeyTree } from './KeyTree';

interface Props {
  tree: TreeNode[];
  selectedKey: string | null;
  expandedPaths: Set<string>;
  scanLoading: boolean;
  hasScanned: boolean;
  nextCursor: number;
  multiSelect: Set<string>;
  onToggle: (p: string) => void;
  onSelect: (k: string) => void;
  onMultiToggle: (k: string) => void;
  onScan: (pattern: string) => void;
  onLoadMore: () => void;
  onDeleteSelected: () => void;
}

export function KeyPanel({ tree, selectedKey, expandedPaths, scanLoading, hasScanned, nextCursor,
  multiSelect, onToggle, onSelect, onMultiToggle, onScan, onLoadMore, onDeleteSelected }: Props) {
  const [search, setSearch] = useState('*');
  const [batchMode, setBatchMode] = useState(false);

  return (
    <div className="key-panel">
      <div className="panel-header">
        <input type="text" value={search} onChange={e => setSearch(e.target.value)}
          placeholder="搜索 key (* 通配)" onKeyDown={e => e.key === 'Enter' && onScan(search)} />
        <button onClick={() => onScan(search)} disabled={scanLoading} title="搜索">🔍</button>
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
            onToggle={onToggle} onSelect={onSelect} onMultiToggle={onMultiToggle} />
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
