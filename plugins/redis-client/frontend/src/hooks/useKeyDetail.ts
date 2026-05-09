import { useState, useCallback } from 'react';
import { KeyInfo } from '../types';
import { call } from '../api';

const VIEWER_METHODS: Record<string, string> = {
  string: 'get_string',
  hash: 'get_hash',
  list: 'get_list',
  set: 'get_set',
  zset: 'get_zset',
};

export function useKeyDetail() {
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [keyDetail, setKeyDetail] = useState<Record<string, unknown> | null>(null);
  const [valueData, setValueData] = useState<unknown>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  const selectKey = useCallback(async (key: string) => {
    setSelectedKey(key);
    setDetailLoading(true);
    setValueData(null);

    try {
      const info = await call('get_key_info', { key });
      setKeyDetail(info);

      const kType = info.type as string;
      const method = VIEWER_METHODS[kType];
      if (method) {
        const v = await call(method, { key });
        setValueData(v);
      }
    } catch { /* handled in component */ }

    setDetailLoading(false);
  }, []);

  const refresh = useCallback(() => {
    if (selectedKey) selectKey(selectedKey);
  }, [selectedKey, selectKey]);

  return { selectedKey, keyDetail, valueData, detailLoading, selectKey, refresh };
}
