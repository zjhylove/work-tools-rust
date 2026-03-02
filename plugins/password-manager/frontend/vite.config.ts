import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  base: "./",
  build: {
    outDir: "../assets",
    emptyOutDir: true,
    rollupOptions: {
      output: {
        entryFileNames: "main.js",
        chunkFileNames: "chunks/[name].js",
        assetFileNames: (assetInfo) => {
          if (assetInfo.name === "index.html") return "index.html";
          if (assetInfo.name?.endsWith(".css")) return "styles.css";
          return "assets/[name][extname]";
        },
      },
    },
  },
});
