import { defineConfig, Plugin } from "vite";
import react from "@vitejs/plugin-react";

function customAssetsPlugin(): Plugin {
  return {
    name: "custom-assets-protocol",
    enforce: "post",
    generateBundle(_options, bundle) {
      // const isWatch = process.argv.includes("--watch");

      // Rewrite asset references in HTML files
      for (const fileName in bundle) {
        const chunk = bundle[fileName];
        if (chunk.type === "asset" && fileName.endsWith(".html")) {
          let html = chunk.source as string;

          // Inject timestamp header if in watch mode
          // if (isWatch) {
          const timestamp = new Date().toISOString();
          const timeHeader = `
              <script>
                console.log("Build timestamp: ${timestamp}");
              </script>
              <meta name="build-timestamp" content="${timestamp}">
            `;

          // Insert after opening head tag
          html = html.replace(/<head>/i, `<head>${timeHeader}`);
          // }

          // Rewrite asset paths
          chunk.source = html.replace(
            /(src|href)="([^"]*\/assets\/[^"]*?)"/g,
            (match, attr, url) => {
              // Only rewrite paths that point to the assets directory
              if (url.includes("/assets/")) {
                return `${attr}="assets://${url.slice(1)}"`;
              }
              return match;
            }
          );
        }
      }
    },
  };
}

export default defineConfig({
  server: {
    hmr: true, // Enable HMR
    cors: true,
    headers: {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type",
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
  },
  plugins: [react(), customAssetsPlugin()],
  base: "/",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      output: {
        // Ensure deterministic file names
        entryFileNames: "assets/index.js",
        chunkFileNames: "assets/chunk.[name].js",
        assetFileNames: (assetInfo) => {
          // Handle CSS files specially
          if (assetInfo.name?.endsWith(".css")) {
            return "assets/style.css";
          }
          // Other assets get predictable names
          return "assets/[name].[ext]";
        },
      },
    },
    // Disable chunk splitting
    target: "esnext",
    sourcemap: false,
    minify: "esbuild",
    cssMinify: true,
    cssCodeSplit: false,
    watch: {
      // Enable watch mode when --watch flag is present
      include: ["src/**", "assets/**"],
    },
  },
});
