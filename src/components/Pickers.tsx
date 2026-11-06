import { useEffect, useMemo, useRef, useState } from "react";
import { Check, Hash, Plus, X } from "lucide-react";
import { ipc } from "../lib/ipc";
import { useItems } from "../store/items";
import { useUi } from "../store/ui";
import { useT } from "../lib/i18n";
import { ModalShell, useListKeys } from "./Modal";
import { SPACE_COLORS, SPACE_ICONS, SpaceIcon } from "./SpaceIcon";
import { useItemActions } from "../hooks/useItemActions";

const TAG_COLORS = ["#EF4444", "#F59E0B", "#10B981", "#0EA5E9", "#6366F1", "#EC4899", "#8B5CF6", "#64748B"];

export function TagPicker({ id }: { id: number }) {
  const t = useT();
  const close = useUi((s) => s.closeModal);
  const tags = useItems((s) => s.tags);
  const item = useItems((s) => s.items.find((i) => i.id === id));
  const [selected, setSelected] = useState<number[]>(item?.tagIds ?? []);
  const [q, setQ] = useState("");
  const [index, setIndex] = useState(0);
  const filtered = useMemo(() => tags.filter((tg) => tg.name.toLowerCase().includes(q.trim().toLowerCase())), [tags, q]);
  const canCreate = q.trim() && !tags.some((tg) => tg.name.toLowerCase() === q.trim().toLowerCase());
  const rows = filtered.length + (canCreate ? 1 : 0);

  const commit = async (ids: number[]) => {
    setSelected(ids);
    await ipc.setItemTags(id, ids);
  };
  const toggle = (tagId: number) => commit(selected.includes(tagId) ? selected.filter((x) => x !== tagId) : [...selected, tagId]);
  const create = async () => {
    const color = TAG_COLORS[tags.length % TAG_COLORS.length];
    const newId = await ipc.createTag(q.trim(), color);
    setQ("");
    await commit([...selected, newId]);
  };
  const onKeyDown = useListKeys(rows, index, setIndex, (i) => {
    if (i < filtered.length) void toggle(filtered[i].id);
    else void create();
  });

  return (
    <ModalShell label={t("tagsAction")} width={340}>
      <input
        autoFocus
        value={q}
        onChange={(e) => {
          setQ(e.target.value);
          setIndex(0);
        }}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.stopPropagation();
            close();
          } else onKeyDown(e);
        }}
        placeholder={t("searchOrCreateTag")}
        className="h-11 shrink-0 bg-transparent px-4 text-[13px] outline-none"
        style={{ borderBottom: "1px solid var(--cs-border)" }}
      />
      <div role="listbox" className="cs-scroll max-h-64 overflow-auto p-1.5">
        {filtered.map((tg, i) => (
          <button
            key={tg.id}
            role="option"
            aria-selected={selected.includes(tg.id)}
            onMouseEnter={() => setIndex(i)}
            onClick={() => toggle(tg.id)}
            className="flex h-8 w-full items-center gap-2 rounded-[7px] px-2 text-left text-[12.5px]"
            style={{ background: index === i ? "var(--cs-surface-active)" : undefined }}
          >
            <span className="h-2.5 w-2.5 rounded-full" style={{ background: tg.color ?? "var(--cs-accent)" }} />
            <span className="flex-1 truncate">{tg.name}</span>
            {selected.includes(tg.id) && <Check size={14} strokeWidth={2.6} style={{ color: "var(--cs-accent)" }} />}
          </button>
        ))}
        {canCreate && (
          <button
            onMouseEnter={() => setIndex(filtered.length)}
            onClick={create}
            className="flex h-8 w-full items-center gap-2 rounded-[7px] px-2 text-left text-[12.5px]"
            style={{ background: index === filtered.length ? "var(--cs-surface-active)" : undefined }}
          >
            <Plus size={14} />
            {t("createTag", { name: q.trim() })}
          </button>
        )}
        {!rows && <div className="flex items-center gap-2 px-2 py-3 text-[12px]" style={{ color: "var(--cs-text-faint)" }}><Hash size={13} />{t("noTags")}</div>}
      </div>
    </ModalShell>
  );
}

export function SpacePicker({ ids }: { ids: number[] }) {
  const t = useT();
  const close = useUi((s) => s.closeModal);
  const spaces = useItems((s) => s.spaces);
  const actions = useItemActions();
  const [index, setIndex] = useState(0);
  const options = [{ id: null as number | null, name: t("none"), color: null as string | null, icon: null as string | null }, ...spaces];
  const choose = async (i: number) => {
    await actions.moveTo(ids, options[i].id);
    close();
  };
  const onKeyDown = useListKeys(options.length, index, setIndex, (i) => void choose(i));
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => ref.current?.focus(), []);
  return (
    <ModalShell label={t("moveToSpace")} width={300}>
      <div className="cs-caps px-4 pb-1 pt-3">{t("chooseSpace")}</div>
      <div ref={ref} tabIndex={0} onKeyDown={onKeyDown} role="listbox" className="p-1.5 outline-none">
        {options.map((s, i) => (
          <button key={s.id ?? "none"} role="option" aria-selected={index === i} onMouseEnter={() => setIndex(i)} onClick={() => choose(i)} className="flex h-8 w-full items-center gap-2 rounded-[7px] px-2 text-left text-[12.5px]" style={{ background: index === i ? "var(--cs-surface-active)" : undefined }}>
            <span style={{ color: s.color ?? "var(--cs-text-faint)" }}>{s.id === null ? <X size={14} /> : <SpaceIcon name={s.icon} />}</span>
            <span className="flex-1">{s.name}</span>
            {i < 9 && <span className="text-[10.5px]" style={{ color: "var(--cs-text-faint)" }}>{i}</span>}
          </button>
        ))}
      </div>
    </ModalShell>
  );
}

export function SpaceEditor({ space }: { space?: { id: number; name: string; color: string | null; icon: string | null } }) {
  const t = useT();
  const close = useUi((s) => s.closeModal);
  const openModal = useUi((s) => s.openModal);
  const selectSpace = useItems((s) => s.selectSpace);
  const [name, setName] = useState(space?.name ?? "");
  const [color, setColor] = useState(space?.color ?? SPACE_COLORS[0]);
  const [icon, setIcon] = useState(space?.icon ?? "folder");
  const save = async () => {
    if (!name.trim()) return;
    if (space) await ipc.renameSpace(space.id, name, color, icon);
    else {
      const id = await ipc.createSpace(name, color, icon);
      selectSpace({ type: "space", id });
    }
    close();
  };
  return (
    <ModalShell label={space ? t("rename") : t("newSpace")} width={360}>
      <div className="flex flex-col gap-3 p-4">
        <div className="text-[13px] font-semibold">{space ? space.name : t("newSpace")}</div>
        <input
          autoFocus
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              void save();
            }
          }}
          placeholder={t("newSpaceName")}
          className="h-9 rounded-[8px] px-3 text-[13px] outline-none"
          style={{ background: "var(--cs-field)" }}
        />
        <div className="flex gap-1.5">
          {SPACE_COLORS.map((c) => (
            <button key={c} aria-label={c} onClick={() => setColor(c)} className="h-6 w-6 rounded-full" style={{ background: c, boxShadow: color === c ? `0 0 0 2px var(--cs-surface), 0 0 0 4px ${c}` : undefined }} />
          ))}
        </div>
        <div className="flex flex-wrap gap-1">
          {Object.keys(SPACE_ICONS).map((k) => (
            <button key={k} aria-label={k} onClick={() => setIcon(k)} className="flex h-8 w-8 items-center justify-center rounded-[8px]" style={{ background: icon === k ? "var(--cs-accent-soft)" : "var(--cs-field)", color: icon === k ? color : undefined }}>
              <SpaceIcon name={k} size={15} />
            </button>
          ))}
        </div>
        <div className="flex items-center gap-2 pt-1">
          {space && (
            <button
              onClick={() =>
                openModal({
                  type: "confirm",
                  message: t("deleteSpaceConfirm", { name: space.name }),
                  onConfirm: async () => {
                    await ipc.deleteSpace(space.id);
                    selectSpace({ type: "all" });
                  },
                })
              }
              className="h-8 rounded-[8px] px-3 text-[12.5px] font-medium"
              style={{ color: "var(--cs-danger)" }}
            >
              {t("delete")}
            </button>
          )}
          <span className="flex-1" />
          <button onClick={close} className="h-8 rounded-[8px] px-3 text-[12.5px]" style={{ background: "var(--cs-field)" }}>
            {t("cancel")}
          </button>
          <button onClick={save} disabled={!name.trim()} className="h-8 rounded-[8px] px-3 text-[12.5px] font-semibold disabled:opacity-40" style={{ background: "var(--cs-accent)", color: "var(--cs-accent-text)" }}>
            {t("save")}
          </button>
        </div>
      </div>
    </ModalShell>
  );
}

export function ConfirmDialog({ message, onConfirm }: { message: string; onConfirm: () => void }) {
  const t = useT();
  const close = useUi((s) => s.closeModal);
  const ref = useRef<HTMLButtonElement>(null);
  useEffect(() => ref.current?.focus(), []);
  return (
    <ModalShell label={message} width={340}>
      <div className="p-4 text-[13px] leading-relaxed">{message}</div>
      <div className="flex justify-end gap-2 px-4 pb-4">
        <button onClick={close} className="h-8 rounded-[8px] px-3 text-[12.5px]" style={{ background: "var(--cs-field)" }}>
          {t("cancel")}
        </button>
        <button
          ref={ref}
          onClick={() => {
            close();
            onConfirm();
          }}
          className="h-8 rounded-[8px] px-3 text-[12.5px] font-semibold"
          style={{ background: "var(--cs-danger)", color: "#fff" }}
        >
          {t("delete")}
        </button>
      </div>
    </ModalShell>
  );
}
