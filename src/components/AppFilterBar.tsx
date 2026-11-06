import { useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Check, ChevronDown, Code2, File, Image, Link2, ListFilter, Palette, Settings, Type, TextQuote } from "lucide-react";
import { fileSrc, ipc, type Kind, type Sort } from "../lib/ipc";
import { useItems } from "../store/items";
import { useT } from "../lib/i18n";

export const KIND_ICONS: Record<Kind, typeof Type> = { text: Type, rich: TextQuote, link: Link2, image: Image, file: File, color: Palette, code: Code2 };
const KINDS: Kind[] = ["text", "rich", "link", "image", "file", "color", "code"];

export function AppFilterBar() {
  const t = useT();
  const { sourceApps, apps, toggleApp } = useItems();
  return (
    <div role="toolbar" aria-label={t("filterByApp")} className="flex shrink-0 items-center gap-1">
      {sourceApps.slice(0, 8).map((a) => {
        const on = apps.includes(a.bundleId);
        return (
          <button
            key={a.bundleId}
            title={a.name}
            aria-label={a.name}
            aria-pressed={on}
            onClick={() => toggleApp(a.bundleId)}
            className="flex h-8 w-8 items-center justify-center rounded-[8px] transition-all"
            style={{
              background: on ? "var(--cs-accent-soft)" : "transparent",
              boxShadow: on ? "inset 0 0 0 1.5px var(--cs-accent)" : undefined,
              opacity: apps.length && !on ? 0.45 : 1,
            }}
          >
            {a.iconPath ? (
              <img src={fileSrc(a.iconPath)} alt="" className="h-[22px] w-[22px]" draggable={false} />
            ) : (
              <span className="flex h-5 w-5 items-center justify-center rounded text-[10px] font-bold" style={{ background: a.color ?? "var(--cs-field)" }}>
                {a.name[0]}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}

export function TypeMenu() {
  const t = useT();
  const { kinds, toggleKind, setKinds, sort, setSort } = useItems();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => ref.current && !ref.current.contains(e.target as Node) && setOpen(false);
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, [open]);
  const label = kinds.length === 0 ? t("allTypes") : kinds.length === 1 ? t(`kind_${kinds[0]}`) : `${kinds.length} ${t("allTypes").toLowerCase().split(" ").pop()}`;
  const Icon = kinds.length === 1 ? KIND_ICONS[kinds[0]] : ListFilter;
  const sorts: [Sort, string][] = [["recent", t("sortRecent")], ["mostUsed", t("sortUsed")], ["pinnedFirst", t("sortPinned")]];
  return (
    <div ref={ref} className="relative shrink-0">
      <button
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
        className="flex h-8 items-center gap-1.5 rounded-[8px] px-2.5 text-[12px] font-medium transition-colors hover:bg-[var(--cs-surface-hover)]"
        style={{ background: kinds.length ? "var(--cs-accent-soft)" : "var(--cs-field)" }}
      >
        <Icon size={14} strokeWidth={2} />
        <span className="max-w-[110px] truncate">{label}</span>
        <ChevronDown size={13} strokeWidth={2.2} />
      </button>
      <AnimatePresence>
        {open && (
          <motion.div
            role="menu"
            initial={{ opacity: 0, y: -4, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -4, scale: 0.98 }}
            transition={{ duration: 0.12 }}
            className="absolute right-0 top-10 z-40 w-52 rounded-[12px] p-1.5"
            style={{ background: "var(--cs-surface)", boxShadow: "var(--cs-shadow-pop)", backdropFilter: "blur(20px)" }}
          >
            <MenuRow checked={kinds.length === 0} onClick={() => setKinds([])} label={t("allTypes")} />
            {KINDS.map((k) => {
              const KI = KIND_ICONS[k];
              return <MenuRow key={k} checked={kinds.includes(k)} onClick={() => toggleKind(k)} label={t(`kind_${k}`)} icon={<KI size={13} />} />;
            })}
            <div className="my-1 h-px" style={{ background: "var(--cs-border)" }} />
            {sorts.map(([s, l]) => (
              <MenuRow key={s} checked={sort === s} onClick={() => setSort(s)} label={l} />
            ))}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function MenuRow({ checked, onClick, label, icon }: { checked: boolean; onClick: () => void; label: string; icon?: React.ReactNode }) {
  return (
    <button role="menuitemcheckbox" aria-checked={checked} onClick={onClick} className="flex h-7 w-full items-center gap-2 rounded-[7px] px-2 text-left text-[12.5px] hover:bg-[var(--cs-surface-hover)]">
      <span className="w-3.5">{checked && <Check size={13} strokeWidth={2.6} style={{ color: "var(--cs-accent)" }} />}</span>
      {icon && <span style={{ color: "var(--cs-text-muted)" }}>{icon}</span>}
      <span className="flex-1">{label}</span>
    </button>
  );
}

export function SettingsButton() {
  const t = useT();
  return (
    <button aria-label={t("settings")} title={`${t("settings")} (⌘,)`} onClick={() => ipc.openSettings()} className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[8px] transition-colors hover:bg-[var(--cs-surface-hover)]" style={{ color: "var(--cs-text-muted)" }}>
      <Settings size={16} strokeWidth={2} />
    </button>
  );
}
