import { useState, useCallback } from 'react';
import { ConnectionInfo } from '../types';

declare global {
  interface Window {
    pluginAPI?: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

export function useConnection() {
  const [connected, setConnected] = useState(false);
  const [connectionInfo, setConnectionInfo] = useState<ConnectionInfo>({ connected: false });
  const [error, setError] = useState<string | null>(null);

  const connect = useCallback(async (id: string, password?: string) => {
    setError(null);
    const r = await window.pluginAPI?.call('redis-client', 'connect', { id, password });
    if (r && (r as Record<string, unknown>).ok) {
      const info = r as ConnectionInfo;
      setConnected(true);
      setConnectionInfo(info);
      return true;
    }
    return false;
  }, []);

  const disconnect = useCallback(async () => {
    await window.pluginAPI?.call('redis-client', 'disconnect', {});
    setConnected(false);
    setConnectionInfo({ connected: false });
  }, []);

  const clearError = useCallback(() => setError(null), []);

  return { connected, connectionInfo, error, setError, connect, disconnect, clearError };
}
