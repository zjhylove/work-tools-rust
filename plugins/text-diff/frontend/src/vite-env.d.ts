/// <reference types="vite/client" />

declare module 'monaco-editor' {
  export const editor: typeof import('monaco-editor/esm/vs/editor/editor.api');
}

interface Window {
  pluginAPI: {
    call: (pluginId: string, method: string, params: any) => Promise<any>;
  };
}
