import path from "path";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import l10nData from "./src/l10n/config";

const projSrcDir = path.join(__dirname, "src");

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    svelte(),
    {
      name: "toml-l10n",
      transform(code, id) {
        if (id.endsWith("src/l10n/config.ts")) {
          console.log("Injecting l10n config data:", l10nData);
          return {
            code: `export default ${JSON.stringify(l10nData)}`,
            map: null,
          };
        }
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
