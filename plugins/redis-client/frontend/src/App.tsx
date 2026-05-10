import { useState, useCallback, useEffect, useRef } from 'react';
import { AppView, SavedConnection } from './types';
import { ConnectView } from './components/ConnectView';
import { WorkspaceView } from './components/WorkspaceView';
import { ConnectionManager } from './components/ConnectionManager';
import { ToastProvider } from './components/Toast';
import { call } from './api';
import './App.css';

const isMac = /Mac/i.test(navigator.userAgent);

function App() {
  const [view, setView] = useState<AppView>('connect');
  const [savedConns, setSavedConns] = useState<SavedConnection[]>([]);
  const [editConnId, setEditConnId] = useState<string | null>(null);
  const [currentConnectionId, setCurrentConnectionId] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);

  const loadSavedConns = useCallback(async () => {
    const r = await call('list_connections');
    if (r.connections) setSavedConns(r.connections as SavedConnection[]);
  }, []);

  useEffect(() => { loadSavedConns(); }, [loadSavedConns]);

  // macOS WKWebView srcdoc iframe skips layout on state-driven view changes.
  // Toggle compositing layer via double rAF to force a full paint cycle.
  useEffect(() => {
    if (!isMac) return;
    const el = rootRef.current;
    if (!el) return;
    let cancelled = false;
    requestAnimationFrame(() => {
      if (cancelled) return;
      el.style.transform = 'translateZ(0)';
      requestAnimationFrame(() => {
        if (cancelled) return;
        el.style.transform = '';
      });
    });
    return () => { cancelled = true; };
  }, [view]);

  const handleConnect = useCallback(async (id: string, password?: string) => {
    await call('connect', { id, password });
    setCurrentConnectionId(id);
    setView('workspace');
  }, []);

  const handleQuickConnect = useCallback(async (host: string, port: number, db: number, password: string) => {
    await call('connect', { host, port, db, password });
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

  return (
    <ToastProvider>
      <div className="redis-client" ref={rootRef}>
        {(() => {
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
        })()}
      </div>
    </ToastProvider>
  );
}

export default App;
