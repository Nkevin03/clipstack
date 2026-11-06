import { memo } from "react";
import { motion } from "framer-motion";
import { Pin, Timer } from "lucide-react";
import { fileSrc, type Item, type Tag } from "../../lib/ipc";
import { readableOn, timeAgo } from "../../lib/format";
import { useLang, useT } from "../../lib/i18n";
import { CodeBody, ColorBody, FileBody, ImageBody, LinkBody, RichBody, TextBody, metaLine } from "./bodies";

export type CardProps = {
  item: Item;
  index: number;
  selected: boolean;
  inMulti: boolean;
  terms: string[];
  tags: Tag[];
  compact?: boolean;
  animateIn?: boolean;
  onSelect: (index: number, e: React.MouseEvent) => void;
  onActivate: (index: number) => void;
  onContext: (index: number, e: React.MouseEvent) => void;
};

export function AppBadge({ item, size = 20 }: { item: Item; size?: number }) {
  const src = fileSrc(item.sourceIconPath);
  if (src) return <img src={src} alt="" width={size} height={size} draggable={false} className="shrink-0" />;
  const letter = (item.sourceAppName ?? "?").slice(0, 1).toUpperCase();
  return (
    <span className="flex shrink-0 items-center justify-center rounded-[5px] text-[10px] font-bold" style={{ width: size, height: size, background: "rgb(255 255 255 / 0.25)" }}>
      {letter}
    </span>
  );
}

function Body({ item, terms }: { item: Item; terms: string[] }) {
  switch (item.kind) {
    case "rich":
      return <RichBody item={item} terms={terms} />;
    case "link":
      return <LinkBody item={item} terms={terms} />;
    case "image":
      return <ImageBody item={item} terms={terms} />;
    case "file":
      return <FileBody item={item} terms={terms} />;
    case "color":
      return <ColorBody item={item} terms={terms} />;
    case "code":
      return <CodeBody item={item} terms={terms} />;
    default:
      return <TextBody item={item} terms={terms} />;
  }
}

function CardImpl({ item, index, selected, inMulti, terms, tags, animateIn, onSelect, onActivate, onContext }: CardProps) {
  const t = useT();
  const lang = useLang();
  const header = item.sourceColor ?? "#8E8E93";
  const fg = readableOn(header) === "dark" ? "#16161a" : "#ffffff";
  const itemTags = tags.filter((tg) => item.tagIds.includes(tg.id));
  const expiresMin = item.expiresAt ? Math.max(1, Math.ceil((item.expiresAt - Date.now()) / 60000)) : null;

  return (
    <div
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData("application/x-clipstack-item", String(item.id));
        e.dataTransfer.effectAllowed = "move";
      }}
      onMouseDown={(e) => {
        if (e.button === 0) onSelect(index, e);
      }}
      onDoubleClick={() => onActivate(index)}
      onContextMenu={(e) => {
        e.preventDefault();
        onContext(index, e);
      }}
    >
    <motion.div
      role="option"
      aria-selected={selected || inMulti}
      aria-label={`${t(`kind_${item.kind}`)} — ${item.sourceAppName ?? t("unknownApp")}, ${timeAgo(item.lastUsedAt, lang)}`}
      initial={animateIn ? { opacity: 0, y: 10, scale: 0.97 } : false}
      animate={{ opacity: 1, y: 0, scale: selected ? 1.02 : 1 }}
      transition={{ type: "spring", stiffness: 420, damping: 32, delay: animateIn ? Math.min(index, 10) * 0.018 : 0 }}
      className="relative flex shrink-0 flex-col overflow-hidden"
      style={{
        width: "var(--cs-card-w)",
        height: "var(--cs-card-h)",
        borderRadius: "var(--cs-radius-card)",
        background: "var(--cs-surface)",
        boxShadow: selected || inMulti
          ? `0 0 0 2px var(--cs-accent), var(--cs-shadow-card)`
          : `0 0 0 0.5px var(--cs-border-strong), var(--cs-shadow-card)`,
      }}
    >
      <div className="flex shrink-0 items-center gap-2 px-2.5" style={{ height: "var(--cs-card-header)", background: header, color: fg }}>
        <AppBadge item={item} />
        <span className="min-w-0 flex-1 truncate text-[12px] font-semibold">{item.sourceAppName ?? t("unknownApp")}</span>
        {expiresMin !== null && (
          <span title={t("expiresIn", { n: expiresMin })} className="flex items-center gap-0.5 text-[10px] font-semibold opacity-90">
            <Timer size={11} strokeWidth={2.4} />
            {expiresMin}′
          </span>
        )}
        {item.pinned && <Pin size={13} strokeWidth={2.4} className="shrink-0 rotate-45" fill="currentColor" aria-label={t("pinned")} />}
      </div>
      <div className="relative min-h-0 flex-1">
        <Body item={item} terms={terms} />
        {item.kind !== "image" && item.kind !== "color" && (
          <div className="pointer-events-none absolute inset-x-0 bottom-0 h-7" style={{ background: item.kind === "code" ? "linear-gradient(transparent, var(--cs-code-bg))" : "linear-gradient(transparent, var(--cs-surface))" }} />
        )}
      </div>
      {itemTags.length > 0 && (
        <div className="flex shrink-0 items-center gap-1 overflow-hidden px-3 pt-1.5">
          {itemTags.slice(0, 4).map((tg) => (
            <span key={tg.id} className="flex min-w-0 items-center gap-1 rounded-full px-1.5 py-[1px] text-[10px] font-medium" style={{ background: "var(--cs-field)" }}>
              <span className="h-1.5 w-1.5 shrink-0 rounded-full" style={{ background: tg.color ?? "var(--cs-accent)" }} />
              <span className="truncate">{tg.name}</span>
            </span>
          ))}
        </div>
      )}
      <div className="flex h-8 shrink-0 items-center gap-2 px-3 text-[10.5px]" style={{ color: "var(--cs-text-faint)" }}>
        <span className="truncate">{timeAgo(item.lastUsedAt, lang)}</span>
        <span className="truncate opacity-80">· {metaLine(item, t)}</span>
        <span className="flex-1" />
        {index < 9 && (
          <span className="rounded-[5px] px-1.5 py-[1px] text-[10px] font-semibold" style={{ background: "var(--cs-field)", color: "var(--cs-text-muted)" }}>
            ⌘{index + 1}
          </span>
        )}
      </div>
    </motion.div>
    </div>
  );
}

export const Card = memo(CardImpl);
