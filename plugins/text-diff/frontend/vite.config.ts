import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  base: './', // 使用相对路径,以便插件环境可以正确加载资源
  build: {
    outDir: '../assets',
    emptyOutDir: true,
    rollupOptions: {
      output: {
        entryFileNames: 'main.js',
        chunkFileNames: 'main.js', // 将所有 chunk 合并到 main.js
        assetFileNames: (assetInfo) => {
          if (assetInfo.name === 'index.html') return 'index.html';
          if (assetInfo.name?.endsWith('.css')) return 'styles.css';
          // 字体文件和其他资产文件保留原名称,放在 assets 子目录
          if (assetInfo.name?.match(/\.(woff|woff2|eot|ttf|otf|ttc)$/)) {
            return 'assets/[name][extname]';
          }
          // Monaco worker 文件
          if (assetInfo.name?.includes('.worker-')) {
            return 'assets/[name][extname]';
          }
          // 其他资产文件
          return 'assets/[name][extname]';
        },
        // 禁用代码分割,将所有代码打包到单个文件
        inlineDynamicImports: true,
      }
    }
  }
});
