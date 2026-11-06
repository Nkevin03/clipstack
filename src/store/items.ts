import { create } from "zustand";
import { emptyQuery, ipc, type Item, type ItemQuery, type Kind, type Sort, type Space, type Tag, type SourceAppInfo, type Counts } from "../lib/ipc";
import { applyChips, extractFilters, type Chip } from "../lib/search-syntax";

export type SpaceSel = { type: "all" } | { type: "pinned" } | { type: "space"; id: number };

const PAGE = 60;

type ItemsState = {
  // query
  input: string;
  chips: Chip[];
  kinds: Kind[];
  apps: string[];
  tagFilter: string[];
  space: SpaceSel;
  sort: Sort;
  // data
  items: Item[];
  hasMore: boolean;
  loading: boolean;
  spaces: Space[];
  tags: Tag[];
  sourceApps: SourceAppInfo[];
  counts: Counts;
  // selection
  index: number;
  multi: number[];
  anchor: number | null;
  // actions
  setInput: (v: string, final?: boolean) => void;
  removeChip: (i: number) => void;
  popChip: () => void;
  clearSearch: () => void;
  toggleKind: (k: Kind) => void;
  setKinds: (k: Kind[]) => void;
  toggleApp: (bundleId: string) => void;
  toggleTag: (name: string) => void;
  selectSpace: (s: SpaceSel) => void;
  cycleSpace: (dir: 1 | -1) => void;
  setSort: (s: Sort) => void;
  buildQuery: (offset?: number) => ItemQuery;
  refresh: () => Promise<void>;
  loadMore: () => Promise<void>;
  loadMeta: () => Promise<void>;
  prepend: (item: Item) => void;
  setIndex: (i: number) => void;
  move: (delta: number, extend?: boolean) => void;
  toggleMulti: (id: number) => void;
  clearMulti: () => void;
  selectedIds: () => number[];
  current: () => Item | undefined;
};

let reqSeq = 0;

export const useItems = create<ItemsState>((set, get) => ({
  input: "",
  chips: [],
  kinds: [],
  apps: [],
  tagFilter: [],
  space: { type: "all" },
  sort: "recent",
  items: [],
  hasMore: false,
  loading: false,
  spaces: [],
  tags: [],
  sourceApps: [],
  counts: { all: 0, pinned: 0 },
  index: 0,
  multi: [],
  anchor: null,

  setInput: (v, final = false) => {
    const { text, chips } = extractFilters(v, final);
    if (chips.length) {
      set({ input: text, chips: [...get().chips, ...chips] });
    } else {
      set({ input: final ? text : v });
    }
    void get().refresh();
  },
  removeChip: (i) => {
    set({ chips: get().chips.filter((_, j) => j !== i) });
    void get().refresh();
  },
  popChip: () => {
    if (!get().chips.length) return;
    set({ chips: get().chips.slice(0, -1) });
    void get().refresh();
  },
  clearSearch: () => {
    set({ input: "", chips: [] });
    void get().refresh();
  },
  toggleKind: (k) => {
    const kinds = get().kinds.includes(k) ? get().kinds.filter((x) => x !== k) : [...get().kinds, k];
    set({ kinds });
    void get().refresh();
  },
  setKinds: (kinds) => {
    set({ kinds });
    void get().refresh();
  },
  toggleApp: (b) => {
    const apps = get().apps.includes(b) ? get().apps.filter((x) => x !== b) : [...get().apps, b];
    set({ apps });
    void get().refresh();
  },
  toggleTag: (name) => {
    const t = get().tagFilter;
    set({ tagFilter: t.includes(name) ? t.filter((x) => x !== name) : [...t, name] });
    void get().refresh();
  },
  selectSpace: (space) => {
    set({ space });
    void get().refresh();
  },
  cycleSpace: (dir) => {
    const { spaces, space } = get();
    const all: SpaceSel[] = [{ type: "all" }, { type: "pinned" }, ...spaces.map((s) => ({ type: "space" as const, id: s.id }))];
    const cur = all.findIndex((s) => s.type === space.type && (s.type !== "space" || (space.type === "space" && s.id === space.id)));
    get().selectSpace(all[(cur + dir + all.length) % all.length]);
  },
  setSort: (sort) => {
    set({ sort });
    void get().refresh();
  },

  buildQuery: (offset = 0) => {
    const s = get();
    const base: ItemQuery = {
      ...emptyQuery(),
      kinds: [...s.kinds],
      apps: [...s.apps],
      tags: [...s.tagFilter],
      sort: s.sort,
      offset,
      limit: PAGE,
      spaceId: s.space.type === "space" ? s.space.id : null,
      pinned: s.space.type === "pinned" ? true : null,
    };
    return applyChips(base, s.input.trim(), s.chips);
  },
  refresh: async () => {
    const id = ++reqSeq;
    set({ loading: true });
    try {
      const items = await ipc.getItems(get().buildQuery(0));
      if (id !== reqSeq) return;
      const prevId = get().items[get().index]?.id;
      const keep = prevId !== undefined ? items.findIndex((i) => i.id === prevId) : -1;
      const ids = new Set(items.map((i) => i.id));
      set({
        items,
        hasMore: items.length === PAGE,
        loading: false,
        index: keep >= 0 && get().input === "" ? keep : 0,
        multi: get().multi.filter((m) => ids.has(m)),
      });
    } catch (e) {
      console.error(e);
      if (id === reqSeq) set({ loading: false });
    }
  },
  loadMore: async () => {
    const s = get();
    if (!s.hasMore || s.loading) return;
    const id = ++reqSeq;
    set({ loading: true });
    const more = await ipc.getItems(s.buildQuery(s.items.length));
    if (id !== reqSeq) return;
    const seen = new Set(get().items.map((i) => i.id));
    set({ items: [...get().items, ...more.filter((m) => !seen.has(m.id))], hasMore: more.length === PAGE, loading: false });
  },
  loadMeta: async () => {
    const [spaces, tags, sourceApps, counts] = await Promise.all([ipc.listSpaces(), ipc.listTags(), ipc.listSourceApps(), ipc.getCounts()]);
    set({ spaces, tags, sourceApps, counts });
  },
  prepend: (item) => {
    // Re-run the query so filters/ordering stay correct; cheap (<5 ms).
    void get().refresh();
    void get().loadMeta();
    void item;
  },
  setIndex: (i) => set({ index: Math.max(0, Math.min(i, get().items.length - 1)), anchor: null, multi: [] }),
  move: (delta, extend = false) => {
    const s = get();
    if (!s.items.length) return;
    const next = Math.max(0, Math.min(s.index + delta, s.items.length - 1));
    if (extend) {
      const anchor = s.anchor ?? s.index;
      const [a, b] = anchor < next ? [anchor, next] : [next, anchor];
      set({ index: next, anchor, multi: s.items.slice(a, b + 1).map((i) => i.id) });
    } else {
      set({ index: next, anchor: null, multi: [] });
    }
    if (next >= s.items.length - 10) void get().loadMore();
  },
  toggleMulti: (id) => {
    const s = get();
    const base = s.multi.length ? s.multi : s.items[s.index] ? [s.items[s.index].id] : [];
    const multi = base.includes(id) ? base.filter((x) => x !== id) : [...base, id];
    const idx = s.items.findIndex((i) => i.id === id);
    set({ multi, index: idx >= 0 ? idx : s.index });
  },
  clearMulti: () => set({ multi: [], anchor: null }),
  selectedIds: () => {
    const s = get();
    if (s.multi.length) return s.items.filter((i) => s.multi.includes(i.id)).map((i) => i.id);
    const cur = s.items[s.index];
    return cur ? [cur.id] : [];
  },
  current: () => get().items[get().index],
}));
