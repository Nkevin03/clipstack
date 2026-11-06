import { useEffect, useMemo, useState } from "react";
import { FileText, Folder, Globe, Mail } from "lucide-react";
import { fileSrc, ipc, type Item } from "../../lib/ipc";
import { basename, bytes, colorFormats, domain, parseColor, readableOn } from "../../lib/format";
import { highlight, languageLabel } from "../../lib/highlight";
import { sanitizeHtml } from "../../lib/sanitize";
import { Highlighted } from "../Highlighted";
import { useT } from "../../lib/i18n";

type BodyProps = { item: Item; terms: string[] };

export function TextBody({ item, terms }: BodyProps) {
  const text = item.preview.split("\n").slice(0, 10).join("\n");
  return (
    <div className="h-full overflow-hidden whitespace-pre-wrap break-words px-3 pt-2.5 text-[12.5px] leading-[1.45]" style={{ color: "var(--cs-text)" }}>
      <Highlighted text={text} terms={terms} />
    </div>
  );
}

export function RichBody({ item, terms }: BodyProps) {
  const [html, setHtml] = useState<string | null>(null);
  useEffect(() => {
    // Only fetch formatted HTML when not searching (highlighting works on plain text).
    if (terms.length || !item.hasRich) return;
    let alive = true;
    ipc.getItem(item.id).then((d) => {
      if (alive && d?.html) setHtml(sanitizeHtml(d.html.slice(0, 20000)));
    });
    return () => {
      alive = false;
    };
  }, [item.id, item.hasRich, terms.length]);
  if (!html || terms.length) return <TextBody item={item} terms={terms} />;
  return <div className="cs-rich h-full px-3 pt-2.5" dangerouslySetInnerHTML={{ __html: html }} />;
}

export function LinkBody({ item, terms }: BodyProps) {
  const url = item.preview.trim();
  const isMail = url.startsWith("mailto:");
  const fav = fileSrc(item.linkFaviconPath);
  return (
    <div className="flex h-full flex-col gap-2 px-3 pt-3">
      <div className="flex items-center gap-2">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-[10px]" style={{ background: "var(--cs-field)" }}>
          {fav ? <img src={fav} alt="" className="h-5 w-5" /> : isMail ? <Mail size={18} strokeWidth={1.8} /> : <Globe size={18} strokeWidth={1.8} style={{ color: "var(--cs-accent)" }} />}
        </div>
        <div className="min-w-0">
          <div className="truncate text-[12.5px] font-semibold">{domain(url)}</div>
          {item.linkTitle && <div className="truncate text-[11.5px]" style={{ color: "var(--cs-text-muted)" }}>{item.linkTitle}</div>}
        </div>
      </div>
      <div className="line-clamp-5 break-all text-[11.5px] leading-snug" style={{ color: "var(--cs-accent)" }}>
        <Highlighted text={url} terms={terms} />
      </div>
    </div>
  );
}

export function ImageBody({ item }: BodyProps) {
  const src = fileSrc(item.thumbPath ?? item.imagePath);
  return (
    <div className="cs-checker relative h-full overflow-hidden">
      {src && <img src={src} alt="" loading="lazy" decoding="async" draggable={false} className="h-full w-full object-cover" />}
    </div>
  );
}

function FileIcon({ path }: { path: string }) {
  const [icon, setIcon] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    ipc.fileIcon(path).then((p) => alive && setIcon(p)).catch(() => {});
    return () => {
      alive = false;
    };
  }, [path]);
  if (icon) return <img src={fileSrc(icon)} alt="" className="h-14 w-14" draggable={false} />;
  return path.endsWith("/") ? <Folder size={44} strokeWidth={1.4} /> : <FileText size={44} strokeWidth={1.4} />;
}

export function FileBody({ item, terms }: BodyProps) {
  const t = useT();
  const first = item.filePaths[0] ?? "";
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 px-3 text-center">
      <div className="relative">
        {item.filePaths.length > 1 && (
          <div className="absolute -right-2 -top-1 rotate-6 opacity-50">
            <FileIcon path={item.filePaths[1]} />
          </div>
        )}
        <div className="relative">
          <FileIcon path={first} />
        </div>
      </div>
      <div className="w-full truncate text-[12.5px] font-medium">
        <Highlighted text={basename(first)} terms={terms} />
      </div>
      <div className="text-[11px]" style={{ color: "var(--cs-text-muted)" }}>
        {item.filePaths.length > 1 ? t("files", { n: item.filePaths.length }) : first.split("/").slice(0, -1).join("/").replace(/^\/Users\/[^/]+/, "~")}
      </div>
    </div>
  );
}

export function ColorBody({ item }: BodyProps) {
  const c = useMemo(() => parseColor(item.preview), [item.preview]);
  const f = c ? colorFormats(c) : null;
  const fg = readableOn(f?.hex);
  return (
    <div className="flex h-full flex-col">
      <div className="cs-checker relative flex-1">
        <div className="absolute inset-0 flex items-end p-3" style={{ background: item.preview.trim(), color: fg === "dark" ? "#111" : "#fff" }}>
          <span className="cs-mono text-[15px] font-semibold">{f?.hex ?? item.preview}</span>
        </div>
      </div>
      {f && (
        <div className="cs-mono space-y-0.5 px-3 py-2 text-[10.5px]" style={{ color: "var(--cs-text-muted)" }}>
          <div className="truncate">{f.rgb}</div>
          <div className="truncate">{f.hsl}</div>
        </div>
      )}
    </div>
  );
}

export function CodeBody({ item, terms }: BodyProps) {
  const code = item.preview.split("\n").slice(0, 14).join("\n");
  const html = useMemo(() => (terms.length ? null : highlight(code, item.language)), [code, item.language, terms.length]);
  return (
    <div className="relative h-full overflow-hidden" style={{ background: "var(--cs-code-bg)", color: "var(--cs-code-text)" }}>
      <span className="absolute right-2 top-1.5 rounded px-1.5 py-[1px] text-[9.5px] font-semibold uppercase tracking-wide" style={{ background: "rgb(255 255 255 / 0.1)", color: "#c9c9d1" }}>
        {languageLabel(item.language)}
      </span>
      {html !== null ? (
        <pre className="cs-mono m-0 whitespace-pre px-3 pt-2.5 text-[11px] leading-[1.5]" dangerouslySetInnerHTML={{ __html: html }} />
      ) : (
        <pre className="cs-mono m-0 whitespace-pre px-3 pt-2.5 text-[11px] leading-[1.5]">
          <Highlighted text={code} terms={terms} />
        </pre>
      )}
    </div>
  );
}

export function metaLine(item: Item, t: ReturnType<typeof useT>): string {
  switch (item.kind) {
    case "image":
      return `${item.width ?? "?"}×${item.height ?? "?"} · ${bytes(item.byteSize)}`;
    case "file":
      return item.byteSize ? bytes(item.byteSize) : t("files", { n: item.filePaths.length });
    case "color":
      return t("kind_color");
    case "link":
      return t("kind_link");
    default: {
      const lines = item.preview.split("\n").length;
      return lines > 1 && item.kind === "code" ? t("lines", { n: lines }) : t("chars", { n: item.charCount });
    }
  }
}
