/* ═══════════════════════════════════════════════════════
   SVG Icons — 基于 Lucide 图标库
   轻量内联 SVG，零外部依赖
   ═══════════════════════════════════════════════════════ */

import React from "react";

interface IconProps {
  size?: number;
}

const svg = (path: React.ReactNode, size: number = 20) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.8"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    {path}
  </svg>
);

/* ── 导航 ─────────────────────────────── */
export const IconChevronRight: React.FC<IconProps> = ({ size }) =>
  svg(<path d="M9 18l6-6-6-6" />, size);
export const IconChevronDown: React.FC<IconProps> = ({ size }) =>
  svg(<path d="M6 9l6 6 6-6" />, size);
export const IconChevronUp: React.FC<IconProps> = ({ size }) =>
  svg(<path d="M18 15l-6-6-6 6" />, size);

/* ── 操作 ─────────────────────────────── */
export const IconPlus: React.FC<IconProps> = ({ size }) =>
  svg(<path d="M12 5v14M5 12h14" />, size);
export const IconSearch: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <circle cx="11" cy="11" r="8" />
      <path d="M21 21l-4.35-4.35" />
    </>,
    size,
  );
export const IconCopy: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
      <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" />
    </>,
    size,
  );
export const IconCheck: React.FC<IconProps> = ({ size }) =>
  svg(<path d="M20 6L9 17l-5-5" />, size);
export const IconX: React.FC<IconProps> = ({ size }) =>
  svg(<path d="M18 6L6 18M6 6l12 12" />, size);
export const IconEdit: React.FC<IconProps> = ({ size }) =>
  svg(
    <path d="M17 3a2.83 2.83 0 114 4L7.5 20.5 2 22l1.5-5.5L17 3z" />,
    size,
  );
export const IconTrash: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2" />
      <line x1="10" y1="11" x2="10" y2="17" />
      <line x1="14" y1="11" x2="14" y2="17" />
    </>,
    size,
  );
export const IconRefresh: React.FC<IconProps> = ({ size }) =>
  svg(
    <path d="M21 2v6h-6M3 12a9 9 0 0115.36-6.36L21 8M3 22v-6h6M21 12a9 9 0 01-15.36 6.36L3 16" />,
    size,
  );
export const IconDownload: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
      <polyline points="7 10 12 15 17 10" />
      <line x1="12" y1="15" x2="12" y2="3" />
    </>,
    size,
  );
export const IconUpload: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
      <polyline points="17 8 12 3 7 8" />
      <line x1="12" y1="3" x2="12" y2="15" />
    </>,
    size,
  );
export const IconPlay: React.FC<IconProps> = ({ size }) =>
  svg(<polygon points="5 3 19 12 5 21 5 3" />, size);
export const IconSettings: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-2 2 2 2 0 01-2-2v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 01-2-2 2 2 0 012-2h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 010-2.83 2 2 0 012.83 0l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 012-2 2 2 0 012 2v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 0 2 2 0 010 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 012 2 2 2 0 01-2 2h-.09a1.65 1.65 0 00-1.51 1z" />
    </>,
    size,
  );
export const IconExternalLink: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6" />
      <polyline points="15 3 21 3 21 9" />
      <line x1="10" y1="14" x2="21" y2="3" />
    </>,
    size,
  );

/* ── 文件与文件夹 ─────────────────────── */
export const IconFolder: React.FC<IconProps> = ({ size }) =>
  svg(
    <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2v11z" />,
    size,
  );
export const IconFolderOpen: React.FC<IconProps> = ({ size }) =>
  svg(
    <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2v2M2 11l1.62-2.43A2 2 0 015.33 7.5H20a2 2 0 011.94 2.49L20 19H4a2 2 0 01-2-2v-6z" />,
    size,
  );
export const IconFile: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
      <polyline points="14 2 14 8 20 8" />
    </>,
    size,
  );
export const IconFileText: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
      <polyline points="14 2 14 8 20 8" />
      <line x1="16" y1="13" x2="8" y2="13" />
      <line x1="16" y1="17" x2="8" y2="17" />
    </>,
    size,
  );

/* ── 功能 ──────────────────────────────── */
export const IconLock: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
      <path d="M7 11V7a5 5 0 0110 0v4" />
    </>,
    size,
  );
export const IconKey: React.FC<IconProps> = ({ size }) =>
  svg(
    <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 11-7.778 7.778 5.5 5.5 0 017.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />,
    size,
  );
export const IconTerminal: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <polyline points="4 17 10 11 4 5" />
      <line x1="12" y1="19" x2="20" y2="19" />
    </>,
    size,
  );
export const IconPackage: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <path d="M16.5 9.4l-9-5.19M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z" />
      <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
      <line x1="12" y1="22.08" x2="12" y2="12" />
    </>,
    size,
  );
export const IconCode: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <polyline points="16 18 22 12 16 6" />
      <polyline points="8 6 2 12 8 18" />
    </>,
    size,
  );
export const IconDatabase: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <ellipse cx="12" cy="5" rx="9" ry="3" />
      <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" />
      <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
    </>,
    size,
  );
export const IconGlobe: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <circle cx="12" cy="12" r="10" />
      <line x1="2" y1="12" x2="22" y2="12" />
      <path d="M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z" />
    </>,
    size,
  );
export const IconServer: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <rect x="2" y="2" width="20" height="8" rx="2" ry="2" />
      <rect x="2" y="14" width="20" height="8" rx="2" ry="2" />
      <line x1="6" y1="6" x2="6.01" y2="6" />
      <line x1="6" y1="18" x2="6.01" y2="18" />
    </>,
    size,
  );
export const IconCloud: React.FC<IconProps> = ({ size }) =>
  svg(
    <path d="M17.5 19H9a7 7 0 116.71-9h1.79a4.5 4.5 0 110 9z" />,
    size,
  );

/* ── 主题 ──────────────────────────────── */
export const IconSun: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <circle cx="12" cy="12" r="5" />
      <line x1="12" y1="1" x2="12" y2="3" />
      <line x1="12" y1="21" x2="12" y2="23" />
      <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
      <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
      <line x1="1" y1="12" x2="3" y2="12" />
      <line x1="21" y1="12" x2="23" y2="12" />
      <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
      <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
    </>,
    size,
  );
export const IconMoon: React.FC<IconProps> = ({ size }) =>
  svg(<path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z" />, size);

/* ── 状态 ─────────────────────────────── */
export const IconAlertCircle: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <circle cx="12" cy="12" r="10" />
      <line x1="12" y1="8" x2="12" y2="12" />
      <line x1="12" y1="16" x2="12.01" y2="16" />
    </>,
    size,
  );
export const IconInfo: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <circle cx="12" cy="12" r="10" />
      <line x1="12" y1="16" x2="12" y2="12" />
      <line x1="12" y1="8" x2="12.01" y2="8" />
    </>,
    size,
  );
export const IconEye: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
      <circle cx="12" cy="12" r="3" />
    </>,
    size,
  );
export const IconEyeOff: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19m-6.72-1.07a3 3 0 11-4.24-4.24" />
      <line x1="1" y1="1" x2="23" y2="23" />
    </>,
    size,
  );
export const IconArrowUp: React.FC<IconProps> = ({ size }) =>
  svg(<path d="M12 19V5M5 12l7-7 7 7" />, size);

/* ── 用户界面 ─────────────────────────── */
export const IconMenu: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <line x1="3" y1="12" x2="21" y2="12" />
      <line x1="3" y1="6" x2="21" y2="6" />
      <line x1="3" y1="18" x2="21" y2="18" />
    </>,
    size,
  );
export const IconMoreVertical: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <circle cx="12" cy="12" r="1" />
      <circle cx="12" cy="5" r="1" />
      <circle cx="12" cy="19" r="1" />
    </>,
    size,
  );
export const IconMoreHorizontal: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <circle cx="12" cy="12" r="1" />
      <circle cx="5" cy="12" r="1" />
      <circle cx="19" cy="12" r="1" />
    </>,
    size,
  );
export const IconFilter: React.FC<IconProps> = ({ size }) =>
  svg(<polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3" />, size);
export const IconLightbulb: React.FC<IconProps> = ({ size }) =>
  svg(
    <>
      <path d="M9 18h6M10 22h4M15.09 14c.18-.98.65-1.74 1.41-2.5A4.65 4.65 0 0018 8 6 6 0 006 8c0 1 .23 2.23 1.5 3.5A4.61 4.61 0 018.91 14" />
    </>,
    size,
  );
