import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  build: {
    outDir: '../assets',
    emptyOutDir: true,
    rollupOptions: {
      output: {
        entryFileNames: 'main.js',
        chunkFileNames: 'main.js',
        assetFileNames: (assetInfo) => {
          const name = assetInfo.names?.[0] ?? '';
          if (name === 'index.html') return 'index.html';
          if (name.endsWith('.css')) return 'styles.css';
          // 字体文件和其他资源使用原始名称
          return '[name].[ext]';
        },
        // 减少代码分割 - 将所有代码打包成少量文件
        manualChunks: (id) => {
          // 将 Monaco 的所有模块合并到一个文件
          if (id.includes('monaco-editor')) {
            return 'monaco';
          }
          // 其他模块合并
          return 'vendor';
        }
      }
    }
  }
});
