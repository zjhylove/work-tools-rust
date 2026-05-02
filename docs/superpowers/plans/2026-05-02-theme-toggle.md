# Theme Toggle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add light/dark theme toggle button in sidebar footer, persisted via localStorage, synced to plugin iframes.

**Architecture:** App reads theme from localStorage → sets `<html data-theme>` → CSS variables cascade everywhere. User clicks toggle → flips state + updates localStorage + postMessage to iframes. PluginPlaceholder injects both `:root` (light) and `[data-theme="dark"]` CSS blocks + a sync `<script>` into iframe srcdoc.

**Tech Stack:** React 19, TypeScript, CSS custom properties, localStorage, postMessage

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `tauri-app/src/components/PluginPlaceholder.tsx` | Modify | Inject dark tokens, theme-aware body, theme sync script, accept `theme` prop |
| `tauri-app/src/App.tsx` | Modify | Theme state, toggle handler, sidebar button, postMessage broadcast |

---

### Task 1: Update PluginPlaceholder — dark tokens + theme prop + sync script

**Files:**
- Modify: `tauri-app/src/components/PluginPlaceholder.tsx`

- [ ] **Step 1: Add `theme` prop to the component interface**

At line 4-7, update the interface:

```typescript
interface PluginPlaceholderProps {
  pluginId: string;
  setSelectedPlugin: (pluginId: string | null) => void;
  theme: "light" | "dark";
}
```

- [ ] **Step 2: Replace INJECTED_TOKENS — add dark block + fix body**

Replace the entire `INJECTED_TOKENS` constant (lines 9-121) with this version that appends `[data-theme="dark"]` block and uses CSS variables for body:

```typescript
const INJECTED_TOKENS = `
  :root {
    --accent: #0066ff;
    --accent-hover: #0052cc;
    --accent-light: #eef3ff;
    --accent-ring: rgba(0, 102, 255, 0.15);
    --success: #10b981;
    --success-light: #ecfdf5;
    --success-border: #a7f3d0;
    --success-text: #059669;
    --warning: #f59e0b;
    --warning-light: #fffbeb;
    --warning-border: #fde68a;
    --warning-text: #b45309;
    --error: #ef4444;
    --error-light: #fef2f2;
    --error-border: #fecaca;
    --error-text: #b91c1c;
    --bg-primary: #ffffff;
    --bg-secondary: #f8f9fa;
    --bg-tertiary: #f1f3f5;
    --hover-bg: rgba(0, 0, 0, 0.04);
    --text-primary: #1b1c1d;
    --text-secondary: #6b7280;
    --text-tertiary: #9ca3af;
    --text-inverse: #ffffff;
    --border-color: #e5e7eb;
    --border-light: #f1f3f5;
    --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.03);
    --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.05), 0 1px 2px rgba(0, 0, 0, 0.04);
    --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.06), 0 2px 4px rgba(0, 0, 0, 0.04);
    --shadow-lg: 0 12px 32px rgba(0, 0, 0, 0.08), 0 4px 8px rgba(0, 0, 0, 0.04);
    --radius-xs: 4px;
    --radius-sm: 6px;
    --radius-md: 8px;
    --radius-lg: 12px;
    --radius-xl: 16px;
    --radius-2xl: 20px;
    --space-xs: 4px;
    --space-sm: 8px;
    --space-md: 12px;
    --space-lg: 16px;
    --space-xl: 24px;
    --space-2xl: 32px;
    --font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans SC", "PingFang SC", "Microsoft YaHei", sans-serif;
    --font-mono: "SF Mono", "Cascadia Code", "Fira Code", "JetBrains Mono", Consolas, monospace;
    --font-size-xs: 11px;
    --font-size-sm: 12px;
    --font-size-base: 13px;
    --font-size-md: 14px;
    --font-size-lg: 16px;
    --font-size-xl: 18px;
    --transition-fast: 0.12s ease;
    --transition-base: 0.2s ease;
    --transition-slow: 0.3s ease;
  }
  [data-theme="dark"] {
    --accent: #3b82f6;
    --accent-hover: #60a5fa;
    --accent-light: #1e3a5f;
    --accent-ring: rgba(59, 130, 246, 0.25);
    --success: #34d399;
    --success-light: #064e3b;
    --success-border: #065f46;
    --success-text: #6ee7b7;
    --warning: #fbbf24;
    --warning-light: #78350f;
    --warning-border: #92400e;
    --warning-text: #fcd34d;
    --error: #f87171;
    --error-light: #7f1d1d;
    --error-border: #991b1b;
    --error-text: #fca5a5;
    --bg-primary: #1a1b1e;
    --bg-secondary: #25262b;
    --bg-tertiary: #2c2e33;
    --hover-bg: rgba(255, 255, 255, 0.05);
    --text-primary: #e5e7eb;
    --text-secondary: #9ca3af;
    --text-tertiary: #6b7280;
    --text-inverse: #1a1b1e;
    --border-color: #373a40;
    --border-light: #2c2e33;
    --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.2);
    --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3);
    --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.4);
    --shadow-lg: 0 12px 32px rgba(0, 0, 0, 0.5);
  }
  html, body, #app {
    height: 100%;
  }
  * {
    box-sizing: border-box;
  }
  body {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans SC", "PingFang SC", "Microsoft YaHei", sans-serif;
    color: var(--text-primary);
    background: var(--bg-primary);
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }
  ::-webkit-scrollbar { width: 5px; height: 5px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: var(--border-color); border-radius: 3px; }
  ::-webkit-scrollbar-thumb:hover { background: var(--text-tertiary); }

  @keyframes slideDown {
    from { opacity: 0; transform: translateX(-50%) translateY(-20px); }
    to { opacity: 1; transform: translateX(-50%) translateY(0); }
  }

  .error-message {
    position: fixed;
    top: 20px;
    left: 50%;
    transform: translateX(-50%);
    padding: 10px 16px;
    background: var(--error-light);
    border: 1px solid var(--error-border);
    border-radius: 8px;
    color: var(--error);
    font-size: 13px;
    font-weight: 500;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    z-index: 1000;
    animation: slideDown 0.3s ease;
  }
  .error-message.success {
    background: var(--success-light);
    border-color: var(--success-border);
    color: var(--success-text);
  }
  .error-message.warning {
    background: var(--warning-light);
    border-color: var(--warning-border);
    color: var(--warning-text);
  }
  .error-message.info {
    background: var(--accent-light);
    border-color: var(--accent);
    color: var(--accent);
  }
`;
```

- [ ] **Step 3: Inject theme sync script into srcdoc**

Replace the srcdoc assembly block (lines 181-190) to include the theme sync script and set initial data-theme:

```typescript
        const themeScript = `<script>document.documentElement.dataset.theme="${theme}";window.addEventListener("message",function(e){if(e.data&&e.data.type==="theme"){document.documentElement.dataset.theme=e.data.theme}});</script>`;
        const fullHtml =
          parts[0] +
          `<style>${INJECTED_TOKENS}${styles}</style>${themeScript}<script type="module">${mainJs}</script>` +
          parts
            .slice(1)
            .join(
              '<script type="module" crossorigin src="./main.js"></script>',
            )
            .split('<link rel="stylesheet" crossorigin href="./styles.css">')
            .join("");
```

- [ ] **Step 4: Destructure `theme` from props**

At line 123-126, update the destructuring:

```typescript
export default function PluginPlaceholder({
  pluginId,
  setSelectedPlugin,
  theme,
}: PluginPlaceholderProps) {
```

- [ ] **Step 5: Commit**

```bash
git add tauri-app/src/components/PluginPlaceholder.tsx
git commit -m "feat: inject dark theme tokens and theme sync script into plugin iframes

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 2: Add theme state, toggle, and sidebar button in App.tsx

**Files:**
- Modify: `tauri-app/src/App.tsx`

- [ ] **Step 1: Import theme icons**

At line 14-16, add `IconSun` and `IconMoon` to the import:

```typescript
import {
  IconTerminal,
  IconPackage,
  IconX,
  IconCode,
  IconSun,
  IconMoon,
} from "./components/icons";
```

- [ ] **Step 2: Add theme state and initialization**

After the `showPluginMarket` state (line 39), add theme state:

```typescript
  const [theme, setTheme] = useState<"light" | "dark">(() => {
    const stored = localStorage.getItem("theme");
    return stored === "dark" ? "dark" : "light";
  });
```

- [ ] **Step 3: Add useEffect for data-theme attribute**

After the existing `useEffect` block (line 97-108), add:

```typescript
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);
```

- [ ] **Step 4: Add toggle handler**

After the `openPlugin` function (lines 112-125), add:

```typescript
  const toggleTheme = () => {
    setTheme((prev) => {
      const next = prev === "light" ? "dark" : "light";
      localStorage.setItem("theme", next);
      document.querySelectorAll("iframe").forEach((iframe) => {
        iframe.contentWindow?.postMessage({ type: "theme", theme: next }, "*");
      });
      return next;
    });
  };
```

- [ ] **Step 5: Add theme toggle button in sidebar footer**

Replace the sidebar footer section (lines 160-175) to include the theme button between existing buttons:

```typescript
        <div className="sidebar-footer">
          <button
            className="sidebar-footer-btn"
            title="系统日志"
            onClick={() => setShowLogs(true)}
          >
            <IconTerminal size={20} />
          </button>
          <button
            className="sidebar-footer-btn"
            title={theme === "light" ? "切换暗色主题" : "切换亮色主题"}
            onClick={toggleTheme}
          >
            {theme === "light" ? <IconMoon size={20} /> : <IconSun size={20} />}
          </button>
          <button
            className="sidebar-footer-btn"
            title="插件市场"
            onClick={() => setShowPluginMarket(true)}
          >
            <IconPackage size={20} />
          </button>
        </div>
```

- [ ] **Step 6: Pass `theme` prop to PluginPlaceholder**

In the `visitedPlugins.map` section (line 192-201), add the `theme` prop:

```typescript
                <ErrorBoundary>
                  <PluginPlaceholder
                    pluginId={pluginId}
                    setSelectedPlugin={setSelectedPlugin}
                    theme={theme}
                  />
                </ErrorBoundary>
```

- [ ] **Step 7: Commit**

```bash
git add tauri-app/src/App.tsx
git commit -m "feat: add theme toggle button in sidebar footer

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 3: Verify

- [ ] **Step 1: TypeScript check**

```bash
cd tauri-app && npx tsc --noEmit 2>&1 | head -30
```
Expected: no errors.

- [ ] **Step 2: Run cargo check**

```bash
cargo check 2>&1 | tail -5
```
Expected: no errors.

- [ ] **Step 3: Manual verification checklist**

Run `cd tauri-app && npm run tauri dev`, then:
1. App starts with light theme (default)
2. Sidebar footer shows moon icon in middle position
3. Click moon → switches to dark theme, icon changes to sun
4. Click sun → switches back to light theme, icon changes to moon
5. Refresh app → theme persists from localStorage
6. Open a plugin → plugin iframe matches parent theme
7. Toggle theme while plugin is open → plugin iframe updates
