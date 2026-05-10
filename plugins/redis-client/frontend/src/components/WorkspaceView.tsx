import { useState, useCallback } from 'react';
import { SavedConnection } from '../types';
import { ConnectionBar } from './ConnectionBar';
import { KeyPanel } from './KeyPanel';
import { DetailPanel } from './DetailPanel';
import { useKeys } from '../hooks/useKeys';
import { useKeyDetail } from '../hooks/useKeyDetail';
import { call } from '../api';
import { useToast } from './Toast';

interface Props {
  savedConns: SavedConnection[];
  currentConnectionId: string | null;
  onDisconnect: () => void;
  onManage: () => void;
  onConnect: (id: string) => void;
}

export function WorkspaceView({ savedConns, currentConnectionId, onDisconnect, onManage, onConnect }: Props) {
  const { tree, nextCursor, scanLoading, hasScanned, expandedPaths,
    togglePath, scan, deleteSelectedKeys } = useKeys();
  const { selectedKey, keyDetail, valueData, detailLoading, selectKey, refresh } = useKeyDetail();
  const [multiSelect, setMultiSelect] = useState<Set<string>>(new Set());
  const [pattern, setPattern] = useState('*');
  const [deleteProgress, setDeleteProgress] = useState<{ prefix: string; scanned: number; phase: 'scanning' | 'deleting' } | null>(null);
  const { showToast } = useToast();

  const handleScan = useCallback((p: string) => {
    setPattern(p);
    scan(p, false);
  }, [scan]);

  const handleLoadMore = useCallback(() => {
    if (nextCursor === 0 || scanLoading) return;
    scan(pattern, true);
  }, [nextCursor, scanLoading, scan, pattern]);

  const handleSaveString = useCallback(async (value: string) => {
    if (!selectedKey) return;
    await call('set_string', { key: selectedKey, value });
    refresh();
  }, [selectedKey, refresh]);

  const handleSetHashField = useCallback(async (field: string, value: string) => {
    if (!selectedKey) return;
    await call('set_hash_field', { key: selectedKey, field, value });
    refresh();
  }, [selectedKey, refresh]);

  const handleDelHashField = useCallback(async (field: string) => {
    if (!selectedKey) return;
    await call('del_hash_field', { key: selectedKey, field });
    refresh();
  }, [selectedKey, refresh]);

  const handleDeleteKey = useCallback(async () => {
    if (!selectedKey) return;
    await call('delete_key', { key: selectedKey });
    selectKey(null);
    scan(pattern, false);
  }, [selectedKey, scan, pattern, selectKey]);

  const handleDeleteSelected = useCallback(async () => {
    await deleteSelectedKeys(Array.from(multiSelect));
    setMultiSelect(new Set());
    scan(pattern, false);
  }, [multiSelect, deleteSelectedKeys, scan, pattern]);

  const handleLoadFolder = useCallback((prefix: string) => {
    const folderPattern = `${prefix}*`;
    setPattern(folderPattern);
    scan(folderPattern, false);
  }, [scan]);

  const handleCtxDeleteKey = useCallback(async (key: string) => {
    try {
      await call('delete_key', { key });
      if (selectedKey === key) selectKey(null);
      scan(pattern, false);
    } catch (e) {
      showToast(`删除失败: ${e}`, 'error');
    }
  }, [selectedKey, selectKey, scan, pattern, showToast]);

  const handleCtxDeleteFolder = useCallback(async (prefix: string) => {
    setDeleteProgress({ prefix, scanned: 0, phase: 'scanning' });
    try {
      const folderPattern = `${prefix}*`;
      let cursor: number = 0;
      let allKeys: string[] = [];
      do {
        const r = await call('scan_keys', { pattern: folderPattern, count: 2000, cursor });
        const keyInfos = (r.keys as { key: string }[]) || [];
        allKeys = allKeys.concat(keyInfos.map(ki => ki.key));
        cursor = (r.cursor as number) || 0;
        setDeleteProgress({ prefix, scanned: allKeys.length, phase: 'scanning' });
      } while (cursor !== 0);

      if (allKeys.length > 0) {
        setDeleteProgress({ prefix, scanned: allKeys.length, phase: 'deleting' });
        for (let i = 0; i < allKeys.length; i += 500) {
          const batch = allKeys.slice(i, i + 500);
          await call('delete_keys', { keys: batch });
        }
        showToast(`已删除 ${allKeys.length} 个 key`, 'success');
      } else {
        showToast('该目录下没有 key', 'info');
      }
    } catch (e) {
      showToast(`删除失败: ${e}`, 'error');
    }
    setDeleteProgress(null);
    scan(pattern, false);
  }, [scan, pattern, showToast]);

  return (
    <>
      <div className="main-layout">
        <div className="left-panel">
          <ConnectionBar savedConns={savedConns} currentId={currentConnectionId}
            onConnect={onConnect} onDisconnect={onDisconnect} onManage={onManage} />
          <KeyPanel tree={tree} selectedKey={selectedKey} expandedPaths={expandedPaths}
            searchPattern={pattern}
            multiSelect={multiSelect} scanLoading={scanLoading} hasScanned={hasScanned}
            nextCursor={nextCursor}
            onToggle={togglePath} onSelect={selectKey}
            onMultiToggle={k => {
              setMultiSelect(prev => { const n = new Set(prev); n.has(k) ? n.delete(k) : n.add(k); return n; });
            }}
            onScan={handleScan} onLoadMore={handleLoadMore}
            onDeleteSelected={handleDeleteSelected}
            onDeleteKey={handleCtxDeleteKey}
            onDeleteFolder={handleCtxDeleteFolder}
            onLoadFolder={handleLoadFolder} />
        </div>
        <DetailPanel selectedKey={selectedKey} keyDetail={keyDetail} valueData={valueData}
          detailLoading={detailLoading}
          onDeleteKey={handleDeleteKey} onSaveString={handleSaveString}
          onSetHashField={handleSetHashField} onDelHashField={handleDelHashField} />
      </div>

      {deleteProgress && (
        <div className="modal-overlay">
          <div className="modal-content modal-sm">
            <div className="modal-body delete-confirm-body">
              <div className="delete-warning-icon">{deleteProgress.phase === 'scanning' ? '🔍' : '🗑'}</div>
              <p>{deleteProgress.phase === 'scanning'
                ? `正在扫描 "${deleteProgress.prefix}" 下的 key…`
                : `正在删除 "${deleteProgress.prefix}" 下的 key…`}</p>
              <p className="delete-warning-hint" style={{ fontFamily: 'var(--font-mono)' }}>
                {deleteProgress.phase === 'scanning'
                  ? `已发现 ${deleteProgress.scanned} 个 key`
                  : `共 ${deleteProgress.scanned} 个 key，批量删除中…`}
              </p>
            </div>
          </div>
        </div>
      )}

    </>
  );
}
