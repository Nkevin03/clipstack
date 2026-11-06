import { useEffect, useState } from "react";
import { ipc } from "../lib/ipc";
import { useUi } from "../store/ui";
import { useT } from "../lib/i18n";
import { ModalShell } from "./Modal";
import { useItemActions } from "../hooks/useItemActions";

export function EditModal({ id }: { id: number }) {
  const t = useT();
  const close = useUi((s) => s.closeModal);
  const toast = useUi((s) => s.toast);
  const { fail } = useItemActions();
  const [text, setText] = useState<string | null>(null);
  useEffect(() => {
    ipc.getItem(id).then((d) => setText(d?.text ?? ""));
  }, [id]);
  const save = async () => {
    if (text === null) return;
    try {
      await ipc.updateItemText(id, text);
      toast({ text: t("saved") });
      close();
    } catch (e) {
      fail(e);
    }
  };
  if (text === null) return null;
  return (
    <ModalShell label={t("editTitle")} width="min(720px, 94%)">
      <div className="flex h-10 items-center px-4 text-[12.5px] font-semibold" style={{ borderBottom: "1px solid var(--cs-border)" }}>
        {t("editTitle")}
      </div>
      <textarea
        autoFocus
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && e.metaKey) {
            e.preventDefault();
            void save();
          }
        }}
        spellCheck={false}
        className="cs-scroll cs-mono min-h-0 resize-none bg-transparent p-4 text-[12.5px] leading-relaxed outline-none"
        style={{ height: "min(50vh, 340px)" }}
      />
      <div className="flex justify-end gap-2 px-4 py-3" style={{ borderTop: "1px solid var(--cs-border)" }}>
        <button onClick={close} className="h-8 rounded-[8px] px-3 text-[12.5px]" style={{ background: "var(--cs-field)" }}>
          {t("cancel")}
        </button>
        <button onClick={save} className="h-8 rounded-[8px] px-3 text-[12.5px] font-semibold" style={{ background: "var(--cs-accent)", color: "var(--cs-accent-text)" }}>
          {t("save")} <span className="opacity-70">⌘↩</span>
        </button>
      </div>
    </ModalShell>
  );
}
