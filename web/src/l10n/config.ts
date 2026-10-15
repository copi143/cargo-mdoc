import fs from "fs";
import path from "path";
import { z } from "zod";
import { globSync } from "glob";
import { parse } from "smol-toml";

export const L10nConfigSchema = z
  .object({
    default: z.string(),
  })
  .catchall(
    z.object({
      name: z.string(),
      display: z.string(),
      fallback: z.string(),
    }),
  );

export type L10nConfig = z.infer<typeof L10nConfigSchema>;

export const l10nConfig = (() => {
  const configToml = fs.readFileSync(
    path.join(import.meta.dirname, "config.toml"),
    "utf-8",
  );
  const config = parse(configToml);
  const result = L10nConfigSchema.safeParse(config);
  if (!result.success) {
    console.error("Failed to parse l10n config:", result.error);
    throw new Error("Invalid l10n config");
  }
  return result.data;
})();

export const L10nFileSchema = z.record(
  z.string(),
  z.object({ fallback: z.boolean().optional() }).catchall(z.string()),
);

export type L10nFile = z.infer<typeof L10nFileSchema>;

export const l10nFile = (() => {
  const projSrcDir = path.join(import.meta.dirname, "..");
  const files = globSync(path.join(projSrcDir, "**/*.l10n"), { nodir: true });
  const l10nFiles = {} as Record<string, L10nFile>;
  let hasError = false;
  for (const file of files) {
    const toml = fs.readFileSync(file, "utf-8");
    const l10n = parse(toml);
    const result = L10nFileSchema.safeParse(l10n);
    if (!result.success) {
      console.error("Failed to parse l10n file:", result.error);
      hasError = true;
      continue;
    }
    const relPath = path.relative(projSrcDir, file);
    l10nFiles[relPath] = result.data;
  }
  if (hasError) {
    throw new Error("Invalid l10n file");
  }
  return l10nFiles;
})();

export default (() => {
  const langs = Object.fromEntries(
    Object.entries(l10nConfig).filter(([k, v]) => k !== "default"),
  ) as Record<string, { name: string; display: string; fallback: string }>;
  const fallbackMap = Object.fromEntries(
    Object.entries(langs).map(([k, v]) => [k, v.fallback]),
  ) as Record<string, string>;
  const trans = Object.fromEntries(
    Object.keys(langs).map((k) => [
      k,
      {} as Record<string, Record<string, string>>,
    ]),
  );
  for (const [file, data] of Object.entries(l10nFile)) {
    const prefix = file.replace(/\.l10n$/, "");
    for (const lang of Object.keys(langs)) {
      if (trans[lang][prefix] === undefined) {
        trans[lang][prefix] = {};
      }
    }
    for (const [key, value] of Object.entries(data)) {
      for (const origLang of Object.keys(langs)) {
        let text = value[origLang];
        if (text === undefined && value.fallback !== true) {
          throw new Error(
            `Missing translation for key "${key}" in language "${origLang}" in file "${file}"`,
          );
        }
        let lang = origLang;
        while (text === undefined) {
          lang = fallbackMap[lang];
          if (lang === "key") {
            text = key;
          } else {
            text = value[lang];
          }
        }
        trans[origLang][prefix][key] = text;
      }
    }
  }
  return {
    default: l10nConfig.default,
    langs,
    trans,
  };
})();
