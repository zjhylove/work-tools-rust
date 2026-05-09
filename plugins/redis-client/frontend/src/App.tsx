import { useState, useCallback, useEffect } from 'react';
import { AppView, SavedConnection } from './types';
import { ConnectView } from './components/ConnectView';
import { WorkspaceView } from './components/WorkspaceView';
import { ConnectionManager } from './components/ConnectionManager';
import './App.css';

declare global {
  interface Window {
    pluginAPI?: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

function App() {
  const [view, setView] = useState<AppView>('connect');
  const [savedConns, setSavedConns] = useState<SavedConnection[]>([]);
  const [editConnId, setEditConnId] = useState<string | null>(null);
  const [currentConnectionId, setCurrentConnectionId] = useState<string | null>(null);

  const loadSavedConns = useCallback(async () => {
    const r = await window.pluginAPI?.call('redis-client', 'list_connections', {});
    if (r && (r as Record<string, unknown>).connections) {
      setSavedConns((r as { connections: SavedConnection[] }).connections);
    }
  }, []);

  useEffect(() => { loadSavedConns(); }, [loadSavedConns]);

  const handleConnect = useCallback(async (id: string, password?: string) => {
    try {
      await window.pluginAPI?.call('redis-client', 'connect', { id, password });
      setCurrentConnectionId(id);
      setView('workspace');
    } catch (e) { /* handled by useConnection hook */ }
  }, []);

  const handleDisconnect = useCallback(async () => {
    await window.pluginAPI?.call('redis-client', 'disconnect', {});
    setCurrentConnectionId(null);
    setView('connect');
  }, []);

  const handleDeleteConn = useCallback(async (id: string) => {
    await window.pluginAPI?.call('redis-client', 'delete_connection', { id });
    loadSavedConns();
  }, [loadSavedConns]);

  switch (view) {
    case 'workspace':
      return (
        <WorkspaceView
          savedConns={savedConns}
          currentConnectionId={currentConnectionId}
          onDisconnect={handleDisconnect}
          onManage={() => setView('manager')}
          onConnect={handleConnect}
        />
      );
    case 'manager':
      return (
        <ConnectionManager
          savedConns={savedConns}
          onBack={() => setView('connect')}
          onSave={loadSavedConns}
          onDelete={handleDeleteConn}
          editId={editConnId}
          onEditStart={setEditConnId}
        />
      );
    default:
      return (
        <ConnectView
          savedConns={savedConns}
          onConnect={handleConnect}
          onManage={() => setView('manager')}
          onRefresh={loadSavedConns}
        />
      );
  }
}

export default App;
