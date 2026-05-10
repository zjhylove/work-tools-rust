declare global {
  interface Window {
    pluginAPI?: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

type ApiResult = Record<string, unknown>;

function waitForAPI(): Promise<NonNullable<typeof window.pluginAPI>> {
  return new Promise((resolve, reject) => {
    if (window.pluginAPI) return resolve(window.pluginAPI);
    let elapsed = 0;
    const t = setInterval(() => {
      if (window.pluginAPI) { clearInterval(t); resolve(window.pluginAPI); }
      elapsed += 50;
      if (elapsed >= 5000) { clearInterval(t); reject(new Error('pluginAPI 注入超时')); }
    }, 50);
  });
}

export async function call(method: string, params?: Record<string, unknown>): Promise<ApiResult> {
  const api = await waitForAPI();
  const r = await api.call('redis-client', method, params || {});
  return (r ?? {}) as ApiResult;
}
