import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import { CheckCircle2, Layers, ShieldAlert, X } from "lucide-react";
import { fileSrc, ipc, type Item } from "../../lib/ipc";
import type { SequenceState } from "../../lib/bindings/SequenceState";
import { basename, parseColor } from "../../lib/format";
import { useT } from "../../lib/i18n";
import { useSettings } from "../../store/settings";
import { useTauriEvent } from "../../hooks/useTauriEvent";
import { prettyShortcut } from "../Settings/ShortcutRecorder";
import { KIND_ICONS } from "../AppFilterBar";

const MAX_ROWS = 7;

function label(item: Item, t: ReturnType<typeof useT>): string {
  switch (item.kind) {
    case "image":
      return `${t("kind_image")} ${item.width ?? "?"}×${item.height ?? "?"}`;
    case "file":
      return item.filePaths.length > 1 ? `${basename(item.filePaths[0])} +${item.filePaths.length - 1}` : basename(item.filePaths[0] ?? "");
    default:
      return item.preview.replace(/\s+/g, " ").trim();
  }
}

function Thumb({ item }: { item: Item }) {
  if (item.kind === "image" && (item.thumbPath || item.imagePath)) {
    return <img src={fileSrc(item.thumbPath ?? item.imagePath)} alt="" className="h-7 w-7 shrink-0 rounded-[6px] object-cover" draggable={false} />;
  }
  if (item.kind === "color" && parseColor(item.preview)) {
    return <span className="h-7 w-7 shrink-0 rounded-[6px]" style={{ background: item.preview.trim(), boxShadow: "inset 0 0 0 0.5px var(--cs-border-strong)" }} />;
  }
  const Icon = KIND_ICONS[item.kind];
  return (
    <span className="relative flex h-7 w-7 shrink-0 items-center justify-center rounded-[6px]" style={{ background: "var(--cs-field)" }}>
      {item.sourceIconPath ? <img src={fileSrc(item.sourceIconPath)} alt="" className="h-[18px] w-[18px]" draggable={false} /> : <Icon size={14} />}
    </span>
  );
}

export function Hud() {
  const t = useT();
  const reduced = useReducedMotion();
  const shortcut = useSettings((s) => s.settings?.sequenceShortcut ?? null);
  const [st, setSt] = useState<SequenceState | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    invoke<SequenceState>("sequence_state").then(setSt);
  }, []);
  useTauriEvent<SequenceState>("sequence:state", setSt);

  // Size the native panel to the content.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(() => void invoke("resize_hud", { height: Math.ceil(el.getBoundingClientRect().height) }));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const items = st?.items ?? [];
  const shown = items.slice(0, MAX_ROWS);
  const more = items.length - shown.length;
  const finished = !!st?.finished;

  return (
    <div
      ref={ref}
      role="status"
      aria-live="polite"
      className="flex w-full flex-col overflow-hidden"
      style={{ borderRadius: 14, background: "var(--cs-panel-tint)", boxShadow: "inset 0 0 0 1px var(--cs-panel-border)" }}
    >
      <div className="flex h-10 shrink-0 items-center gap-2 px-3">
        <span className="flex h-6 w-6 items-center justify-center rounded-[7px]" style={{ background: "var(--cs-accent)", color: "var(--cs-accent-text)" }}>
          <Layers size={14} strokeWidth={2.2} />
        </span>
        <span className="text-[12.5px] font-semibold">{t("seqTitle")}</span>
        {items.length > 0 && (
          <span className="rounded-full px-1.5 text-[10.5px] font-bold tabular-nums" style={{ background: "var(--cs-accent-soft)", color: "var(--cs-accent)" }}>
            {items.length}
          </span>
        )}
        <span className="flex-1" />
        <button aria-label={t("seqStop")} title={t("seqStop")} onClick={() => invoke("stop_sequence")} className="flex h-6 w-6 items-center justify-center rounded-full transition-colors hover:bg-[var(--cs-surface-hover)]" style={{ color: "var(--cs-text-muted)" }}>
          <X size={14} strokeWidth={2.4} />
        </button>
      </div>

      {st && !st.accessibility && (
        <div className="mx-2 mb-2 flex items-start gap-2 rounded-[9px] px-2.5 py-2 text-[11.5px] leading-snug" style={{ background: "color-mix(in srgb, #f59e0b 18%, transparent)" }}>
          <ShieldAlert size={14} className="mt-[1px] shrink-0" style={{ color: "#d97706" }} />
          <span className="flex-1">{t("seqNeedsAx")}</span>
          <button onClick={() => ipc.requestAccessibility()} className="shrink-0 rounded-[6px] px-1.5 py-0.5 font-semibold" style={{ background: "rgb(0 0 0 / 0.08)" }}>
            {t("enable")}
          </button>
        </div>
      )}

      <div className="flex flex-col gap-1 px-2 pb-2">
        <AnimatePresence initial={false} mode="popLayout">
          {finished ? (
            <motion.div key="done" initial={{ opacity: 0, scale: 0.96 }} animate={{ opacity: 1, scale: 1 }} className="flex h-12 items-center justify-center gap-2 text-[12.5px] font-semibold" style={{ color: "#16a34a" }}>
              <CheckCircle2 size={16} /> {t("seqDone")}
            </motion.div>
          ) : items.length === 0 ? (
            <motion.div key="empty" initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="flex flex-col items-center gap-0.5 px-3 py-3 text-center">
              <div className="text-[12.5px] font-medium">{t("seqEmpty")}</div>
              <div className="text-[11px]" style={{ color: "var(--cs-text-faint)" }}>{t("seqEmptyHint")}</div>
            </motion.div>
          ) : (
            shown.map((item, i) => (
              <motion.div
                key={item.id}
                layout={!reduced}
                initial={{ opacity: 0, y: -10, scale: 0.98 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                exit={{ opacity: 0, x: -24, transition: { duration: 0.16 } }}
                transition={{ type: "spring", stiffness: 520, damping: 38 }}
                className="flex h-10 items-center gap-2 rounded-[9px] px-2"
                style={{
                  background: i === 0 ? "var(--cs-accent-soft)" : "var(--cs-surface-2)",
                  boxShadow: i === 0 ? "inset 0 0 0 1.5px var(--cs-accent)" : "inset 0 0 0 0.5px var(--cs-border)",
                  opacity: i === 0 ? 1 : Math.max(0.55, 1 - i * 0.08),
                }}
              >
                <Thumb item={item} />
                <span className={`min-w-0 flex-1 truncate text-[12px] ${item.kind === "code" ? "cs-mono text-[11px]" : ""}`}>{label(item, t)}</span>
                {i === 0 ? (
                  <span className="shrink-0 rounded-[5px] px-1.5 py-[1px] text-[10px] font-bold" style={{ background: "var(--cs-accent)", color: "var(--cs-accent-text)" }}>
                    ⌘V
                  </span>
                ) : (
                  <span className="w-5 shrink-0 text-center text-[10.5px] font-semibold tabular-nums" style={{ color: "var(--cs-text-faint)" }}>
                    {i + 1}
                  </span>
                )}
              </motion.div>
            ))
          )}
        </AnimatePresence>
        {more > 0 && !finished && (
          <div className="px-2 pt-0.5 text-[11px]" style={{ color: "var(--cs-text-faint)" }}>
            {t("seqMore", { n: more })}
          </div>
        )}
      </div>

      {!finished && (
        <div className="flex h-7 shrink-0 items-center justify-between gap-2 whitespace-nowrap px-3 text-[10.5px]" style={{ borderTop: "1px solid var(--cs-border)", color: "var(--cs-text-faint)" }}>
          <span className="truncate">{t("seqFooter")}</span>
          {shortcut && <span className="shrink-0 font-semibold">{prettyShortcut(shortcut)} {t("seqToEnd")}</span>}
        </div>
      )}
    </div>
  );
}
