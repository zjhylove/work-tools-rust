# Plugin Frontend UI Unification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unify toast/notification, form validation, button, modal, and common CSS patterns across all 8 plugin frontends via injected CSS tokens + JS utility.

**Architecture:** Extend `PluginPlaceholder.tsx` to inject `WorkTools` JS utility (toast + field error helpers) alongside existing `INJECTED_TOKENS` CSS. Each plugin replaces its ad-hoc toast/form implementation with `WorkTools.toast.*()` / `WorkTools.FieldError.*()` calls and `wt-*` CSS classes.

**Tech Stack:** React 19 + TypeScript, no new dependencies.

---

## File Structure

| File | Action |
|------|--------|
| `tauri-app/src/components/PluginPlaceholder.tsx` | Modify — extend INJECTED_TOKENS, add WorkTools script injection |
| `plugins/password-manager/frontend/src/App.tsx` | Modify — toast + form + loading adapt |
| `plugins/password-manager/frontend/src/App.css` | Modify — class rename + hardcoded fix |
| `plugins/json-tools/frontend/src/App.tsx` | Modify — toast adapt |
| `plugins/json-tools/frontend/src/App.css` | Modify — syntax tokens + message remove |
| `plugins/auth-plugin/frontend/src/App.tsx` | Modify — toast + form adapt |
| `plugins/auth-plugin/frontend/src/App.css` | Modify — hardcoded fix |
| `plugins/text-diff/frontend/src/App.tsx` | Modify — error banner → toast |
| `plugins/text-diff/frontend/src/App.css` | Modify — button rename |
| `plugins/db-doc/frontend/src/App.tsx` | Modify — validation + loading + toast rename |
| `plugins/db-doc/frontend/src/App.css` | Modify — toast class rename |
| `plugins/k8s-forward/frontend/src/App.tsx` | Modify — toast adapt |
| `plugins/k8s-forward/frontend/src/components/TabSshForward.tsx` | Modify — toast adapt |
| `plugins/k8s-forward/frontend/src/components/TabK8sForward.tsx` | Modify — toast adapt |
| `plugins/k8s-forward/frontend/src/App.css` | Modify — hardcoded fix + class rename |
| `plugins/db-router/frontend/src/App.tsx` | Modify — toast + validation adapt |
| `plugins/db-router/frontend/src/App.css` | Modify — toast class rename + modal fix |
| `plugins/object-storage/frontend/src/App.tsx` | Modify — toast + validation + delete confirm |
| `plugins/object-storage/frontend/src/App.css` | Modify — hardcoded fix + modal classes |
| `CLAUDE.md` | Modify — add plugin frontend development section |

---

### Task 1: Extend PluginPlaceholder — Inject WorkTools CSS + JS

**Files:**
- Modify: `tauri-app/src/components/PluginPlaceholder.tsx:10-154`

- [ ] **Step 1: Replace INJECTED_TOKENS with expanded unified CSS**

Replace the entire `INJECTED_TOKENS` constant (lines 10-154) with the expanded version that includes all `wt-*` classes. The existing `.error-message` classes are kept for backward compatibility during migration but deprecated.

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
    --wt-modal-overlay-bg: rgba(0, 0, 0, 0.5);
    /* syntax highlight tokens */
    --wt-syntax-key: #d32f2f;
    --wt-syntax-string: #2e7d32;
    --wt-syntax-number: #1565c0;
    --wt-syntax-boolean: #c62828;
    --wt-syntax-null: #7f8c8d;
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
    --wt-modal-overlay-bg: rgba(0, 0, 0, 0.65);
    --wt-syntax-key: #ff6b6b;
    --wt-syntax-string: #69db7c;
    --wt-syntax-number: #74b9ff;
    --wt-syntax-boolean: #ff6b6b;
    --wt-syntax-null: #adb5bd;
  }
  html, body, #app { height: 100%; }
  * { box-sizing: border-box; }
  body {
    margin: 0; padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans SC", "PingFang SC", "Microsoft YaHei", sans-serif;
    color: var(--text-primary); background: var(--bg-primary);
    -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale;
  }
  ::-webkit-scrollbar { width: 5px; height: 5px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: var(--border-color); border-radius: 3px; }
  ::-webkit-scrollbar-thumb:hover { background: var(--text-tertiary); }

  /* ── Toast ─────────────────────────── */
  @keyframes wt-toast-slide-in {
    from { opacity: 0; transform: translateX(100%); }
    to { opacity: 1; transform: translateX(0); }
  }
  .wt-toast-container {
    position: fixed; top: 16px; right: 16px; z-index: 9999;
    display: flex; flex-direction: column; gap: 8px;
    pointer-events: none;
  }
  .wt-toast {
    pointer-events: auto;
    padding: 10px 16px; border-radius: var(--radius-md);
    font-size: var(--font-size-base); font-weight: 500;
    box-shadow: var(--shadow-md); cursor: pointer;
    animation: wt-toast-slide-in 0.25s var(--ease-out, ease-out);
    max-width: 360px; word-break: break-word;
    display: flex; align-items: center; gap: 8px;
  }
  .wt-toast--success { background: var(--success-light); border: 1px solid var(--success-border); color: var(--success-text); }
  .wt-toast--error   { background: var(--error-light); border: 1px solid var(--error-border); color: var(--error-text); }
  .wt-toast--info    { background: var(--accent-light); border: 1px solid var(--accent-ring); color: var(--accent); }
  .wt-toast--warning { background: var(--warning-light); border: 1px solid var(--warning-border); color: var(--warning-text); }

  /* ── Buttons ───────────────────────── */
  .wt-btn {
    display: inline-flex; align-items: center; justify-content: center; gap: 6px;
    padding: 7px 14px; border-radius: var(--radius-sm); border: 1px solid transparent;
    font-size: var(--font-size-base); font-family: var(--font-sans); font-weight: 500;
    cursor: pointer; transition: background var(--transition-fast), box-shadow var(--transition-fast);
    outline: none; line-height: 1.4;
  }
  .wt-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .wt-btn--primary   { background: var(--accent); color: var(--text-inverse); }
  .wt-btn--primary:hover:not(:disabled) { background: var(--accent-hover); }
  .wt-btn--secondary { background: var(--bg-secondary); color: var(--text-primary); border-color: var(--border-color); }
  .wt-btn--secondary:hover:not(:disabled) { background: var(--bg-tertiary); }
  .wt-btn--danger    { background: var(--error); color: var(--text-inverse); }
  .wt-btn--danger:hover:not(:disabled) { background: var(--error-text); }
  .wt-btn--ghost     { background: transparent; color: var(--text-secondary); }
  .wt-btn--ghost:hover:not(:disabled) { background: var(--hover-bg); color: var(--text-primary); }
  .wt-btn--sm        { padding: 4px 10px; font-size: var(--font-size-sm); }

  /* ── Forms ─────────────────────────── */
  .wt-form-group { margin-bottom: var(--space-md); }
  .wt-form-label { display: block; font-size: var(--font-size-sm); font-weight: 500; color: var(--text-secondary); margin-bottom: 4px; }
  .wt-form-input {
    width: 100%; padding: 7px 10px; border-radius: var(--radius-sm);
    border: 1px solid var(--border-color); background: var(--bg-primary);
    color: var(--text-primary); font-size: var(--font-size-base); font-family: var(--font-sans);
    transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
    outline: none;
  }
  .wt-form-input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-ring); }
  .wt-form-input--error { border-color: var(--error); }
  .wt-form-input--error:focus { box-shadow: 0 0 0 3px var(--error-light); }
  .wt-field-error { font-size: var(--font-size-xs); color: var(--error-text); margin-top: 4px; }

  /* ── Modal ─────────────────────────── */
  .wt-modal-overlay {
    position: fixed; inset: 0; z-index: 10000;
    background: var(--wt-modal-overlay-bg);
    display: flex; align-items: center; justify-content: center;
  }
  .wt-modal {
    background: var(--bg-primary); border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg); max-width: 440px; width: 90%;
    max-height: 80vh; display: flex; flex-direction: column;
  }
  .wt-modal-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: var(--space-lg) var(--space-lg) var(--space-sm);
  }
  .wt-modal-header h3 { margin: 0; font-size: var(--font-size-lg); }
  .wt-modal-body { padding: var(--space-sm) var(--space-lg) var(--space-lg); overflow-y: auto; color: var(--text-secondary); font-size: var(--font-size-base); }
  .wt-modal-footer { display: flex; justify-content: flex-end; gap: var(--space-sm); padding: 0 var(--space-lg) var(--space-lg); }

  /* ── Misc ──────────────────────────── */
  @keyframes wt-spin { to { transform: rotate(360deg); } }
  .wt-spinner {
    width: 16px; height: 16px; border: 2px solid var(--border-color);
    border-top-color: var(--accent); border-radius: 50%;
    animation: wt-spin 0.6s linear infinite; display: inline-block;
  }
  .wt-empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: var(--space-2xl); color: var(--text-tertiary); gap: var(--space-sm); font-size: var(--font-size-base); }
  .wt-badge { display: inline-block; padding: 2px 8px; border-radius: var(--radius-xs); font-size: var(--font-size-xs); font-weight: 600; }
  .wt-status-dot { width: 8px; height: 8px; border-radius: 50%; display: inline-block; }
  .wt-status-dot--online { background: var(--success); }
  .wt-status-dot--offline { background: var(--text-tertiary); }

  /* deprecated: kept for plugins not yet migrated */
  @keyframes slideDown { from { opacity: 0; transform: translateX(-50%) translateY(-20px); } to { opacity: 1; transform: translateX(-50%) translateY(0); } }
  .error-message { position: fixed; top: 20px; left: 50%; transform: translateX(-50%); padding: 10px 16px; background: var(--error-light); border: 1px solid var(--error-border); border-radius: 8px; color: var(--error); font-size: 13px; font-weight: 500; box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1); z-index: 1000; animation: slideDown 0.3s ease; }
  .error-message.success { background: var(--success-light); border-color: var(--success-border); color: var(--success-text); }
  .error-message.warning { background: var(--warning-light); border-color: var(--warning-border); color: var(--warning-text); }
  .error-message.info { background: var(--accent-light); border-color: var(--accent); color: var(--accent); }
`;
```

- [ ] **Step 2: Add WorkTools script injection after the plugins-ready handler**

In the `fullHtml` construction (line 217), add `INJECTED_SCRIPTS` right after `INJECTED_TOKENS`. Add this constant above `PluginPlaceholder`:

```typescript
const INJECTED_SCRIPTS = `
<script>
(function() {
  var toastTimer = null;
  var container = null;

  function getContainer() {
    if (!container || !document.body.contains(container)) {
      container = document.createElement('div');
      container.className = 'wt-toast-container';
      document.body.appendChild(container);
    }
    return container;
  }

  function showToast(type, message) {
    var el = document.createElement('div');
    el.className = 'wt-toast wt-toast--' + type;
    var icons = { success: '✓ ', error: '✗ ', info: 'ℹ ', warning: '⚠ ' };
    el.textContent = (icons[type] || '') + message;
    el.addEventListener('click', function() { el.remove(); });
    getContainer().appendChild(el);
    setTimeout(function() {
      if (el.parentNode) el.remove();
      var c = getContainer();
      if (c.children.length === 0 && container) { container.remove(); container = null; }
    }, 3000);
  }

  window.WorkTools = {
    toast: {
      success: function(m) { showToast('success', m); },
      error: function(m)   { showToast('error', m); },
      info: function(m)    { showToast('info', m); },
      warning: function(m) { showToast('warning', m); }
    },
    FieldError: {
      show: function(inputEl, message) {
        this.clear(inputEl);
        inputEl.classList.add('wt-form-input--error');
        var err = document.createElement('div');
        err.className = 'wt-field-error';
        err.textContent = message;
        inputEl.parentNode.appendChild(err);
      },
      clear: function(inputEl) {
        inputEl.classList.remove('wt-form-input--error');
        var next = inputEl.nextElementSibling;
        if (next && next.classList.contains('wt-field-error')) next.remove();
      },
      clearAll: function(formEl) {
        formEl.querySelectorAll('.wt-form-input--error').forEach(function(el) {
          el.classList.remove('wt-form-input--error');
        });
        formEl.querySelectorAll('.wt-field-error').forEach(function(el) {
          el.remove();
        });
      }
    }
  };
})();
</script>`;
```

- [ ] **Step 3: Wire INJECTED_SCRIPTS into fullHtml**

Change line 218 from:
```typescript
`<style>${styles}${INJECTED_TOKENS}</style>${themeScript}<script type="module">${mainJs}</script>`
```
to:
```typescript
`<style>${styles}${INJECTED_TOKENS}</style>${themeScript}${INJECTED_SCRIPTS}<script type="module">${mainJs}</script>`
```

- [ ] **Step 4: Run TypeScript check**

```bash
cd tauri-app && npx tsc --noEmit
```
Expected: no new errors (INJECTED_TOKENS and INJECTED_SCRIPTS are string constants, no TS impact).

- [ ] **Step 5: Commit**

```bash
git add tauri-app/src/components/PluginPlaceholder.tsx
git commit -m "feat: inject WorkTools toast/form utility into plugin iframes

Add wt-toast, wt-btn, wt-form, wt-modal CSS classes to INJECTED_TOKENS.
Add WorkTools JS utility (toast + FieldError) injected into each iframe.
Deprecate old .error-message classes, kept for migration period.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 2: Adapt Password Manager

**Files:**
- Modify: `plugins/password-manager/frontend/src/App.tsx`
- Modify: `plugins/password-manager/frontend/src/App.css`

- [ ] **Step 1: Replace toast in App.tsx**

Replace all `setError(...)` calls with `WorkTools.toast.*()`:

```typescript
// Remove: const [error, setError] = useState("");
// Remove the entire Toast block (lines 489-501)

// Replace setError calls:
// Line 114: setError("加载密码列表失败") →
//   WorkTools.toast.error("加载密码列表失败")

// Line 214-215: setError("✓ 密码已复制") + setTimeout →
//   WorkTools.toast.success("密码已复制")

// Line 218: setError("复制失败,请手动复制") →
//   WorkTools.toast.error("复制失败,请手动复制")

// Line 304: setError("请填写所有必填字段") →
//   WorkTools.toast.warning("请填写所有必填字段")

// Line 349: setError("保存密码失败") →
//   WorkTools.toast.error("保存密码失败")

// Line 377-378: setError(`✅ 密码已导出到...`) + setTimeout →
//   WorkTools.toast.success(`密码已导出到 ${filePath}`)

// Line 380-381: setError("❌ 导出失败: ...") + setTimeout →
//   WorkTools.toast.error("导出失败: " + (err as Error).message)

// Import lines (415, 421, 435-436, 444, 450, 458, 461, 474):
//   setError("⏳ ...") → WorkTools.toast.info(...)
//   setError("❌ ...") → WorkTools.toast.error(...)
//   setError("⚠️ ...") → WorkTools.toast.warning(...)
//   setError("✅ ...") → WorkTools.toast.success(...)
```

- [ ] **Step 2: Add loading spinner to submit button**

Replace the submit button JSX (lines 707-718) with:
```tsx
{field.type === "button" && (
  <button
    className={`wt-btn wt-btn--primary${!isFormValid() ? " disabled" : ""}`}
    disabled={!isFormValid()}
    onClick={(e) => {
      e.preventDefault();
      e.stopPropagation();
      handleAction(field.key);
    }}
  >
    {isEditMode ? "更新密码" : field.label}
  </button>
)}
```

- [ ] **Step 3: Align form CSS classes**

In `handleFieldChange` error display, change:
- `field-input-error` → `wt-form-input--error`
- `field-error` → `wt-field-error`

Update the input className (line 693):
```tsx
className={`field-input ${formErrors[field.key] ? "wt-form-input--error" : ""}`}
```

And the error div (line 703):
```tsx
<div className="wt-field-error">{formErrors[field.key]}</div>
```

- [ ] **Step 4: Align button CSS classes**

Replace button class names throughout App.tsx:
- `btn-primary` → `wt-btn wt-btn--primary`
- `btn-secondary` → `wt-btn wt-btn--secondary`
- `btn-icon` → `wt-btn wt-btn--ghost wt-btn--sm`
- `btn-danger` → `wt-btn wt-btn--danger wt-btn--sm`
- `btn-submit` → `wt-btn wt-btn--primary`

- [ ] **Step 5: Align modal CSS classes**

Replace:
- `modal-overlay` → `wt-modal-overlay`
- `modal` → `wt-modal`
- `modal-actions` → `wt-modal-footer`

- [ ] **Step 6: Fix hardcoded colors in App.css**

```css
/* Remove .btn-submit and .btn-submit:hover hardcoded shadows */
/* Replace: box-shadow: 0 2px 8px rgba(0, 122, 212, 0.25) */
/* With: box-shadow: var(--shadow-sm) */

/* Remove .btn-danger:hover hardcoded #c82333 */
/* Replace with: background: var(--error-text) */

/* Remove modal overlay rgba(0,0,0,0.5) */
/* Replace with: background: var(--wt-modal-overlay-bg) */

/* Remove secondary button rgba(0,0,0,0.06) */
/* Replace with: box-shadow: var(--shadow-xs) */
```

- [ ] **Step 7: Commit**

```bash
git add plugins/password-manager/frontend/src/App.tsx plugins/password-manager/frontend/src/App.css
git commit -m "refactor(password-manager): adopt unified toast/form/button CSS

Replace emoji-prefixed toast with WorkTools.toast.*().
Align form classes to wt-form-*. Add loading state to submit.
Fix hardcoded colors: rgba shadows → var(--shadow-*).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 3: Adapt JSON Tools

**Files:**
- Modify: `plugins/json-tools/frontend/src/App.tsx`
- Modify: `plugins/json-tools/frontend/src/App.css`

- [ ] **Step 1: Replace success/error messages with WorkTools.toast**

Remove `successMessage` state and associated JSX. Replace:
- `setSuccessMessage(...)` → `WorkTools.toast.success(...)` / `WorkTools.toast.info(...)`
- Validation errors that appear as fixed messages should use toast

For formatting success (currently `setSuccessMessage`):
```typescript
// Before: setSuccessMessage('JSON 格式化完成')
// After: WorkTools.toast.success('JSON 格式化完成')
```

- [ ] **Step 2: Replace hardcoded syntax colors with tokens in App.css**

```css
/* Replace: color: #d32f2f → color: var(--wt-syntax-key) */
/* Replace: color: #2e7d32 → color: var(--wt-syntax-string) */
/* Replace: color: #1565c0 → color: var(--wt-syntax-number) */
/* Replace: color: #c62828 → color: var(--wt-syntax-boolean) */
/* Replace: color: #7f8c8d → color: var(--wt-syntax-null) */
```

Also replace `background: rgba(211, 47, 47, 0.06)` etc. with token-based equivalents:
```css
/* Error location color: #a33 → var(--error-text) */
/* Error suggestion: #2e7d32 → var(--success-text) */
```

- [ ] **Step 3: Remove .error-message / .json-success CSS classes from App.css**

These are now provided by INJECTED_TOKENS (as deprecated, but still available). Remove the plugin's custom definitions; the WorkTools toast replaces them.

- [ ] **Step 4: Commit**

```bash
git add plugins/json-tools/frontend/src/App.tsx plugins/json-tools/frontend/src/App.css
git commit -m "refactor(json-tools): adopt WorkTools.toast, tokenize syntax colors

Replace success/error messages with WorkTools.toast.*().
Replace hardcoded syntax highlight colors with --wt-syntax-* tokens.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 4: Adapt Auth Plugin

**Files:**
- Modify: `plugins/auth-plugin/frontend/src/App.tsx`
- Modify: `plugins/auth-plugin/frontend/src/App.css`

- [ ] **Step 1: Replace toast with WorkTools.toast**

Same pattern as Task 2. Remove `error` state, replace all `setError(...)` + `setTimeout` with `WorkTools.toast.*()`.

- [ ] **Step 2: Align form classes**

Change `field-error` → `wt-field-error`, `field-input-error` → `wt-form-input--error`.

- [ ] **Step 3: Align button classes**

Same as Task 2: `btn-primary` → `wt-btn wt-btn--primary`, etc.

- [ ] **Step 4: Align modal classes**

`modal-overlay` → `wt-modal-overlay`, `modal` → `wt-modal`, `modal-actions` → `wt-modal-footer`.

- [ ] **Step 5: Fix hardcoded colors in App.css**

```css
/* .btn-danger:hover: color: #c82333 → color: var(--error-text) */
/* .modal-overlay: background: rgba(0,0,0,0.5) → var(--wt-modal-overlay-bg) */
/* select chevron fill: #666 → var(--text-tertiary) */
```

- [ ] **Step 6: Commit**

```bash
git add plugins/auth-plugin/frontend/src/App.tsx plugins/auth-plugin/frontend/src/App.css
git commit -m "refactor(auth-plugin): adopt unified toast/form/button CSS

Replace emoji-prefixed toast with WorkTools.toast.*().
Align form/modal/button classes to wt-* naming.
Fix hardcoded colors #c82333, #666, rgba overlay.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 5: Adapt Text Diff

**Files:**
- Modify: `plugins/text-diff/frontend/src/App.tsx`
- Modify: `plugins/text-diff/frontend/src/App.css`

- [ ] **Step 1: Replace error-banner with WorkTools.toast**

Remove `error` state and the `.error-banner` JSX block (lines 84-89). Replace:
```typescript
// Before:
setError(`读取文件失败: ${file.name}`);
setTimeout(() => setError(null), 3000);

// After:
WorkTools.toast.error(`读取文件失败: ${file.name}`);
```

- [ ] **Step 2: Align button classes in Toolbar.tsx**

Replace all button classes in `plugins/text-diff/frontend/src/Toolbar.tsx`:
- Custom button classes → `wt-btn wt-btn--secondary wt-btn--sm`

- [ ] **Step 3: Remove .error-banner CSS from App.css**

Remove the `.error-banner` and `.close-btn` CSS rules.

- [ ] **Step 4: Commit**

```bash
git add plugins/text-diff/frontend/src/App.tsx plugins/text-diff/frontend/src/App.css plugins/text-diff/frontend/src/Toolbar.tsx
git commit -m "refactor(text-diff): replace error-banner with WorkTools.toast

Replace full-width error banner with unified top-right toast.
Align toolbar buttons to wt-btn-* classes.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 6: Adapt DB Doc

**Files:**
- Modify: `plugins/db-doc/frontend/src/App.tsx`
- Modify: `plugins/db-doc/frontend/src/App.css`

- [ ] **Step 1: Rename toast to WorkTools.toast**

Replace the `showToast` helper and `toasts` state with direct `WorkTools.toast.*()` calls:

```typescript
// Remove: const [toasts, setToasts] = useState<ToastMessage[]>([])
// Remove: showToast function (lines 108-112)
// Remove: toast JSX block (lines 462-472)
// Remove: ToastMessage interface (lines 5-9)

// Replace showToast('success', ...) → WorkTools.toast.success(...)
// Replace showToast('error', ...) → WorkTools.toast.error(...)
// Replace showToast('info', ...) → WorkTools.toast.info(...)
```

- [ ] **Step 2: Add inline field validation to ConnectionForm**

```typescript
// In ConnectionForm, add validation before submit:
const handleSubmit = async (e: React.FormEvent) => {
  e.preventDefault()

  // Validate
  let valid = true
  const nameInput = (e.target as HTMLFormElement).querySelector('[name="name"]') as HTMLInputElement
  const dbInput = (e.target as HTMLFormElement).querySelector('[name="database"]') as HTMLInputElement
  if (!config.name?.trim()) {
    if (nameInput) WorkTools.FieldError.show(nameInput, '连接名称不能为空')
    valid = false
  }
  if (!config.database?.trim()) {
    if (dbInput) WorkTools.FieldError.show(dbInput, '数据库名不能为空')
    valid = false
  }
  if (!valid) return

  setSaving(true)
  try { await onSave(config) } finally { setSaving(false) }
}
```

Add `name` attributes to inputs for querySelector targeting:
```tsx
<input className="wt-form-input" name="name" ... />
<input className="wt-form-input" name="database" ... />
```

- [ ] **Step 3: Add loading spinner to export button**

```tsx
<button
  className="wt-btn wt-btn--primary export-btn"
  disabled={exporting || !outputDir || exportFormats.size === 0 || selectedTables.size === 0}
  onClick={handleExport}
>
  {exporting && <span className="wt-spinner" />}
  {exporting ? '导出中...' : `导出 ${selectedTables.size} 张表`}
</button>
```

- [ ] **Step 4: Align CSS classes**

Replace in App.tsx:
- `btn-primary` → `wt-btn wt-btn--primary`
- `btn-secondary` → `wt-btn wt-btn--secondary`
- `btn-danger` → `wt-btn wt-btn--danger`
- `btn-back` → `wt-btn wt-btn--ghost wt-btn--sm`

In App.css, rename:
- `.toast` → `.wt-toast` (or remove, since WorkTools handles creation)
- `.toast-container` → `.wt-toast-container` (or remove)
- `.toast--success` → `.wt-toast--success` (or remove)
- `.toast--error` → `.wt-toast--error` (or remove)
- `.toast--info` → `.wt-toast--info` (or remove)
- `.spinner` → `.wt-spinner` (keep for non-WorkTools usage)

Actually, since WorkTools creates toast DOM elements directly, the toast CSS is now provided by INJECTED_TOKENS. **Remove** the `.toast*` CSS from App.css entirely.

- [ ] **Step 5: Commit**

```bash
git add plugins/db-doc/frontend/src/App.tsx plugins/db-doc/frontend/src/App.css
git commit -m "refactor(db-doc): adopt WorkTools.toast, add form validation, loading state

Replace custom toast system with WorkTools.toast.*().
Add inline field validation for name/database fields.
Add loading spinner to export button.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 7: Adapt K8s Forward

**Files:**
- Modify: `plugins/k8s-forward/frontend/src/App.css`
- Modify: `plugins/k8s-forward/frontend/src/components/TabSshForward.tsx`
- Modify: `plugins/k8s-forward/frontend/src/components/TabK8sForward.tsx`

- [ ] **Step 1: Replace per-tab toast with WorkTools.toast**

In `TabSshForward.tsx` and `TabK8sForward.tsx`:
- Remove `const [toast, setToast] = useState<string | null>(null)` and its JSX
- Replace `setToast(msg); setTimeout(() => setToast(null), 3000)` with `WorkTools.toast.info(msg)` / `WorkTools.toast.error(msg)`

The `showToast` helper in each tab:
```typescript
// Before (TabSshForward.tsx):
const showToast = (msg: string, isErr?: boolean) => {
  setToast(msg);
  setTimeout(() => setToast(null), 3000);
};

// After: Remove showToast, use:
// WorkTools.toast.error(msg) for errors
// WorkTools.toast.success(msg) for success
// WorkTools.toast.info(msg) for info
```

- [ ] **Step 2: Fix hardcoded colors in App.css**

```css
/* .btn-danger:hover: background → use var(--error-text) */
/* Empty table state: color: #666 → var(--text-tertiary) */
/* Select chevron fill: #666 → var(--text-tertiary) */
/* Modal overlay: rgba(0,0,0,0.5) → var(--wt-modal-overlay-bg) */
```

- [ ] **Step 3: Add inline validation for connection forms**

In each tab component, add required-field validation before form submission:
```typescript
// Example: SSH forward form
const handleConnect = () => {
  const hostInput = document.querySelector('.ssh-host-input') as HTMLInputElement;
  if (!sshHost.trim()) {
    WorkTools.FieldError.show(hostInput, 'SSH 主机地址不能为空');
    return;
  }
  // ... existing connect logic
}
```

- [ ] **Step 4: Commit**

```bash
git add plugins/k8s-forward/frontend/src/
git commit -m "refactor(k8s-forward): adopt WorkTools.toast, fix hardcoded colors

Replace per-tab toast with WorkTools.toast.*() (fixes isErr bug).
Add inline validation for connection forms.
Fix #666 empty state and rgba modal overlay colors.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 8: Adapt DB Router

**Files:**
- Modify: `plugins/db-router/frontend/src/App.tsx`
- Modify: `plugins/db-router/frontend/src/App.css`

- [ ] **Step 1: Replace toast with WorkTools.toast**

Remove `Toast` interface, `toasts` state, toast JSX, and `addToast` helper. Replace all call sites:
```typescript
// Remove interface Toast, state toasts, addToast function, toast JSX block
// Replace addToast('success', msg) → WorkTools.toast.success(msg)
// Replace addToast('error', msg) → WorkTools.toast.error(msg)
// Replace addToast('info', msg) → WorkTools.toast.info(msg)
```

- [ ] **Step 2: Change validation to inline field hints**

Form validation errors (currently shown via toast) should use `WorkTools.FieldError.show()`:
```typescript
// Before:
if (!formData.name.trim()) {
  addToast('error', '规则名称不能为空');
  return;
}

// After:
const nameInput = document.querySelector('.rule-name-input') as HTMLInputElement;
if (!formData.name.trim()) {
  WorkTools.FieldError.show(nameInput, '规则名称不能为空');
  return;
}
```

Add `onInput` handlers to clear field errors:
```tsx
<input
  className="wt-form-input rule-name-input"
  value={formData.name}
  onInput={(e) => {
    setFormData({...formData, name: (e.target as HTMLInputElement).value});
    WorkTools.FieldError.clear(e.target as HTMLInputElement);
  }}
/>
```

- [ ] **Step 3: Fix modal overlay color**

In App.css:
```css
/* .modal-overlay background: rgba(15, 23, 42, 0.5) → var(--wt-modal-overlay-bg) */
```

- [ ] **Step 4: Remove toast CSS from App.css**

Remove `.toast`, `.toast-success`, `.toast-error`, `.toast-info`, `.toast-container` CSS rules.

- [ ] **Step 5: Commit**

```bash
git add plugins/db-router/frontend/src/App.tsx plugins/db-router/frontend/src/App.css
git commit -m "refactor(db-router): adopt WorkTools.toast, inline field validation

Replace custom toast with WorkTools.toast.*().
Change form validation from toast to inline field hints.
Fix modal overlay hardcoded color.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 9: Adapt Object Storage

**Files:**
- Modify: `plugins/object-storage/frontend/src/App.tsx`
- Modify: `plugins/object-storage/frontend/src/App.css`

- [ ] **Step 1: Replace error/success state toast with WorkTools.toast**

Remove `error`, `success` state variables, `showError`, `showSuccess` helpers, and their JSX blocks. Replace:
```typescript
// Remove: showError, showSuccess, setError, setSuccess state, toast JSX
// Replace showError(msg) → WorkTools.toast.error(msg)
// Replace showSuccess(msg) → WorkTools.toast.success(msg)
```

- [ ] **Step 2: Add inline field validation to connection form**

```typescript
// Before submit:
const handleSaveConnection = () => {
  const nameInput = document.querySelector('.conn-name-input') as HTMLInputElement;
  if (!connForm.name.trim()) {
    WorkTools.FieldError.show(nameInput, '连接名称不能为空');
    return;
  }
  // ... existing logic
}
```

- [ ] **Step 3: Add delete confirmation modal**

```tsx
{/* Delete confirm modal */}
{showDeleteConfirm && (
  <div className="wt-modal-overlay">
    <div className="wt-modal">
      <div className="wt-modal-header">
        <h3>确认删除</h3>
      </div>
      <div className="wt-modal-body">
        确定要删除对象 "{deleteTarget}" 吗？此操作不可撤销。
      </div>
      <div className="wt-modal-footer">
        <button className="wt-btn wt-btn--secondary" onClick={() => setShowDeleteConfirm(false)}>取消</button>
        <button className="wt-btn wt-btn--danger" onClick={handleConfirmDelete}>删除</button>
      </div>
    </div>
  </div>
)}
```

Add state: `const [showDeleteConfirm, setShowDeleteConfirm] = useState(false)` and `const [deleteTarget, setDeleteTarget] = useState('')`.

- [ ] **Step 4: Fix hardcoded colors in App.css**

```css
/* .btn-danger:hover: background: #c82333 → var(--error-text) */
/* .btn-sm.btn-danger:hover: background: #c82333 → var(--error-text) */
/* select chevron fill: #666 → var(--text-tertiary) */
/* toast shadow: rgba(0,0,0,0.1) → var(--shadow-sm) */
```

- [ ] **Step 5: Commit**

```bash
git add plugins/object-storage/frontend/src/App.tsx plugins/object-storage/frontend/src/App.css
git commit -m "refactor(object-storage): adopt WorkTools.toast, add validation + delete confirm

Replace manual error/success toast with WorkTools.toast.*() (auto-dismiss).
Add inline field validation for connection form.
Add delete confirmation modal using wt-modal-* classes.
Fix hardcoded colors #c82333, #666.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 10: Update CLAUDE.md

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Add plugin frontend development section**

Insert after "### 前端插件开发" (current line ~97, after the existing frontend paragraph):

```markdown
### 前端开发规范

**反馈提示**:
- 操作成功/失败 → `WorkTools.toast.success(msg)` / `.error(msg)` / `.info(msg)` / `.warning(msg)`
- 禁止自行实现 toast 或使用 alert()
- Toast 自动消失 3s，click 可提前关闭，支持多条同时显示

**表单校验**:
- 必须逐字段校验，失焦触发校验，输入时清除本字段错误
- 校验错误显示在本字段下方：`WorkTools.FieldError.show(inputEl, msg)`
- 提交前全量校验，有任一错误不提交
- 禁止用 toast 显示校验错误
- 禁止使用原生 `alert()` 或 `confirm()` 进行用户交互

**CSS 变量**:
- 所有颜色必须使用 `var(--xxx)` 设计令牌，禁止硬编码色值（如 `#c82333`、`#666`、`rgba(0,0,0,0.5)`）
- 按钮统一使用：`.wt-btn--primary` / `.wt-btn--secondary` / `.wt-btn--danger` / `.wt-btn--ghost`
- 模态框统一使用：`.wt-modal-overlay` / `.wt-modal` / `.wt-modal-header` / `.wt-modal-body` / `.wt-modal-footer`
- 空状态：`.wt-empty-state`
- 加载态：按钮内嵌 `.wt-spinner` + disabled 状态

**组件规范**:
- 删除/不可逆操作必须使用 `.wt-modal-*` 确认弹窗
- 提交/导出等异步操作按钮必须有 loading 态（`.wt-spinner` + disabled）
- 表单输入框使用 `.wt-form-input`，标签使用 `.wt-form-label`，容器使用 `.wt-form-group`
```

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: add plugin frontend development standards to CLAUDE.md

Document toast/form/button/modal conventions for future plugin development.
Enforce CSS token usage, inline validation, and loading state rules.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Verification

After all tasks complete, run:

```bash
# Type check
cd tauri-app && npx tsc --noEmit

# Rust check
cargo check

# Build all plugin frontends
for p in plugins/*/frontend; do
  if [ -f "$p/package.json" ]; then
    echo "Building $p..."
    cd "$p" && npm run build && cd -
  fi
done

# Full Rust test suite
cargo test
```

Expected: all checks pass, all plugin frontend builds succeed.

---

## Rollback Plan

Each task is an isolated commit. To roll back any plugin's changes: `git revert <commit-hash>`. The INJECTED_TOKENS change (Task 1) is backward-compatible — old `.error-message` classes remain available.
