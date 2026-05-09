import { useState, useEffect } from 'react';

interface Props { selectedKey: string | null; }

export function HexViewer({ selectedKey }: Props) {
  const [hex, setHex] = useState('');
  const [length, setLength] = useState(0);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!selectedKey) return;
    (async () => {
      setLoading(true);
      const r = await window.pluginAPI?.call('redis-client', 'hex_dump', { key: selectedKey, max_bytes: 1024 });
      const data = r as { hex: string; length: number };
      setHex(data.hex);
      setLength(data.length);
      setLoading(false);
    })();
  }, [selectedKey]);

  if (loading) return <div className="detail-loading"><span className="spinner" />加载中…</div>;
  if (!selectedKey) return <div className="detail-loading">选择 Key 查看 HEX</div>;

  const lines: string[] = [];
  for (let i = 0; i < hex.length; i += 32) {
    const hexPart = hex.slice(i, i + 32).match(/.{1,2}/g)?.join(' ') || '';
    const bytePart = hex.slice(i, i + 32).match(/.{1,2}/g)?.map(b => {
      const c = parseInt(b, 16);
      return c >= 32 && c <= 126 ? String.fromCharCode(c) : '.';
    }).join('') || '';
    lines.push(`${(i / 2).toString(16).padStart(4, '0')}| ${hexPart.padEnd(47)}|${bytePart}|`);
  }

  return (
    <div className="hex-viewer">
      <div className="hex-length">共 {length} 字节</div>
      <pre className="hex-dump">{lines.join('\n')}</pre>
    </div>
  );
}
