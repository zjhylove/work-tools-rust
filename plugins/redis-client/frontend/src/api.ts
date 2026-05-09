export const COLORS = ['#ef4444', '#f59e0b', '#10b981', '#3b82f6', '#8b5cf6', '#ec4899', '#06b6d4', '#6b7280'];

declare global {
  interface Window {
    pluginAPI?: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

type ApiResult = Record<string, unknown>;

function waitForAPI(): Promise<NonNullable<typeof window.pluginAPI>> {
  return new Promise(resolve => {
    if (window.pluginAPI) return resolve(window.pluginAPI);
    const t = setInterval(() => {
      if (window.pluginAPI) { clearInterval(t); resolve(window.pluginAPI); }
    }, 50);
    setTimeout(() => clearInterval(t), 3000);
  });
}

export async function call(method: string, params?: Record<string, unknown>): Promise<ApiResult> {
  const api = await waitForAPI();
  const r = await api.call('redis-client', method, params || {});
  return (r || {}) as ApiResult;
}
