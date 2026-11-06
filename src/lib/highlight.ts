import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import cpp from "highlight.js/lib/languages/cpp";
import csharp from "highlight.js/lib/languages/csharp";
import css from "highlight.js/lib/languages/css";
import go from "highlight.js/lib/languages/go";
import java from "highlight.js/lib/languages/java";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import php from "highlight.js/lib/languages/php";
import python from "highlight.js/lib/languages/python";
import ruby from "highlight.js/lib/languages/ruby";
import rust from "highlight.js/lib/languages/rust";
import sql from "highlight.js/lib/languages/sql";
import swift from "highlight.js/lib/languages/swift";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";

const langs = { bash, cpp, csharp, css, go, java, javascript, json, php, python, ruby, rust, sql, swift, typescript, xml, html: xml };
for (const [name, def] of Object.entries(langs)) hljs.registerLanguage(name, def);

const cache = new Map<string, string>();

/** Highlighted HTML (escaped by highlight.js). */
export function highlight(code: string, language: string | null): string {
  const key = `${language}\u0000${code}`;
  const hit = cache.get(key);
  if (hit) return hit;
  let html: string;
  try {
    html = language && hljs.getLanguage(language) ? hljs.highlight(code, { language }).value : hljs.highlightAuto(code).value;
  } catch {
    html = code.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]!);
  }
  if (cache.size > 300) cache.clear();
  cache.set(key, html);
  return html;
}

export const languageLabel = (l: string | null) =>
  ({ javascript: "JavaScript", typescript: "TypeScript", csharp: "C#", cpp: "C/C++", json: "JSON", sql: "SQL", html: "HTML", xml: "XML", css: "CSS", php: "PHP" } as Record<string, string>)[l ?? ""] ??
  (l ? l[0].toUpperCase() + l.slice(1) : "Code");
