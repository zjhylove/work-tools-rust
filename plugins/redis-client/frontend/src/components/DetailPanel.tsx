import { useState } from 'react';
import { DetailToolbar } from './DetailToolbar';
import { StringViewer } from './viewers/StringViewer';
import { HashViewer } from './viewers/HashViewer';
import { ListViewer } from './viewers/ListViewer';
import { SetViewer } from './viewers/SetViewer';
import { ZSetViewer } from './viewers/ZSetViewer';
import { HexViewer } from './viewers/HexViewer';

interface Props {
  selectedKey: string | null;
  keyDetail: Record<string, unknown> | null;
  valueData: unknown;
  detailLoading: boolean;
  onDeleteKey: () => void;
  onSaveString: (value: string) => void;
  onSetHashField: (field: string, value: string) => void;
  onDelHashField: (field: string) => void;
}

export function DetailPanel({ selectedKey, keyDetail, valueData, detailLoading,
  onDeleteKey, onSaveString, onSetHashField, onDelHashField }: Props) {
  const [viewerMode, setViewerMode] = useState<'text' | 'hex'>('text');
  const [showSearch, setShowSearch] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [hashMultiSelect, setHashMultiSelect] = useState<Set<string>>(new Set());

  if (detailLoading) return <div className="detail-loading"><span className="spinner" />加载中…</div>;
  if (!selectedKey || !keyDetail) return <div className="empty-detail">选择一个 Key 查看详情</div>;

  const kType = keyDetail.type as string;
  const ttl = keyDetail.ttl as number;
  const ttlText = ttl === -1 ? '永不过期' : ttl === -2 ? '已过期' : `${ttl}s`;

  return (
    <div className="detail-panel">
      <div className="detail-header">
        <h4>{selectedKey}</h4>
        <span className="type-badge">{kType}</span>
        <span className="ttl-badge">TTL: {ttlText}</span>
        <button className="btn-delete-key" onClick={onDeleteKey}>删除</button>
      </div>

      <DetailToolbar viewerMode={viewerMode} showSearch={showSearch} searchQuery={searchQuery}
        onViewerModeChange={setViewerMode} onSearchChange={setSearchQuery} onSearchToggle={() => setShowSearch(!showSearch)} />

      {viewerMode === 'hex' ? (
        <HexViewer selectedKey={selectedKey} />
      ) : (
        <>
          {kType === 'string' && valueData && (
            <StringViewer value={valueData as { value: string }} selectedKey={selectedKey} onSave={onSaveString} />
          )}
          {kType === 'hash' && valueData && (
            <HashViewer fields={(valueData as { fields: Record<string, string> }).fields}
              selectedKey={selectedKey} onSetField={onSetHashField} onDelField={onDelHashField}
              searchQuery={searchQuery}
              multiSelect={hashMultiSelect} onMultiToggle={f => {
                setHashMultiSelect(prev => { const n = new Set(prev); n.has(f) ? n.delete(f) : n.add(f); return n; });
              }}
              onDeleteSelected={() => {
                hashMultiSelect.forEach(f => onDelHashField(f));
                setHashMultiSelect(new Set());
              }} />
          )}
          {kType === 'list' && valueData && (
            <ListViewer items={(valueData as { items: string[] }).items} searchQuery={searchQuery} />
          )}
          {kType === 'set' && valueData && (
            <SetViewer members={(valueData as { members: string[] }).members} searchQuery={searchQuery} />
          )}
          {kType === 'zset' && valueData && (
            <ZSetViewer members={(valueData as { members: Array<{ member: string; score: number }> }).members} searchQuery={searchQuery} />
          )}
        </>
      )}
    </div>
  );
}
