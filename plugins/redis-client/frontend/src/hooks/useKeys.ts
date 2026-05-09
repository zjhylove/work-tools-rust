import { useState, useCallback, useMemo } from 'react';
import { KeyInfo, TreeNode } from '../types';
import { buildTree } from '../utils/tree';

export function useKeys() {
  const [keys, setKeys] = useState<KeyInfo[]>([]);
  const [nextCursor, setNextCursor] = useState(0);
  const [scanLoading, setScanLoading] = useState(false);
  const [hasScanned, setHasScanned] = useState(false);
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());

  const tree = useMemo(() => buildTree(keys), [keys]);

  const togglePath = useCallback((path: string) => {
    setExpandedPaths(prev => {
      const next = new Set(prev);
      next.has(path) ? next.delete(path) : next.add(path);
      return next;
    });
  }, []);

  const scan = useCallback(async (pattern: string, append = false) => {
    setScanLoading(true);
    setHasScanned(false);
    const cursor = append ? nextCursor : 0;
    const r = await window.pluginAPI?.call('redis-client', 'scan_keys', { cursor, pattern, count: 200 });
    if (r && (r as Record<string, unknown>).keys) {
      const data = r as { keys: KeyInfo[]; cursor: number };
      setKeys(prev => append ? [...prev, ...data.keys] : data.keys);
      setNextCursor(data.cursor);
    }
    setHasScanned(true);
    setScanLoading(false);
  }, [nextCursor]);

  const deleteSelectedKeys = useCallback(async (selectedKeys: string[]) => {
    await window.pluginAPI?.call('redis-client', 'delete_keys', { keys: selectedKeys });
    setKeys(prev => prev.filter(k => !selectedKeys.includes(k.key)));
  }, []);

  return { keys, setKeys, tree, nextCursor, scanLoading, hasScanned, expandedPaths, togglePath, scan, deleteSelectedKeys };
}
