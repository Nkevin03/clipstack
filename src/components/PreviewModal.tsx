import { useEffect, useMemo, useState } from "react";
import { fileSrc, ipc, type ItemDetail } from "../lib/ipc";
import { basename, bytes, colorFormats, domain, parseColor, timeAgo } from "../lib/format";
import { highlight, languageLabel } from "../lib/highlight";
import { sanitizeHtml } from "../lib/sanitize";
import { useLang, useT } from "../lib/i18n";
import { ModalShell } from "./Modal";
import { AppBadge } from "./Card/Card";
import { useItemActions } from "../hooks/useItemActions";

export function PreviewContent({ detail, large = true }: { detail: ItemDetail; large?: boolean }) {
  const t = useT();
  const { item, text, html } = detail;
  const actions = useItemActions();
  const code = useMemo(() => (item.kind === "code" && text ? highlight(text, item.language) : null), [item, text]);
  const rich = useMemo(() => (item.kind === "rich" && html ? sanitizeHtml(html) : null), [item.kind, html]);
  switch (item.kind) {
    case "image": {
      const src = fileSrc(item.imagePath);
      return (
        <div className="flex min-h-0 flex-1 flex-col">
          <div className="cs-checker flex min-h-0 flex-1 items-center justify-center overflow-auto">
            {src && <img src={src} alt="" className="max-h-full max-w-full object-contain" draggable={false} />}
          </div>
          {detail.ocrText && (
            <div className="cs-scroll max-h-28 overflow-auto px-4 py-2 text-[12px]" style={{ borderTop: "1px solid var(--cs-border)" }}>
              <div className="cs-caps mb-1">{t("ocrText")}</div>
              <div className="select-text whitespace-pre-wrap">{detail.ocrText}</div>
            </div>
          )}
        </div>
      );
    }
    case "code":
      return (
        <div className="cs-scroll min-h-0 flex-1 overflow-auto" style={{ background: "var(--cs-code-bg)", color: "var(--cs-code-text)" }}>
          <div className="px-4 pt-3 text-[10px] font-semibold uppercase tracking-wide opacity-60">{languageLabel(item.language)}</div>
          <pre className="cs-mono m-0 select-text whitespace-pre px-4 pb-4 pt-1 text-[12px] leading-[1.55]" dangerouslySetInnerHTML={{ __html: code ?? "" }} />
        </div>
      );
    case "color": {
      const c = parseColor(item.preview);
      const f = c ? colorFormats(c) : null;
      return (
        <div className="flex min-h-0 flex-1 flex-col">
          <div className="cs-checker min-h-[160px] flex-1">
            <div className="h-full w-full" style={{ background: item.preview.trim() }} />
          </div>
          {f && (
            <div className="grid grid-cols-3 gap-2 p-3">
              {Object.entries(f).map(([k, v]) => (
                <button key={k} onClick={() => navigator.clipboard?.writeText(v)} className="cs-mono rounded-[8px] px-2 py-1.5 text-left text-[11.5px] hover:bg-[var(--cs-surface-hover)]" style={{ background: "var(--cs-field)" }}>
                  <div className="cs-caps !text-[9px]">{k}</div>
                  <div className="select-text truncate">{v}</div>
                </button>
              ))}
            </div>
          )}
        </div>
      );
    }
    case "link":
      return (
        <div className="flex flex-1 flex-col gap-3 p-5">
          <div className="text-[17px] font-semibold">{item.linkTitle ?? domain(item.preview)}</div>
          <button onClick={() => actions.openLink(item)} className="select-text break-all text-left text-[13px] underline" style={{ color: "var(--cs-accent)" }}>
            {item.preview}
          </button>
        </div>
      );
    case "file":
      return (
        <div className="cs-scroll flex-1 overflow-auto p-4">
          {item.filePaths.map((p) => (
            <div key={p} className="flex items-center gap-2 py-1 text-[12.5px]">
              <span className="font-medium">{basename(p)}</span>
              <span className="truncate select-text" style={{ color: "var(--cs-text-faint)" }}>
                {p}
              </span>
            </div>
          ))}
        </div>
      );
    default:
      return rich && large ? (
        <div className="cs-scroll cs-rich min-h-0 flex-1 select-text overflow-auto p-4 !text-[13px]" dangerouslySetInnerHTML={{ __html: rich }} />
      ) : (
        <div className="cs-scroll min-h-0 flex-1 select-text overflow-auto whitespace-pre-wrap break-words p-4 text-[13px] leading-relaxed">{text}</div>
      );
  }
}

export function PreviewModal({ id }: { id: number }) {
  const t = useT();
  const lang = useLang();
  const [detail, setDetail] = useState<ItemDetail | null>(null);
  useEffect(() => {
    ipc.getItem(id).then(setDetail);
  }, [id]);
  if (!detail) return null;
  const { item } = detail;
  return (
    <ModalShell label={t("preview")} width="min(860px, 92%)">
      <div className="flex h-11 shrink-0 items-center gap-2 px-4" style={{ borderBottom: "1px solid var(--cs-border)" }}>
        <AppBadge item={item} size={18} />
        <span className="text-[12.5px] font-semibold">{item.sourceAppName ?? t("unknownApp")}</span>
        <span className="text-[11.5px]" style={{ color: "var(--cs-text-faint)" }}>
          · {timeAgo(item.lastUsedAt, lang)}
          {item.kind === "image" && ` · ${t("dimensions", { w: item.width ?? 0, h: item.height ?? 0 })} · ${bytes(item.byteSize)}`}
          {item.kind !== "image" && item.kind !== "file" && ` · ${t("chars", { n: item.charCount })}`}
        </span>
        <span className="flex-1" />
        <span className="text-[11px]" style={{ color: "var(--cs-text-faint)" }}>
          ↩ {t("paste")} · esc
        </span>
      </div>
      <div className="flex min-h-0 flex-col" style={{ height: "min(70vh, 520px)" }}>
        <PreviewContent detail={detail} />
      </div>
    </ModalShell>
  );
}
