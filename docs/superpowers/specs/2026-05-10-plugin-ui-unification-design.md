# Plugin Frontend UI Unification Design

**Date**: 2026-05-10
**Status**: Approved

## Scope

Unify UI patterns across all 8 plugin frontends without modifying plugin functionality.

- **Toast**: single shared system, top-right, multi-stack, auto-dismiss
- **Form validation**: inline field hints under each input, validate on blur + on submit
- **Common CSS classes**: buttons, modals, empty states, badges, spinners
- **Hardcoded colors**: replace with CSS tokens
- **CLAUDE.md**: add plugin frontend development constraints

## Approach

**Option B — CSS + JS utility injection** (chosen).

Extend `PluginPlaceholder.tsx` to inject both standardized CSS classes and a lightweight `window.WorkTools` JS utility into each plugin iframe's srcdoc. Plugins call `WorkTools.toast.success(msg)` instead of building their own toast state management.

Why: plugin iframes are fully isolated — cannot share React components. Injecting CSS + JS utility is the only way to achieve behavioral consistency without a framework dependency.

---

## Design

### 1. Toast System

**CSS** (injected via INJECTED_TOKENS):

| Class | Purpose |
|---|---|
| `.wt-toast-container` | Fixed top-right, z-index 9999, flex column gap |
| `.wt-toast` | Base: radius, shadow, padding, slide-in animation from right |
| `.wt-toast--success` | `var(--success-light)` bg, `var(--success-text)` text |
| `.wt-toast--error` | `var(--error-light)` bg, `var(--error-text)` text |
| `.wt-toast--info` | `var(--accent-light)` bg, `var(--accent)` text |
| `.wt-toast--warning` | `var(--warning-light)` bg, `var(--warning-text)` text |

**JS utility** (injected via INJECTED_SCRIPTS):

```js
window.WorkTools = {
  toast: {
    success(msg) { /* create .wt-toast--success, auto-dismiss 3s, click to close */ },
    error(msg)   { /* .wt-toast--error */ },
    info(msg)    { /* .wt-toast--info */ },
    warning(msg) { /* .wt-toast--warning */ }
  }
}
```

Supports multiple simultaneous toasts stacked vertically. Each toast dismisses on click or after 3s.

### 2. Form Validation

**CSS** (injected via INJECTED_TOKENS):

| Class | Purpose |
|---|---|
| `.wt-form-group` | Bottom margin spacing |
| `.wt-form-label` | Unified font size, color, spacing |
| `.wt-form-input` | Unified input/select/textarea: border, radius, focus ring |
| `.wt-form-input--error` | Red border, red focus ring |
| `.wt-field-error` | Small red text below input |

**JS utility**:

```js
window.WorkTools.FieldError = {
  show(inputEl, message) { /* append .wt-field-error span, add .wt-form-input--error */ },
  clear(inputEl)         { /* remove error + error class from one field */ },
  clearAll(formEl)       { /* remove all field errors in form */ }
}
```

**Validation rules**:
- Validate on blur; clear this field's error on input
- Full validation on submit; block submit if any field fails
- Backend field-level errors use same display pattern

### 3. Common Component CSS

**Buttons**:

| Class | Purpose |
|---|---|
| `.wt-btn` | Base: padding, radius, font, cursor, transition |
| `.wt-btn--primary` | `var(--accent)` bg + `var(--text-inverse)` text |
| `.wt-btn--secondary` | `var(--bg-secondary)` bg + border |
| `.wt-btn--danger` | `var(--error)` bg + `var(--text-inverse)` text |
| `.wt-btn--ghost` | Transparent, hover shows `var(--hover-bg)` |
| `.wt-btn--sm` | Smaller padding variant |

**Modals**:

| Class | Purpose |
|---|---|
| `.wt-modal-overlay` | Fixed fullscreen, `var(--wt-modal-overlay-bg)` |
| `.wt-modal` | Centered card, `var(--bg-primary)` bg |
| `.wt-modal-header` | Title + close button |
| `.wt-modal-body` | Scrollable content area |
| `.wt-modal-footer` | Button row, flex-end justified |

**Misc**:

| Class | Purpose |
|---|---|
| `.wt-empty-state` | Centered muted text (icon + message) |
| `.wt-badge` | Small inline label chip |
| `.wt-status-dot` | Green/gray circle dot |
| `.wt-spinner` | CSS rotation animation |

### 4. JSON Syntax Highlight Tokens

New CSS tokens for JSON Tools syntax coloring, supporting dark mode:

| Token | Light | Dark |
|---|---|---|
| `--wt-syntax-key` | `#d32f2f` | `#ff6b6b` |
| `--wt-syntax-string` | `#2e7d32` | `#69db7c` |
| `--wt-syntax-number` | `#1565c0` | `#74b9ff` |
| `--wt-syntax-boolean` | `#c62828` | `#ff6b6b` |
| `--wt-syntax-null` | `#7f8c8d` | `#adb5bd` |

Defined in INJECTED_TOKENS alongside existing tokens, with `[data-theme="dark"]` overrides.

---

## Per-Plugin Adaptation

### Password Manager
- Replace emoji-prefixed toast with `WorkTools.toast.*()`
- Align form classes to `wt-form-*`, `wt-field-error`
- Add loading spinner on submit button during save
- Fix hardcoded `rgba(0,122,212,0.25)` submit button shadow → `var(--shadow-sm)`

### JSON Tools
- Replace hardcoded syntax colors with `--wt-syntax-*` tokens
- Replace `json-error` / `json-success` fixed messages with `WorkTools.toast`

### Auth Plugin
- Replace emoji-prefixed toast with `WorkTools.toast.*()`
- Align form classes to `wt-form-*`
- Fix `#c82333` → `var(--error-text)` on `.btn-danger:hover`
- Fix `#666` select chevron → `var(--text-tertiary)`

### Text Diff
- Replace full-width `error-banner` with `WorkTools.toast.error()`
- Align button classes to `wt-btn-*`

### DB Doc
- Add inline field validation (currently only `required` attribute)
- Add loading spinner on export button
- Align toast classes to `wt-toast-*` (already best implementation, just rename)

### K8s Forward
- Replace `showToast(msg, isErr)` with `WorkTools.toast.*()` — fixes bug where error style was never applied
- Add inline field validation for required connection fields
- Fix `#666` empty state text → `var(--text-tertiary)`

### DB Router
- Replace custom toast with `WorkTools.toast.*()`
- Change validation error display from toast to inline field hints
- Fix modal overlay color to match unified `wt-modal-overlay`

### Object Storage
- Replace manual `error`/`success` state toast with `WorkTools.toast.*()` (auto-dismiss)
- Add inline field validation for connection form
- Add delete confirmation modal using `.wt-modal-*` classes
- Fix `#c82333` → `var(--error-text)`

---

## Files Changed

| File | Change |
|---|---|
| `tauri-app/src/components/PluginPlaceholder.tsx` | Extend INJECTED_TOKENS with ~200 lines CSS; add INJECTED_SCRIPTS with WorkTools JS |
| `plugins/password-manager/frontend/src/App.tsx` | Toast + form + loading adapt |
| `plugins/password-manager/frontend/src/App.css` | Class rename + hardcoded fix |
| `plugins/json-tools/frontend/src/App.tsx` | Toast adapt |
| `plugins/json-tools/frontend/src/App.css` | Syntax tokens + message classes remove |
| `plugins/auth-plugin/frontend/src/App.tsx` | Toast + form adapt |
| `plugins/auth-plugin/frontend/src/App.css` | Hardcoded fix |
| `plugins/text-diff/frontend/src/App.tsx` | Error banner → toast |
| `plugins/text-diff/frontend/src/App.css` | Button rename |
| `plugins/db-doc/frontend/src/App.tsx` | Validation + loading + toast adapt |
| `plugins/db-doc/frontend/src/App.css` | Toast class rename |
| `plugins/k8s-forward/frontend/src/App.tsx` | Toast + validation adapt |
| `plugins/k8s-forward/frontend/src/components/*.tsx` | Toast adapt per tab |
| `plugins/k8s-forward/frontend/src/App.css` | Hardcoded fix + class rename |
| `plugins/db-router/frontend/src/App.tsx` | Toast + validation adapt |
| `plugins/db-router/frontend/src/App.css` | Toast class rename + modal overlay fix |
| `plugins/object-storage/frontend/src/App.tsx` | Toast + validation + delete confirm |
| `plugins/object-storage/frontend/src/App.css` | Hardcoded fix + modal classes |
| `CLAUDE.md` | Add plugin frontend development section |

## Not Changed

- Plugin backend code
- Plugin API interface (`pluginAPI.call`, etc.)
- Build/packaging scripts
- Main app sidebar, LogViewer, PluginStore
- JSON Tools syntax coloring logic (only color values replaced with tokens)

---

## CLAUDE.md Addition

A new section to be added under "插件开发要点":

```markdown
### 前端开发规范

**反馈提示**:
- 操作成功/失败 → `WorkTools.toast.success(msg)` / `.error(msg)` / `.info(msg)` / `.warning(msg)`
- 禁止自行实现 toast 或使用 alert()
- Toast 自动消失 3s，click 可提前关闭

**表单校验**:
- 必须逐字段校验，失焦触发，输入清错
- 校验错误显示在本字段下方：`WorkTools.FieldError.show(inputEl, msg)`
- 提交前全量校验，有任一错误不提交
- 禁止用 toast 显示校验错误

**CSS 变量**:
- 所有颜色必须使用 `var(--xxx)` 设计令牌，禁止硬编码
- 按钮：`.wt-btn--primary` / `.wt-btn--secondary` / `.wt-btn--danger` / `.wt-btn--ghost`
- 模态框：`.wt-modal-overlay` / `.wt-modal` / `.wt-modal-header` / `.wt-modal-body` / `.wt-modal-footer`
- 空状态：`.wt-empty-state`
- 加载态：`.wt-spinner`

**组件规范**:
- 删除/不可逆操作必须使用 `.wt-modal-*` 确认弹窗
- 提交按钮必须有 loading 态（`.wt-spinner` + disabled）
```
