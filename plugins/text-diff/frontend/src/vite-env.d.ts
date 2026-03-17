/// <reference types="vite/client" />

interface Window {
  pluginAPI: {
    call: (pluginId: string, method: string, params: any) => Promise<any>;
  };
}
