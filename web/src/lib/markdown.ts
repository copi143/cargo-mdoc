import MarkdownIt from "markdown-it";

const renderer = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
});

export const renderMarkdown = (source?: string | null): string => {
  return renderer.render((source ?? "").trim());
};
