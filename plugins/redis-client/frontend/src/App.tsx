import { useState, useCallback, useEffect } from 'react';
import { AppView, SavedConnection } from './types';
import { ConnectView } from './components/ConnectView';
import { WorkspaceView } from './components/WorkspaceView';
import { ConnectionManager } from './components/ConnectionManager';
import { call } from './api';
import './App.css';

function App() {
  const [view, setView] = useState<AppView>('connect');
  const [savedConns, setSavedConns] = useState<SavedConnection[]>([]);
  const [editConnId, setEditConnId] = useState<string | null>(null);
  const [currentConnectionId, setCurrentConnectionId] = useState<string | null>(null);

  const loadSavedConns = useCallback(async () => {
    const r = await call('list_connections');
    if (r.connections) setSavedConns(r.connections as SavedConnection[]);
  }, []);

  useEffect(() => { loadSavedConns(); }, [loadSavedConns]);

  const handleConnect = useCallback(async (id: string, password?: string) => {
    await call('connect', { id, password });
    setCurrentConnectionId(id);
    setView('workspace');
  }, []);

  const handleQuickConnect = useCallback(async (_host: string, _port: number, _db: number, _password: string) => {
    setCurrentConnectionId(null);
    setView('workspace');
  }, []);

  const handleDisconnect = useCallback(async () => {
    await call('disconnect');
    setCurrentConnectionId(null);
    setView('connect');
  }, []);

  const handleDeleteConn = useCallback(async (id: string) => {
    await call('delete_connection', { id });
    loadSavedConns();
  }, [loadSavedConns]);

  switch (view) {
    case 'workspace':
      return (
        <WorkspaceView
          savedConns={savedConns} currentConnectionId={currentConnectionId}
          onDisconnect={handleDisconnect} onManage={() => setView('manager')}
          onConnect={handleConnect} />
      );
    case 'manager':
      return (
        <ConnectionManager
          savedConns={savedConns} onBack={() => setView('connect')}
          onSave={loadSavedConns} onDelete={handleDeleteConn}
          editId={editConnId} onEditStart={setEditConnId} />
      );
    default:
      return (
        <ConnectView
          savedConns={savedConns} onConnect={handleConnect}
          onQuickConnect={handleQuickConnect} onManage={() => setView('manager')} />
      );
  }
}

export default App;
