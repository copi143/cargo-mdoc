import "highlight.js/styles/github.css";
import hljs from "highlight.js/lib/core";
import rust from "highlight.js/lib/languages/rust";

hljs.registerLanguage("rust", rust);

export const renderRustSource = (source?: string | null): string => {
  const code = (source ?? "").trim();

  if (!code) return "";

  return hljs.highlight(code, { language: "rust" }).value;
};
