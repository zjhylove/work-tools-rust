import { useState, useCallback } from 'react';
import { KeyInfo } from '../types';

export function useKeyDetail() {
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [openTabs, setOpenTabs] = useState<KeyInfo[]>([]);
  const [keyDetail, setKeyDetail] = useState<Record<string, unknown> | null>(null);
  const [valueData, setValueData] = useState<unknown>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  const viewerMethods: Record<string, string> = {
    string: 'get_string',
    hash: 'get_hash',
    list: 'get_list',
    set: 'get_set',
    zset: 'get_zset',
  };

  const selectKey = useCallback(async (key: string) => {
    setSelectedKey(key);
    setDetailLoading(true);
    setValueData(null);

    try {
      const info = await window.pluginAPI?.call('redis-client', 'get_key_info', { key });
      setKeyDetail(info as Record<string, unknown>);

      const kType = (info as Record<string, string>).type;
      const method = viewerMethods[kType];
      if (method) {
        const v = await window.pluginAPI?.call('redis-client', method, { key });
        setValueData(v);
      }

      setOpenTabs(prev => {
        const exists = prev.find(t => t.key === key);
        if (exists) return prev;
        const keyInfo: KeyInfo = { key, type: kType || 'unknown', ttl: (info as Record<string, number>).ttl || 0 };
        return [...prev, keyInfo];
      });
    } catch { /* handled in component */ }

    setDetailLoading(false);
  }, []);

  const closeTab = useCallback((key: string) => {
    setOpenTabs(prev => prev.filter(t => t.key !== key));
    if (selectedKey === key) {
      setSelectedKey(null);
      setKeyDetail(null);
      setValueData(null);
    }
  }, [selectedKey]);

  const refresh = useCallback(() => {
    if (selectedKey) selectKey(selectedKey);
  }, [selectedKey, selectKey]);

  return { selectedKey, setSelectedKey, openTabs, closeTab, keyDetail, valueData, detailLoading, selectKey, refresh };
}
