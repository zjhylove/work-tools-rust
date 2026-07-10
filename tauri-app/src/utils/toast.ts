/* ═══════════════════════════════════════════════════════
   Toast Notifications — host-level (non-iframe)
   DOM-based, mirrors the wt-toast pattern injected into
   plugin iframes so visual style stays consistent.
   ═══════════════════════════════════════════════════════ */

let container: HTMLElement | null = null;

function getContainer(): HTMLElement {
  if (!container || !document.body.contains(container)) {
    container = document.createElement('div');
    container.className = 'host-toast-container';
    document.body.appendChild(container);
  }
  return container;
}

function showToast(type: 'success' | 'error' | 'info' | 'warning', message: string): void {
  const el = document.createElement('div');
  el.className = `host-toast host-toast--${type}`;
  const icons: Record<string, string> = {
    success: '\u2713 ',
    error: '\u2717 ',
    info: '\u2139 ',
    warning: '\u26A0 ',
  };
  el.textContent = (icons[type] || '') + message;
  el.addEventListener('click', () => el.remove());
  getContainer().appendChild(el);
  setTimeout(() => {
    if (el.parentNode) el.remove();
    if (container && container.children.length === 0) {
      container.remove();
      container = null;
    }
  }, 3000);
}

export const toast = {
  success: (msg: string) => showToast('success', msg),
  error: (msg: string) => showToast('error', msg),
  info: (msg: string) => showToast('info', msg),
  warning: (msg: string) => showToast('warning', msg),
};
