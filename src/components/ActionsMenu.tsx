import { useEffect, useMemo, useState } from "react";
import { CornerDownLeft, Save } from "lucide-react";
import { ipc, type Action, type ActionResult } from "../lib/ipc";
import { useUi } from "../store/ui";
import { useT, type StringKey } from "../lib/i18n";
import { ModalShell, useListKeys } from "./Modal";
import { useItemActions } from "../hooks/useItemActions";

const ACTIONS: Action[] = ["upper", "lower", "title", "stripFormatting", "trim", "dedupLines", "sortLines", "jsonPretty", "jsonMinify", "urlEncode", "urlDecode", "base64Encode", "base64Decode", "count"];

export function ActionsMenu({ id }: { id: number }) {
  const t = useT();
  const close = useUi((s) => s.closeModal);
  const toast = useUi((s) => s.toast);
  const { fail } = useItemActions();
  const [q, setQ] = useState("");
  const [index, setIndex] = useState(0);
  const [preview, setPreview] = useState<ActionResult | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const list = useMemo(() => ACTIONS.filter((a) => t(`act_${a}` as StringKey).toLowerCase().includes(q.toLowerCase())), [q, t]);
  const current = list[index];

  useEffect(() => {
    if (!current) return;
    let alive = true;
    ipc
      .runAction(id, current, "preview")
      .then((r) => {
        if (alive) {
          setPreview(r);
          setPreviewError(null);
        }
      })
      .catch((e) => {
        if (alive) {
          setPreview(null);
          setPreviewError(String(e));
        }
      });
    return () => {
      alive = false;
    };
  }, [id, current]);

  const run = async (i: number, save: boolean) => {
    const a = list[i];
    if (!a) return;
    if (a === "count") return; // stats are shown live in the preview
    try {
      await ipc.runAction(id, a, save ? "save" : "paste");
      close();
      if (save) toast({ text: t("savedAsNew") });
    } catch (e) {
      fail(e);
    }
  };
  const onKeyDown = useListKeys(list.length, index, setIndex, (i, e) => void run(i, e.metaKey));

  return (
    <ModalShell label={t("actions")} width="min(680px, 94%)">
      <input
        autoFocus
        value={q}
        onChange={(e) => {
          setQ(e.target.value);
          setIndex(0);
        }}
        onKeyDown={onKeyDown}
        placeholder={t("searchActions")}
        className="h-11 shrink-0 bg-transparent px-4 text-[13px] outline-none"
        style={{ borderBottom: "1px solid var(--cs-border)" }}
      />
      <div className="flex min-h-0" style={{ height: 300 }}>
        <div role="listbox" className="cs-scroll w-56 shrink-0 overflow-auto p-1.5" style={{ borderRight: "1px solid var(--cs-border)" }}>
          {list.map((a, i) => (
            <button key={a} role="option" aria-selected={i === index} onMouseEnter={() => setIndex(i)} onClick={() => run(i, false)} className="flex h-8 w-full items-center rounded-[7px] px-2 text-left text-[12.5px]" style={{ background: i === index ? "var(--cs-surface-active)" : undefined }}>
              {t(`act_${a}` as StringKey)}
            </button>
          ))}
        </div>
        <div className="flex min-w-0 flex-1 flex-col">
          <div className="cs-scroll cs-mono min-h-0 flex-1 select-text overflow-auto whitespace-pre-wrap break-all p-3 text-[11.5px]" style={{ color: previewError ? "var(--cs-danger)" : undefined }}>
            {previewError ?? preview?.text.slice(0, 5000)}
          </div>
          {preview && (
            <div className="flex items-center gap-2 px-3 py-2 text-[11px]" style={{ borderTop: "1px solid var(--cs-border)", color: "var(--cs-text-muted)" }}>
              <span className="flex-1">{t("statsLine", { chars: preview.stats.chars, words: preview.stats.words, lines: preview.stats.lines })}</span>
              {current !== "count" && !previewError && (
                <>
                  <button onClick={() => run(index, true)} className="flex items-center gap-1 rounded-[6px] px-2 py-1" style={{ background: "var(--cs-field)" }}>
                    <Save size={12} /> {t("resultSave")} <span className="opacity-60">⌘↩</span>
                  </button>
                  <button onClick={() => run(index, false)} className="flex items-center gap-1 rounded-[6px] px-2 py-1 font-semibold" style={{ background: "var(--cs-accent)", color: "var(--cs-accent-text)" }}>
                    <CornerDownLeft size={12} /> {t("resultPaste")}
                  </button>
                </>
              )}
            </div>
          )}
        </div>
      </div>
    </ModalShell>
  );
}
