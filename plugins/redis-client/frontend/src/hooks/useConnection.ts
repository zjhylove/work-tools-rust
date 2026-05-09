import { useState, useCallback } from 'react';
import { ConnectionInfo } from '../types';
import { call } from '../api';

export function useConnection() {
  const [connected, setConnected] = useState(false);
  const [connectionInfo, setConnectionInfo] = useState<ConnectionInfo>({ connected: false });
  const [error, setError] = useState<string | null>(null);

  const connect = useCallback(async (id: string, password?: string) => {
    setError(null);
    try {
      const r = await call('connect', { id, password });
      if (r.ok) {
        const info = await call('get_connection_info');
        setConnected(true);
        setConnectionInfo(info as unknown as ConnectionInfo);
        return true;
      }
    } catch (e) {
      setError(String(e));
    }
    return false;
  }, []);

  const disconnect = useCallback(async () => {
    await call('disconnect');
    setConnected(false);
    setConnectionInfo({ connected: false });
  }, []);

  return { connected, connectionInfo, error, connect, disconnect };
}
