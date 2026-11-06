import { useState } from "react";
import { Reorder } from "framer-motion";
import { Clock, Pin, Plus, Hash } from "lucide-react";
import { ipc, type Space } from "../lib/ipc";
import { useItems, type SpaceSel } from "../store/items";
import { useUi } from "../store/ui";
import { useT } from "../lib/i18n";
import { useItemActions } from "../hooks/useItemActions";
import { SpaceIcon } from "./SpaceIcon";

function Row({ active, onClick, icon, label, count, color, onDropItem, onContext }: {
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
  count?: number;
  color?: string | null;
  onDropItem?: (id: number) => void;
  onContext?: (e: React.MouseEvent) => void;
}) {
  const [over, setOver] = useState(false);
  return (
    <button
      onClick={onClick}
      onContextMenu={onContext}
      onDragOver={(e) => {
        if (onDropItem && e.dataTransfer.types.includes("application/x-clipstack-item")) {
          e.preventDefault();
          setOver(true);
        }
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(e) => {
        setOver(false);
        const id = Number(e.dataTransfer.getData("application/x-clipstack-item"));
        if (onDropItem && id) onDropItem(id);
      }}
      aria-current={active}
      className="flex h-7 w-full items-center gap-2 rounded-[7px] px-2 text-left text-[12.5px] transition-colors"
      style={{
        background: over ? "var(--cs-accent-soft)" : active ? "var(--cs-surface-active)" : "transparent",
        boxShadow: over ? "inset 0 0 0 1.5px var(--cs-accent)" : undefined,
        fontWeight: active ? 600 : 450,
      }}
    >
      <span className="flex w-4 shrink-0 justify-center" style={{ color: color ?? "var(--cs-text-muted)" }}>
        {icon}
      </span>
      <span className="min-w-0 flex-1 truncate">{label}</span>
      {count !== undefined && <span className="text-[11px] tabular-nums" style={{ color: "var(--cs-text-faint)" }}>{count}</span>}
    </button>
  );
}

const same = (a: SpaceSel, b: SpaceSel) => a.type === b.type && (a.type !== "space" || (b.type === "space" && a.id === b.id));

export function Sidebar() {
  const t = useT();
  const { space, spaces, tags, counts, tagFilter, selectSpace, toggleTag } = useItems();
  const openModal = useUi((s) => s.openModal);
  const actions = useItemActions();
  const [order, setOrder] = useState<Space[] | null>(null);
  const list = order ?? spaces;

  const spaceMenu = (s: Space) => (e: React.MouseEvent) => {
    e.preventDefault();
    openModal({ type: "space-edit", space: s });
  };

  return (
    <nav aria-label={t("spaces")} className="cs-scroll flex h-full w-[184px] shrink-0 flex-col gap-0.5 overflow-y-auto py-2 pl-2 pr-1.5">
      <Row active={same(space, { type: "all" })} onClick={() => selectSpace({ type: "all" })} icon={<Clock size={14} strokeWidth={2} />} label={t("history")} count={counts.all} onDropItem={(id) => actions.moveTo([id], null)} />
      <Row
        active={same(space, { type: "pinned" })}
        onClick={() => selectSpace({ type: "pinned" })}
        icon={<Pin size={14} strokeWidth={2} />}
        label={t("pinned")}
        count={counts.pinned}
        onDropItem={(id) => ipc.pinItem([id], true)}
      />
      <div className="cs-caps mt-3 mb-1 px-2">{t("spaces")}</div>
      <Reorder.Group
        axis="y"
        values={list}
        onReorder={setOrder}
        className="flex flex-col gap-0.5"
        as="div"
      >
        {list.map((s) => (
          <Reorder.Item
            key={s.id}
            value={s}
            as="div"
            onDragEnd={() => {
              if (order) void ipc.reorderSpaces(order.map((x) => x.id)).then(() => setOrder(null));
            }}
          >
            <Row
              active={same(space, { type: "space", id: s.id })}
              onClick={() => selectSpace({ type: "space", id: s.id })}
              icon={<SpaceIcon name={s.icon} size={14} />}
              color={s.color}
              label={s.name}
              count={s.count}
              onDropItem={(id) => actions.moveTo([id], s.id)}
              onContext={spaceMenu(s)}
            />
          </Reorder.Item>
        ))}
      </Reorder.Group>
      <button
        onClick={() => openModal({ type: "space-edit" })}
        className="mt-0.5 flex h-7 items-center gap-2 rounded-[7px] px-2 text-[12.5px] transition-colors hover:bg-[var(--cs-surface-hover)]"
        style={{ color: "var(--cs-text-muted)" }}
      >
        <Plus size={14} strokeWidth={2} className="w-4" />
        {t("newSpace")}
      </button>
      {tags.length > 0 && (
        <>
          <div className="cs-caps mt-3 mb-1 px-2">{t("tags")}</div>
          {tags.map((tg) => (
            <Row
              key={tg.id}
              active={tagFilter.includes(tg.name)}
              onClick={() => toggleTag(tg.name)}
              icon={tg.color ? <span className="h-2 w-2 rounded-full" style={{ background: tg.color }} /> : <Hash size={13} />}
              label={tg.name}
              count={tg.count}
            />
          ))}
        </>
      )}
    </nav>
  );
}
