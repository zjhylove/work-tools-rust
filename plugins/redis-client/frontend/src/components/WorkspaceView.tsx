import { useState, useCallback } from 'react';
import { SavedConnection } from '../types';
import { ConnectionBar } from './ConnectionBar';
import { KeyPanel } from './KeyPanel';
import { DetailPanel } from './DetailPanel';
import { useKeys } from '../hooks/useKeys';
import { useKeyDetail } from '../hooks/useKeyDetail';

interface Props {
  savedConns: SavedConnection[];
  currentConnectionId: string | null;
  onDisconnect: () => void;
  onManage: () => void;
  onConnect: (id: string) => void;
}

export function WorkspaceView({ savedConns, currentConnectionId, onDisconnect, onManage, onConnect }: Props) {
  const { keys: _keys, tree, nextCursor, scanLoading, hasScanned, expandedPaths,
    togglePath, scan, deleteSelectedKeys } = useKeys();
  const { selectedKey, keyDetail, valueData, detailLoading, selectKey, refresh } = useKeyDetail();
  const [multiSelect, setMultiSelect] = useState<Set<string>>(new Set());
  const [pattern, setPattern] = useState('*');

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
    await window.pluginAPI?.call('redis-client', 'set_string', { key: selectedKey, value });
    refresh();
  }, [selectedKey, refresh]);

  const handleSetHashField = useCallback(async (field: string, value: string) => {
    if (!selectedKey) return;
    await window.pluginAPI?.call('redis-client', 'set_hash_field', { key: selectedKey, field, value });
    refresh();
  }, [selectedKey, refresh]);

  const handleDelHashField = useCallback(async (field: string) => {
    if (!selectedKey) return;
    await window.pluginAPI?.call('redis-client', 'del_hash_field', { key: selectedKey, field });
    refresh();
  }, [selectedKey, refresh]);

  const handleDeleteKey = useCallback(async () => {
    if (!selectedKey) return;
    await window.pluginAPI?.call('redis-client', 'delete_key', { key: selectedKey });
    scan(pattern, false);
  }, [selectedKey, scan, pattern]);

  const handleDeleteSelected = useCallback(async () => {
    await deleteSelectedKeys(Array.from(multiSelect));
    setMultiSelect(new Set());
    scan(pattern, false);
  }, [multiSelect, deleteSelectedKeys, scan, pattern]);

  return (
    <div className="redis-client">
      <div className="main-layout">
        <div className="left-panel">
          <ConnectionBar savedConns={savedConns} currentId={currentConnectionId}
            onConnect={onConnect} onDisconnect={onDisconnect} onManage={onManage} />
          <KeyPanel tree={tree} selectedKey={selectedKey} expandedPaths={expandedPaths}
            multiSelect={multiSelect} scanLoading={scanLoading} hasScanned={hasScanned}
            nextCursor={nextCursor}
            onToggle={togglePath} onSelect={selectKey}
            onMultiToggle={k => {
              setMultiSelect(prev => { const n = new Set(prev); n.has(k) ? n.delete(k) : n.add(k); return n; });
            }}
            onScan={handleScan} onLoadMore={handleLoadMore}
            onDeleteSelected={handleDeleteSelected} />
        </div>
        <DetailPanel selectedKey={selectedKey} keyDetail={keyDetail} valueData={valueData}
          detailLoading={detailLoading}
          onDeleteKey={handleDeleteKey} onSaveString={handleSaveString}
          onSetHashField={handleSetHashField} onDelHashField={handleDelHashField} />
      </div>
    </div>
  );
}
