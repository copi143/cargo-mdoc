import path from "path";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import l10nData from "./src/l10n/config.ts";

const projSrcDir = path.join(import.meta.dirname, "src");
const l10nIndex = path.join(projSrcDir, "l10n/index.ts");

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
      name: "l10n-auto-bind",
      enforce: "pre",
      transform(code, id) {
        // 仅处理主请求；?svelte&type=style 等子请求会直接跳过，
        // 否则 JS 注入会进入 CSS 模块导致 postcss 解析失败
        if (id.includes("?")) return null;
        const file = id.split("?")[0];
        if (!file.startsWith(projSrcDir) || !file.endsWith(".svelte")) return null;
        const ns = path.relative(projSrcDir, file).replace(/\.svelte$/, "");
        let relImport = path.relative(path.dirname(file), l10nIndex).replace(/\.ts$/, "");
        if (!relImport.startsWith(".")) relImport = "./" + relImport;
        const inject = `import { l10nDerive } from "${relImport}";\n  $: l10n = $l10nDerive("${ns}");\n`;
        const scriptMatch = code.match(/<script(?![^>]*\bcontext=)[^>]*>/);
        if (scriptMatch && scriptMatch.index !== undefined) {
          const at = scriptMatch.index + scriptMatch[0].length;
          return {
            code: code.slice(0, at) + "\n" + inject + code.slice(at),
            map: null,
          };
        }
        return {
          code: `<script lang="ts">\n${inject}</script>\n\n` + code,
          map: null,
        };
      },
    },
  ],
});
