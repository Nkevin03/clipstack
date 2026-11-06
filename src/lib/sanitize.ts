/** Minimal HTML sanitizer for rich-text previews: keeps formatting tags, drops everything active. */
const ALLOWED = new Set(["B", "STRONG", "I", "EM", "U", "S", "P", "BR", "DIV", "SPAN", "UL", "OL", "LI", "H1", "H2", "H3", "H4", "H5", "H6", "BLOCKQUOTE", "CODE", "PRE", "A", "TABLE", "TBODY", "THEAD", "TR", "TD", "TH", "SUB", "SUP", "MARK"]);

export function sanitizeHtml(html: string): string {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const walk = (node: Element) => {
    for (const child of Array.from(node.children)) {
      if (!ALLOWED.has(child.tagName)) {
        if (["SCRIPT", "STYLE", "IFRAME", "OBJECT", "EMBED", "LINK", "META", "IMG", "SVG", "VIDEO", "AUDIO", "FORM", "INPUT", "BUTTON"].includes(child.tagName)) {
          child.remove();
          continue;
        }
        // Unwrap unknown containers but keep their text.
        walk(child);
        child.replaceWith(...Array.from(child.childNodes));
        continue;
      }
      for (const attr of Array.from(child.attributes)) {
        const keep = child.tagName === "A" && attr.name === "href" && /^https?:|^mailto:/i.test(attr.value);
        if (!keep) child.removeAttribute(attr.name);
      }
      walk(child);
    }
  };
  walk(doc.body);
  return doc.body.innerHTML;
}
