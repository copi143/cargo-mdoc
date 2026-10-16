import MarkdownIt from "markdown-it";

const renderer = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: false,
});

export const renderMarkdown = (source?: string | null): string => {
  return renderer.render((source ?? "").trim());
};
