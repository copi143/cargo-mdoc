import path from "path";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import l10nData from "./src/l10n/config.ts";

const projSrcDir = path.join(import.meta.dirname, "src");

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    svelte(),
    {
      name: "toml-l10n",
      transform(code, id) {
        if (id === path.join(projSrcDir, "l10n/config.ts")) {
          console.log("Injecting l10n config data:", l10nData);
          return {
            code: `export default ${JSON.stringify(l10nData)}`,
            map: null,
          };
        }
      },
    },
    {
      name: "l10n-watch",
      configureServer(server) {
        server.watcher.add(path.join(projSrcDir, "**/*.l10n"));
        server.watcher.on("change", (file) => {
          if (file.endsWith(".l10n")) {
            server.restart();
          }
        });
      },
    },
    {
      name: "file-path",
      transform(code, id) {
        if (
          id.startsWith(projSrcDir) &&
          (id.endsWith(".ts") || id.endsWith(".svelte"))
        ) {
          const relPath = path.relative(projSrcDir, id);
          return {
            code: code.replace(
              /['"]\$\$_FILE_PATH_\$\$['"]/g,
              JSON.stringify(relPath),
            ),
            map: null,
          };
        }
      },
    },
  ],
});
