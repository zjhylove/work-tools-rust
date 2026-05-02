# Theme Toggle — Light/Dark

**Date**: 2026-05-02
**Status**: approved

## Background

`tokens.css` already defines complete dark theme tokens under `[data-theme="dark"]`. No theme switching mechanism exists. Plugin iframe `INJECTED_TOKENS` is light-only with hardcoded color values.

## Goal

Add a one-click theme toggle (light/dark) in sidebar footer. Persist preference via localStorage. Sync theme to plugin iframes.

## Design

### Architecture

```
localStorage("theme") → App init → set <html data-theme="light|dark">
                                    ↓
                        user clicks toggle → update state + localStorage + data-theme
                                    ↓
                        postMessage → all plugin iframes sync data-theme on <html>
```

### Changes

#### 1. `tauri-app/src/components/icons.tsx` — New icons
- Add `IconMoon` and `IconSun` SVG components (20x20, matching existing icon style)

#### 2. `tauri-app/src/App.tsx` — Theme state + toggle button
- `useState<"light"|"dark">` init from `localStorage.getItem("theme")`, default `"light"`
- `useEffect` sets `document.documentElement.dataset.theme` on change
- Toggle function: flip state, write localStorage, iterate `iframe` elements calling `postMessage({type:"theme",theme})`
- New button in sidebar footer, between existing two buttons
- Button shows `IconMoon` when light, `IconSun` when dark
- Button uses existing `.sidebar-footer-btn` class

#### 3. `tauri-app/src/components/PluginPlaceholder.tsx` — Token injection + theme sync
- Accept new `theme` prop
- `INJECTED_TOKENS`: append `[data-theme="dark"]` block with dark token values (copy from tokens.css)
- `body` styles: replace hardcoded `color: #1b1c1d; background: #ffffff` with `color: var(--text-primary); background: var(--bg-primary)`
- Set `data-theme` on `<html>` in srcdoc matching current theme
- Register `window.addEventListener("message")` in iframe to handle `{type:"theme", theme}` and update `document.documentElement.dataset.theme`

### Non-changes
- No backend changes
- No new dependencies
- `tokens.css` unchanged (already complete)
- Plugin source CSS unchanged (already uses `var(--xxx)` tokens)

### Persistence
- localStorage key: `"theme"`, values: `"light"` | `"dark"`
- Fallback: `"light"` if unset or invalid
