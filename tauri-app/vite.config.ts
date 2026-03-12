import { defineConfig } from "vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  // React 迁移完成 - 不再需要 Solid.js 插件
  plugins: [],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : {
          // 优化 HMR 配置
          overlay: true,
        },
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
      // 4. 监听 CSS 文件变化
      usePolling: false,
    },
  },
  // 优化 CSS 处理
  css: {
    devSourcemap: true,
  },
}));
