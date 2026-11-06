import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import { useItems } from "../store/items";
import { useUi } from "../store/ui";
import { useT } from "../lib/i18n";
import { useItemActions } from "../hooks/useItemActions";

type Entry = { label: string; hint?: string; run: () => void; danger?: boolean } | "sep";

export function ContextMenu() {
  const t = useT();
  const menu = useUi((s) => s.contextMenu);
  const setMenu = useUi((s) => s.setContextMenu);
  const openModal = useUi((s) => s.openModal);
  const item = useItems((s) => s.items.find((i) => i.id === menu?.id));
  const ids = useItems((s) => s.selectedIds)();
  const a = useItemActions();
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ x: number; y: number } | null>(null);

  useLayoutEffect(() => {
    if (!menu || !ref.current) return;
    const r = ref.current.getBoundingClientRect();
    setPos({ x: Math.min(menu.x, window.innerWidth - r.width - 8), y: Math.min(menu.y, window.innerHeight - r.height - 8) });
  }, [menu]);
  useEffect(() => {
    if (!menu) return;
    const close = (e: MouseEvent) => ref.current && !ref.current.contains(e.target as Node) && setMenu(null);
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, [menu, setMenu]);

  if (!menu || !item) return null;
  const textual = item.kind !== "image" && item.kind !== "file";
  const sel = ids.includes(item.id) ? ids : [item.id];
  const entries: Entry[] = [
    { label: sel.length > 1 ? t("pasteAll") : t("paste"), hint: "↩", run: () => a.paste(false, sel) },
    ...(textual ? [{ label: t("pastePlain"), hint: "⇧↩", run: () => a.paste(true, sel) }] : []),
    { label: t("copyOnly"), hint: "⌘C", run: () => a.copyOnly(sel) },
    "sep",
    { label: t("preview"), hint: "␣", run: () => openModal({ type: "preview", id: item.id }) },
    ...(textual ? [{ label: t("edit"), hint: "⌘E", run: () => openModal({ type: "edit", id: item.id }) }, { label: t("transform"), hint: "⌘K", run: () => openModal({ type: "actions", id: item.id }) }] : []),
    ...(item.kind === "link" ? [{ label: t("openLink"), run: () => a.openLink(item) }] : []),
    ...(item.kind === "file" || item.kind === "image" ? [{ label: t("revealInFinder"), run: () => a.reveal(item) }] : []),
    "sep",
    { label: item.pinned ? t("unpin") : t("pin"), hint: "⌘D", run: () => a.togglePin(sel) },
    { label: t("tagsAction"), hint: "⌘T", run: () => openModal({ type: "tags", id: item.id }) },
    { label: t("moveToSpace"), hint: "⌘M", run: () => openModal({ type: "space", ids: sel }) },
    ...(item.spaceId !== null ? [{ label: t("removeFromSpace"), run: () => a.moveTo(sel, null) }] : []),
    "sep",
    { label: t("delete"), hint: "⌘⌫", run: () => a.remove(sel), danger: true },
  ];
  return (
    <motion.div
      ref={ref}
      role="menu"
      initial={{ opacity: 0, scale: 0.97 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.08 }}
      className="fixed z-[60] w-[264px] rounded-[10px] p-1"
      style={{ left: (pos ?? menu).x, top: (pos ?? menu).y, background: "var(--cs-surface)", boxShadow: "var(--cs-shadow-pop)", backdropFilter: "blur(30px)" }}
    >
      {entries.map((e, i) =>
        e === "sep" ? (
          <div key={i} className="mx-2 my-1 h-px" style={{ background: "var(--cs-border)" }} />
        ) : (
          <button
            key={i}
            role="menuitem"
            onClick={() => {
              setMenu(null);
              e.run();
            }}
            className="flex h-[26px] w-full items-center rounded-[6px] px-2.5 text-left text-[12.5px] hover:bg-[var(--cs-accent)] hover:text-white"
            style={{ color: e.danger ? "var(--cs-danger)" : undefined }}
          >
            <span className="flex-1 truncate whitespace-nowrap">{e.label}</span>
            {e.hint && <span className="text-[11px] opacity-50">{e.hint}</span>}
          </button>
        ),
      )}
    </motion.div>
  );
}
