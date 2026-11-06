import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import { useVirtualizer } from "@tanstack/react-virtual";
import { PanelLeftClose, PanelLeftOpen, ShieldAlert } from "lucide-react";
import { ipc, type Item, type ItemDetail } from "../lib/ipc";
import { useItems } from "../store/items";
import { useUi } from "../store/ui";
import { useSettings } from "../store/settings";
import { useT, useLang } from "../lib/i18n";
import { timeAgo } from "../lib/format";
import { useKeyboardNav } from "../hooks/useKeyboardNav";
import { useItemActions } from "../hooks/useItemActions";
import { useTauriEvent } from "../hooks/useTauriEvent";
import { Card, AppBadge } from "./Card/Card";
import { Sidebar } from "./Sidebar";
import { SearchBar } from "./SearchBar";
import { AppFilterBar, KIND_ICONS, SettingsButton, TypeMenu } from "./AppFilterBar";
import { EmptyState } from "./EmptyState";
import { Toasts } from "./Toasts";
import { ContextMenu } from "./ContextMenu";
import { PreviewContent, PreviewModal } from "./PreviewModal";
import { ConfirmDialog, SpaceEditor, SpacePicker, TagPicker } from "./Pickers";
import { ActionsMenu } from "./ActionsMenu";
import { EditModal } from "./EditModal";
import { Highlighted } from "./Highlighted";
import type { Settings } from "../lib/ipc";

const CARD = { small: 186, medium: 230, large: 272 } as const;

function cardWidth(s: Settings | null) {
  return CARD[(s?.cardSize as keyof typeof CARD) ?? "medium"] ?? 230;
}
function gap(s: Settings | null) {
  return s?.density === "compact" ? 9 : 14;
}

function useTerms() {
  const input = useItems((s) => s.input);
  return useMemo(() => input.split(/\s+/).filter((x) => x.length > 0), [input]);
}

function useCardHandlers() {
  const actions = useItemActions();
  const onSelect = useCallback((index: number, e: React.MouseEvent) => {
    const st = useItems.getState();
    if (e.metaKey) st.toggleMulti(st.items[index].id);
    else if (e.shiftKey) st.move(index - st.index, true);
    else st.setIndex(index);
  }, []);
  const onActivate = useCallback((index: number) => {
    const it = useItems.getState().items[index];
    if (it) void actions.paste(false, [it.id]);
  }, [actions]);
  const onContext = useCallback((index: number, e: React.MouseEvent) => {
    const st = useItems.getState();
    const it = st.items[index];
    if (!st.multi.includes(it.id)) st.setIndex(index);
    useUi.getState().setContextMenu({ x: e.clientX, y: e.clientY, id: it.id });
  }, []);
  return { onSelect, onActivate, onContext };
}

function Empty() {
  const { items, input, chips, kinds, apps, tagFilter, space, loading } = useItems();
  const paused = useUi((s) => s.paused);
  if (items.length || loading) return null;
  const filtered = input || chips.length || kinds.length || apps.length || tagFilter.length;
  const variant = filtered ? "noResults" : space.type === "space" ? "emptySpace" : paused ? "paused" : "empty";
  return (
    <div className="absolute inset-0">
      <EmptyState variant={variant} />
    </div>
  );
}

/** Horizontal, virtualised strip of cards (top/bottom layouts). */
function CardStrip() {
  const settings = useSettings((s) => s.settings);
  const { items, index, multi, tags, hasMore, loadMore } = useItems();
  const visibleTick = useUi((s) => s.visibleTick);
  const terms = useTerms();
  const handlers = useCardHandlers();
  const scrollRef = useRef<HTMLDivElement>(null);
  const w = cardWidth(settings);
  const g = gap(settings);
  const v = useVirtualizer({
    horizontal: true,
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => w + g,
    overscan: 6,
    paddingStart: 16,
    paddingEnd: 16,
  });
  useEffect(() => v.measure(), [w, g, v]);
  useEffect(() => {
    if (items.length) v.scrollToIndex(index, { align: "auto" });
  }, [index, items.length, v]);
  useEffect(() => {
    scrollRef.current?.scrollTo({ left: 0 });
  }, [visibleTick]);
  const virtualItems = v.getVirtualItems();
  const last = virtualItems[virtualItems.length - 1];
  useEffect(() => {
    if (last && hasMore && last.index >= items.length - 8) void loadMore();
  }, [last, hasMore, items.length, loadMore]);

  return (
    <div
      ref={scrollRef}
      role="listbox"
      aria-orientation="horizontal"
      aria-multiselectable
      className="cs-noscrollbar relative min-h-0 flex-1 overflow-x-auto overflow-y-hidden"
      onWheel={(e) => {
        // Map vertical wheel to horizontal scrolling.
        if (Math.abs(e.deltaY) > Math.abs(e.deltaX) && scrollRef.current) scrollRef.current.scrollLeft += e.deltaY;
      }}
    >
      <div style={{ width: v.getTotalSize(), height: "100%", position: "relative" }}>
        {virtualItems.map((vi) => {
          const item = items[vi.index];
          return (
            <div key={item.id} className="absolute top-0 flex h-full items-center" style={{ transform: `translateX(${vi.start}px)`, width: w }}>
              <Card
                item={item}
                index={vi.index}
                selected={vi.index === index && !multi.length}
                inMulti={multi.includes(item.id)}
                terms={terms}
                tags={tags}
                animateIn={vi.index < 10}
                {...handlers}
              />
            </div>
          );
        })}
      </div>
      <Empty />
    </div>
  );
}

function Row({ item, index, selected, inMulti, terms, ...h }: { item: Item; index: number; selected: boolean; inMulti: boolean; terms: string[] } & ReturnType<typeof useCardHandlers>) {
  const t = useT();
  const lang = useLang();
  const Icon = KIND_ICONS[item.kind];
  const label =
    item.kind === "image" ? `${t("kind_image")} ${item.width}×${item.height}` : item.kind === "file" ? item.filePaths.map((p) => p.split("/").pop()).join(", ") : item.preview.replace(/\s+/g, " ").slice(0, 160);
  return (
    <div
      role="option"
      aria-selected={selected || inMulti}
      onMouseDown={(e) => e.button === 0 && h.onSelect(index, e)}
      onDoubleClick={() => h.onActivate(index)}
      onContextMenu={(e) => {
        e.preventDefault();
        h.onContext(index, e);
      }}
      draggable
      onDragStart={(e) => e.dataTransfer.setData("application/x-clipstack-item", String(item.id))}
      className="mx-2 flex h-[44px] items-center gap-2.5 rounded-[9px] px-2.5"
      style={{ background: selected || inMulti ? "var(--cs-accent)" : undefined, color: selected || inMulti ? "var(--cs-accent-text)" : undefined }}
    >
      <AppBadge item={item} size={22} />
      <div className="min-w-0 flex-1">
        <div className="truncate text-[12.5px]">
          <Highlighted text={label} terms={terms} />
        </div>
        <div className="flex items-center gap-1 text-[10.5px] opacity-70">
          <Icon size={10} />
          {item.sourceAppName ?? t("unknownApp")} · {timeAgo(item.lastUsedAt, lang)}
        </div>
      </div>
      {index < 9 && <span className="text-[10.5px] font-semibold opacity-60">⌘{index + 1}</span>}
    </div>
  );
}

/** Spotlight-style layout: vertical list + large preview. */
function CenterLayout() {
  const { items, index, multi, hasMore, loadMore } = useItems();
  const terms = useTerms();
  const handlers = useCardHandlers();
  const scrollRef = useRef<HTMLDivElement>(null);
  const v = useVirtualizer({ count: items.length, getScrollElement: () => scrollRef.current, estimateSize: () => 46, overscan: 10, paddingStart: 6, paddingEnd: 6 });
  useEffect(() => {
    if (items.length) v.scrollToIndex(index, { align: "auto" });
  }, [index, items.length, v]);
  const vis = v.getVirtualItems();
  const last = vis[vis.length - 1];
  useEffect(() => {
    if (last && hasMore && last.index >= items.length - 10) void loadMore();
  }, [last, hasMore, items.length, loadMore]);
  const cur = items[index];
  const [detail, setDetail] = useState<ItemDetail | null>(null);
  useEffect(() => {
    if (!cur) return setDetail(null);
    let alive = true;
    const timer = window.setTimeout(() => ipc.getItem(cur.id).then((d) => alive && setDetail(d)), 40);
    return () => {
      alive = false;
      window.clearTimeout(timer);
    };
  }, [cur?.id, cur?.lastUsedAt]); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="relative flex min-h-0 flex-1">
      <div ref={scrollRef} role="listbox" aria-multiselectable className="cs-scroll relative w-[42%] max-w-[380px] shrink-0 overflow-y-auto" style={{ borderRight: "1px solid var(--cs-border)" }}>
        <div style={{ height: v.getTotalSize(), position: "relative" }}>
          {vis.map((vi) => {
            const item = items[vi.index];
            return (
              <div key={item.id} className="absolute left-0 right-0 top-0" style={{ transform: `translateY(${vi.start}px)` }}>
                <Row item={item} index={vi.index} selected={vi.index === index && !multi.length} inMulti={multi.includes(item.id)} terms={terms} {...handlers} />
              </div>
            );
          })}
        </div>
      </div>
      <div className="flex min-w-0 flex-1 flex-col">{detail && detail.item.id === cur?.id && <PreviewContent detail={detail} large />}</div>
      <Empty />
    </div>
  );
}

function Modals() {
  const modal = useUi((s) => s.modal);
  return (
    <AnimatePresence>
      {modal?.type === "preview" && <PreviewModal key="preview" id={modal.id} />}
      {modal?.type === "tags" && <TagPicker key="tags" id={modal.id} />}
      {modal?.type === "space" && <SpacePicker key="space" ids={modal.ids} />}
      {modal?.type === "actions" && <ActionsMenu key="actions" id={modal.id} />}
      {modal?.type === "edit" && <EditModal key="edit" id={modal.id} />}
      {modal?.type === "space-edit" && <SpaceEditor key="space-edit" space={modal.space} />}
      {modal?.type === "confirm" && <ConfirmDialog key="confirm" message={modal.message} onConfirm={modal.onConfirm} />}
    </AnimatePresence>
  );
}

function PermissionBanner() {
  const t = useT();
  const ok = useUi((s) => s.accessibility);
  const autoPaste = useSettings((s) => s.settings?.autoPaste);
  if (ok || !autoPaste) return null;
  return (
    <div className="mx-3 mb-1 flex shrink-0 items-center gap-2 rounded-[8px] px-3 py-1.5 text-[11.5px]" style={{ background: "color-mix(in srgb, #f59e0b 18%, transparent)" }}>
      <ShieldAlert size={14} style={{ color: "#d97706" }} />
      <span className="flex-1">{t("permissionBanner")}</span>
      <button onClick={() => ipc.requestAccessibility()} className="rounded-[6px] px-2 py-0.5 font-semibold" style={{ background: "rgb(0 0 0 / 0.08)" }}>
        {t("enable")}
      </button>
    </div>
  );
}

function MultiBar() {
  const t = useT();
  const multi = useItems((s) => s.multi);
  const actions = useItemActions();
  if (multi.length < 2) return null;
  return (
    <div className="flex shrink-0 items-center gap-2 text-[11.5px] font-medium" style={{ color: "var(--cs-text-muted)" }}>
      <span>{t("selected", { n: multi.length })}</span>
      <button onClick={() => actions.paste(false)} className="rounded-[6px] px-2 py-0.5 font-semibold" style={{ background: "var(--cs-accent)", color: "var(--cs-accent-text)" }}>
        {t("pasteAll")} ↩
      </button>
    </div>
  );
}

export function Panel() {
  const t = useT();
  const settings = useSettings((s) => s.settings);
  const setSetting = useSettings((s) => s.set);
  const layout = settings?.layout ?? "top";
  const collapsed = settings?.sidebarCollapsed ?? false;
  const searchRef = useRef<HTMLInputElement>(null);
  const reduced = useReducedMotion();
  const visibleTick = useUi((s) => s.visibleTick);
  const columns = useCallback(() => 1, []);
  useKeyboardNav(searchRef, columns);

  useTauriEvent("panel:shown", () => {
    useUi.getState().bumpVisible();
    const st = useItems.getState();
    st.setIndex(0);
    void st.refresh();
    void st.loadMeta();
    ipc.checkAccessibility().then((v) => useUi.getState().setAccessibility(v));
    window.setTimeout(() => searchRef.current?.focus(), 10);
  });
  useTauriEvent("panel:hidden", () => {
    const ui = useUi.getState();
    ui.closeModal();
    ui.setContextMenu(null);
    // Fresh start next time: clear the search, keep filters chosen with the mouse.
    useItems.setState({ input: "", chips: [], multi: [], anchor: null });
  });

  const from = reduced ? { opacity: 0 } : layout === "top" ? { opacity: 0, y: -18 } : layout === "bottom" ? { opacity: 0, y: 18 } : { opacity: 0, scale: 0.97 };

  return (
    <motion.div
      key={visibleTick}
      initial={from}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      transition={{ type: "spring", stiffness: 520, damping: 40, mass: 0.8 }}
      className="relative flex h-full w-full flex-col overflow-hidden"
      style={{ borderRadius: "var(--cs-radius-panel)", background: "var(--cs-panel-tint)", boxShadow: "inset 0 0 0 1px var(--cs-panel-border)" }}
    >
      <header className="flex h-[52px] shrink-0 items-center gap-2 px-3" data-tauri-drag-region={layout === "center" ? true : undefined}>
        <button
          aria-label={t("toggleSidebar")}
          title={t("toggleSidebar")}
          onClick={() => setSetting("sidebarCollapsed", !collapsed)}
          className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[8px] transition-colors hover:bg-[var(--cs-surface-hover)]"
          style={{ color: "var(--cs-text-muted)" }}
        >
          {collapsed ? <PanelLeftOpen size={16} /> : <PanelLeftClose size={16} />}
        </button>
        <div className="flex min-w-0 flex-1 justify-center">
          <div className="flex w-full max-w-[640px] items-center gap-2">
            <SearchBar ref={searchRef} />
            <MultiBar />
          </div>
        </div>
        {layout !== "center" && <AppFilterBar />}
        <TypeMenu />
        <SettingsButton />
      </header>
      <PermissionBanner />
      <div className="flex min-h-0 flex-1">
        <AnimatePresence initial={false}>
          {!collapsed && (
            <motion.div initial={{ width: 0, opacity: 0 }} animate={{ width: 184, opacity: 1 }} exit={{ width: 0, opacity: 0 }} transition={{ duration: 0.18 }} className="shrink-0 overflow-hidden" style={{ borderRight: "1px solid var(--cs-border)" }}>
              <Sidebar />
            </motion.div>
          )}
        </AnimatePresence>
        {layout === "center" ? <CenterLayout /> : <CardStrip />}
      </div>
      <Modals />
      <ContextMenu />
      <Toasts />
    </motion.div>
  );
}
