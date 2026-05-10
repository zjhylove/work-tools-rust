import { useState, useEffect, useCallback } from 'react';
import './App.css';
import type { ConnectionConfig, ObjectInfo, ListObjectsResult } from './models';

declare global {
  var WorkTools: {
    toast: {
      success(m: string): void;
      error(m: string): void;
      info(m: string): void;
      warning(m: string): void;
    };
    FieldError: {
      show(el: HTMLElement, m: string): void;
      clear(el: HTMLElement): void;
      clearAll(f: HTMLElement): void;
    };
  };
}

const PLUGIN_ID = 'object-storage';

const EMPTY_FORM = {
  name: '', provider: 'aliyun', access_key: '', secret_key: '', region: 'oss-cn-hangzhou', bucket: '', endpoint: '',
};

function App() {
  const [connections, setConnections] = useState<ConnectionConfig[]>([]);
  const [selectedConnId, setSelectedConnId] = useState<string>('');
  const [objects, setObjects] = useState<ObjectInfo[]>([]);
  const [currentPrefix, setCurrentPrefix] = useState<string>('');
  const [search, setSearch] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [editingConnId, setEditingConnId] = useState<string | null>(null);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState('');
  const [showDeleteConnConfirm, setShowDeleteConnConfirm] = useState(false);

  const [connForm, setConnForm] = useState(EMPTY_FORM);

  useEffect(() => { loadConnections(); }, []);

  const api = useCallback(
    (method: string, params: Record<string, unknown> = {}) =>
      window.pluginAPI?.call(PLUGIN_ID, method, params) as Promise<any>,
    []
  );

  const loadConnections = async () => {
    try {
      const list = (await api('list_connections')) as ConnectionConfig[];
      setConnections(list);
    } catch (e) { WorkTools.toast.error('加载连接失败: ' + (e as Error).message); }
  };

  const handleSelectConn = async (id: string) => {
    setSelectedConnId(id);
    setObjects([]);
    setCurrentPrefix('');
    if (!id) return;

    const conn = connections.find((c) => c.id === id);
    if (!conn || !conn.bucket) return;
    await loadObjects(id, conn.bucket, '');
  };

  const handleNavigate = async (prefix: string) => {
    setCurrentPrefix(prefix);
    const conn = connections.find((c) => c.id === selectedConnId);
    if (conn?.bucket) await loadObjects(selectedConnId, conn.bucket, prefix);
  };

  const handleGoUp = async () => {
    const parts = currentPrefix.split('/').filter(Boolean);
    parts.pop();
    const parent = parts.length > 0 ? parts.join('/') + '/' : '';
    setCurrentPrefix(parent);
    const conn = connections.find((c) => c.id === selectedConnId);
    if (conn?.bucket) await loadObjects(selectedConnId, conn.bucket, parent);
  };

  const loadObjects = async (connId: string, bucket: string, prefix: string) => {
    if (!bucket || !connId) return;
    setLoading(true);
    try {
      const result = (await api('list_objects', {
        connection_id: connId,
        bucket,
        prefix,
        delimiter: '/',
        max_keys: 200,
      })) as ListObjectsResult;

      const dirs: ObjectInfo[] = result.prefixes.map((p) => ({
        key: p, size: 0, last_modified: '', etag: '', is_dir: true,
      }));
      setObjects([...dirs, ...result.objects]);
    } catch (e) {
      WorkTools.toast.error('列举对象失败: ' + (e as Error).message);
    } finally { setLoading(false); }
  };

  const handleRefresh = async () => {
    const conn = connections.find((c) => c.id === selectedConnId);
    if (conn?.bucket) await loadObjects(selectedConnId, conn.bucket, currentPrefix);
  };

  const handleDelete = (key: string) => {
    setDeleteTarget(key);
    setShowDeleteConfirm(true);
  };

  const handleConfirmDelete = async () => {
    const conn = connections.find((c) => c.id === selectedConnId);
    if (!conn?.bucket || !deleteTarget) return;
    setShowDeleteConfirm(false);
    setLoading(true);
    try {
      await api('delete_object', { connection_id: selectedConnId, bucket: conn.bucket, key: deleteTarget });
      WorkTools.toast.success('删除成功');
      await loadObjects(selectedConnId, conn.bucket, currentPrefix);
    } catch (e) { WorkTools.toast.error('删除失败: ' + (e as Error).message); } finally { setLoading(false); }
  };

  const handleUpload = async () => {
    const filePath = await window.pluginAPI?.open_file_dialog('选择要上传的文件');
    if (!filePath) return;
    const conn = connections.find((c) => c.id === selectedConnId);
    if (!conn?.bucket) return;
    const fileName = filePath.split(/[/\\]/).pop() || 'file';
    const key = currentPrefix + fileName;
    setLoading(true);
    try {
      await api('upload_object', { connection_id: selectedConnId, bucket: conn.bucket, key, file_path: filePath });
      WorkTools.toast.success('上传成功');
      await loadObjects(selectedConnId, conn.bucket, currentPrefix);
    } catch (e) { WorkTools.toast.error('上传失败: ' + (e as Error).message); } finally { setLoading(false); }
  };

  const handleDownload = async (key: string) => {
    const conn = connections.find((c) => c.id === selectedConnId);
    if (!conn?.bucket) return;
    const fileName = key.split('/').pop() || 'file';
    const dir = await window.pluginAPI?.open_folder_dialog('选择下载目录');
    if (!dir) return;
    const filePath = dir + '/' + fileName;
    setLoading(true);
    try {
      await api('download_object', { connection_id: selectedConnId, bucket: conn.bucket, key, file_path: filePath });
      WorkTools.toast.success('下载完成: ' + filePath);
    } catch (e) { WorkTools.toast.error('下载失败: ' + (e as Error).message); } finally { setLoading(false); }
  };

  const handleEditConn = async () => {
    if (!selectedConnId) return;
    try {
      const data = (await api('get_connection', { id: selectedConnId })) as any;
      setConnForm({ name: data.name || '', provider: data.provider || 'aliyun', access_key: data.access_key || '', secret_key: data.secret_key || '', region: data.region || '', bucket: data.bucket || '', endpoint: data.endpoint || '' });
      setEditingConnId(selectedConnId);
      setShowForm(true);
    } catch (e) { WorkTools.toast.error('获取连接信息失败: ' + (e as Error).message); }
  };

  const handleSaveConnection = async () => {
    let valid = true;
    if (!connForm.name.trim()) {
      const el = document.querySelector('.conn-name-input') as HTMLInputElement;
      if (el) WorkTools.FieldError.show(el, '连接名称不能为空');
      valid = false;
    }
    if (!connForm.access_key.trim()) {
      const el = document.querySelector('.conn-ak-input') as HTMLInputElement;
      if (el) WorkTools.FieldError.show(el, 'Access Key 不能为空');
      valid = false;
    }
    if (!valid) return;

    try {
      setLoading(true);
      if (editingConnId) {
        await api('update_connection', { id: editingConnId, ...connForm });
        WorkTools.toast.success('连接已更新');
      } else {
        await api('add_connection', connForm);
        WorkTools.toast.success('连接已保存');
      }
      setShowForm(false); setEditingConnId(null); setConnForm(EMPTY_FORM);
      await loadConnections();
    } catch (e) { WorkTools.toast.error((editingConnId ? '更新' : '添加') + '连接失败: ' + (e as Error).message); } finally { setLoading(false); }
  };

  const handleDeleteConn = () => {
    setShowDeleteConnConfirm(true);
  };

  const handleConfirmDeleteConn = async () => {
    if (!selectedConnId) return;
    setShowDeleteConnConfirm(false);
    try {
      await api('delete_connection', { id: selectedConnId });
      setSelectedConnId(''); setObjects([]);
      WorkTools.toast.success('连接已删除');
      await loadConnections();
    } catch (e) { WorkTools.toast.error('删除连接失败: ' + (e as Error).message); }
  };

  const currentConn = connections.find((c) => c.id === selectedConnId);
  const filteredObjects = objects.filter((o) =>
    search ? o.key.toLowerCase().includes(search.toLowerCase()) : true
  );
  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  };

  return (
    <div className="object-storage">
      {showDeleteConfirm && (
        <div className="wt-modal-overlay" onClick={() => setShowDeleteConfirm(false)}>
          <div className="wt-modal" onClick={e => e.stopPropagation()}>
            <div className="wt-modal-header">
              <h3>确认删除</h3>
            </div>
            <div className="wt-modal-body">
              确定要删除 "{deleteTarget}" 吗？此操作不可撤销。
            </div>
            <div className="wt-modal-footer">
              <button className="wt-btn wt-btn--secondary" onClick={() => setShowDeleteConfirm(false)}>取消</button>
              <button className="wt-btn wt-btn--danger" onClick={handleConfirmDelete}>删除</button>
            </div>
          </div>
        </div>
      )}

      {showDeleteConnConfirm && (
        <div className="wt-modal-overlay" onClick={() => setShowDeleteConnConfirm(false)}>
          <div className="wt-modal" onClick={e => e.stopPropagation()}>
            <div className="wt-modal-header">
              <h3>确认删除连接</h3>
            </div>
            <div className="wt-modal-body">
              确定要删除此连接吗？此操作不可撤销。
            </div>
            <div className="wt-modal-footer">
              <button className="wt-btn wt-btn--secondary" onClick={() => setShowDeleteConnConfirm(false)}>取消</button>
              <button className="wt-btn wt-btn--danger" onClick={handleConfirmDeleteConn}>删除</button>
            </div>
          </div>
        </div>
      )}

      <div className="toolbar">
        <select value={selectedConnId} onChange={(e) => handleSelectConn(e.target.value)}>
          <option value="">-- 选择连接 --</option>
          {connections.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name} / {c.bucket} ({c.provider === 'aliyun' ? '阿里云' : '腾讯云'})
            </option>
          ))}
        </select>
        <button className="wt-btn wt-btn--secondary" onClick={() => { setEditingConnId(null); setConnForm(EMPTY_FORM); setShowForm(!showForm); }}>+ 添加连接</button>
        {selectedConnId && (
          <>
            <button className="wt-btn wt-btn--secondary" onClick={handleEditConn}>编辑连接</button>
            <button className="wt-btn wt-btn--secondary" onClick={handleDeleteConn}>删除连接</button>
          </>
        )}
        <span className="spacer" />
        <button className="btn-icon" onClick={handleRefresh} disabled={!selectedConnId} title="刷新">↻</button>
      </div>

      {showForm && (
        <div className="conn-form">
          <h3>{editingConnId ? '编辑连接' : '添加云服务连接'}</h3>
          <div className="form-row"><label>名称</label><input className="conn-name-input" value={connForm.name} onChange={(e) => setConnForm({ ...connForm, name: e.target.value })} onInput={(e) => WorkTools.FieldError.clear(e.currentTarget as HTMLInputElement)} placeholder="我的阿里云" /></div>
          <div className="form-row"><label>服务商</label><select value={connForm.provider} onChange={(e) => setConnForm({ ...connForm, provider: e.target.value })}><option value="aliyun">阿里云 OSS</option><option value="tencent">腾讯云 COS</option></select></div>
          <div className="form-row"><label>AccessKey</label><input className="conn-ak-input" value={connForm.access_key} onChange={(e) => setConnForm({ ...connForm, access_key: e.target.value })} onInput={(e) => WorkTools.FieldError.clear(e.currentTarget as HTMLInputElement)} placeholder="AccessKey ID" /></div>
          <div className="form-row"><label>SecretKey</label><input type="password" value={connForm.secret_key} onChange={(e) => setConnForm({ ...connForm, secret_key: e.target.value })} placeholder="AccessKey Secret" /></div>
          <div className="form-row"><label>Region</label><input value={connForm.region} onChange={(e) => setConnForm({ ...connForm, region: e.target.value })} placeholder="oss-cn-hangzhou" /></div>
          <div className="form-row"><label>Bucket</label><input value={connForm.bucket} onChange={(e) => setConnForm({ ...connForm, bucket: e.target.value })} placeholder="my-bucket" /></div>
          <div className="form-row"><label>Endpoint</label><input value={connForm.endpoint} onChange={(e) => setConnForm({ ...connForm, endpoint: e.target.value })} placeholder="如 oss-cn-hangzhou.aliyuncs.com" /></div>
          <div className="form-actions">
            <button className="wt-btn wt-btn--primary" onClick={handleSaveConnection} disabled={!connForm.name || !connForm.access_key || !connForm.secret_key || !connForm.bucket}>
              {editingConnId ? '更新连接' : '保存连接'}
            </button>
            <button className="wt-btn wt-btn--secondary" onClick={() => { setShowForm(false); setEditingConnId(null); }}>取消</button>
          </div>
        </div>
      )}

      <div className="main">
        <div className="object-panel" style={{ flex: 1 }}>
          {selectedConnId && currentConn?.bucket ? (
            <>
              <div className="object-toolbar">
                <div className="path-nav">
                  <span className="path-link" onClick={() => handleNavigate('')}>{currentConn.bucket}</span>
                  {currentPrefix.split('/').filter(Boolean).map((part, i, arr) => (
                    <span key={i}>
                      <span className="path-sep">/</span>
                      <span className="path-link" onClick={() => handleNavigate(arr.slice(0, i + 1).join('/') + '/')}>{part}</span>
                    </span>
                  ))}
                </div>
                <input className="search-input" placeholder="搜索文件..." value={search} onChange={(e) => setSearch(e.target.value)} />
                <button className="wt-btn wt-btn--primary" onClick={handleUpload}>上传文件</button>
              </div>

              {currentPrefix && <div className="go-up" onClick={handleGoUp}>返回上级目录</div>}

              <div className="table-scroll">
                <table className="obj-table">
                  <thead>
                    <tr><th>名称</th><th style={{ width: 100 }}>大小</th><th style={{ width: 180 }}>修改时间</th><th style={{ width: 100 }}>操作</th></tr>
                  </thead>
                  <tbody>
                    {filteredObjects.map((o, i) => (
                      <tr key={o.key}>
                        <td>
                          {o.is_dir ? <span className="obj-link" onClick={() => handleNavigate(o.key)}>{o.key.replace(currentPrefix, '')}</span>
                            : <span>{o.key.replace(currentPrefix, '')}</span>}
                        </td>
                        <td>{o.is_dir ? '-' : formatSize(o.size)}</td>
                        <td>{o.last_modified ? new Date(o.last_modified).toLocaleString('zh-CN') : ''}</td>
                        <td>
                          {!o.is_dir && <button className="wt-btn wt-btn--sm" onClick={() => handleDownload(o.key)}>下载</button>}
                          <button className="wt-btn wt-btn--sm wt-btn--danger" onClick={() => handleDelete(o.key)}>删除</button>
                        </td>
                      </tr>
                    ))}
                    {filteredObjects.length === 0 && !loading && <tr><td colSpan={4} className="empty-tip">暂无文件</td></tr>}
                  </tbody>
                </table>
              </div>
            </>
          ) : (
            <div className="empty-state">
              <div className="empty-title">开始使用对象存储</div>
              <div className="empty-desc">添加云服务连接，浏览和管理您的 OSS/COS 文件</div>
            </div>
          )}
          {loading && <div className="loading">加载中...</div>}
        </div>
      </div>
    </div>
  );
}

export default App;
