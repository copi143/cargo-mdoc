import MarkdownIt from "markdown-it";

const renderer = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: false,
});

function shiftHeadings(html: string): string {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const headings = Array.from(doc.querySelectorAll("h1, h2, h3, h4, h5, h6"));
  if (headings.length === 0) return html;

  let maxLevel = Infinity;
  for (const h of headings) {
    const level = Number(h.tagName.charAt(1));
    if (!Number.isNaN(level) && level < maxLevel) maxLevel = level;
  }

  if (maxLevel > 2 || maxLevel === Infinity) return html;

  const shift = 3 - maxLevel; // maxLevel=1 -> 2, maxLevel=2 -> 1

  for (const h of headings) {
    const level = Number(h.tagName.charAt(1));
    const newLevel = Math.min(6, level + shift);
    const newTag = `h${newLevel}`;

    const replacement = doc.createElement(newTag);
    for (const attr of Array.from(h.attributes)) {
      replacement.setAttribute(attr.name, attr.value);
    }
    while (h.firstChild) replacement.appendChild(h.firstChild);
    h.parentNode?.replaceChild(replacement, h);
  }

  return doc.body.innerHTML;
}

export const renderMarkdown = (source?: string | null): string => {
  const html = renderer.render((source ?? "").trim());
  return shiftHeadings(html);
};
