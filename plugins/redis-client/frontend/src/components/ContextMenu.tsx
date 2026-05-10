import { useEffect, useRef } from 'react';

export interface ContextMenuAction {
  label: string;
  danger?: boolean;
  onClick: () => void;
}

interface Props {
  x: number;
  y: number;
  actions: ContextMenuAction[];
  onClose: () => void;
}

export function ContextMenu({ x, y, actions, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = () => onClose();
    document.addEventListener('click', handler);
    return () => document.removeEventListener('click', handler);
  }, [onClose]);

  // 调整位置防止溢出
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    if (rect.right > vw) el.style.left = `${x - rect.width}px`;
    if (rect.bottom > vh) el.style.top = `${y - rect.height}px`;
  }, [x, y]);

  return (
    <div ref={ref} className="context-menu" style={{ left: x, top: y }}>
      {actions.map((a, i) => (
        <div key={i} className={`context-menu-item ${a.danger ? 'context-menu-danger' : ''}`}
          onClick={e => { e.stopPropagation(); a.onClick(); onClose(); }}>
          {a.label}
        </div>
      ))}
    </div>
  );
}
