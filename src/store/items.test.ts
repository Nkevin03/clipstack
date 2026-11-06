import { beforeEach, describe, expect, it, vi } from "vitest";

const getItems = vi.fn();
vi.mock("../lib/ipc", async () => {
  const actual = await vi.importActual<typeof import("../lib/ipc")>("../lib/ipc");
  return { ...actual, ipc: { ...actual.ipc, getItems: (q: unknown) => getItems(q) } };
});

const { useItems } = await import("./items");

const item = (id: number) => ({ id, kind: "text", preview: `item ${id}`, tagIds: [], filePaths: [] });

describe("items store", () => {
  beforeEach(() => {
    getItems.mockReset();
    getItems.mockResolvedValue([1, 2, 3, 4, 5].map(item));
    useItems.setState({ items: [1, 2, 3, 4, 5].map(item) as never, index: 0, multi: [], anchor: null, input: "", chips: [], kinds: [], apps: [], tagFilter: [], space: { type: "all" } });
  });

  it("moves and extends the selection with shift", () => {
    const s = useItems.getState();
    s.move(1);
    expect(useItems.getState().index).toBe(1);
    s.move(2, true);
    expect(useItems.getState().multi).toEqual([2, 3, 4]);
    expect(useItems.getState().selectedIds()).toEqual([2, 3, 4]);
    s.move(-1);
    expect(useItems.getState().multi).toEqual([]);
    expect(useItems.getState().selectedIds()).toEqual([3]);
  });

  it("cmd-click toggles items into the multi selection", () => {
    const s = useItems.getState();
    s.toggleMulti(3);
    expect(useItems.getState().multi).toEqual([1, 3]);
    s.toggleMulti(1);
    expect(useItems.getState().multi).toEqual([3]);
  });

  it("turns typed filters into chips and builds the backend query", async () => {
    useItems.getState().setInput("type:link app:safari hello ");
    const st = useItems.getState();
    expect(st.chips.map((c) => c.key)).toEqual(["type", "app"]);
    expect(st.input).toBe("hello ");
    useItems.getState().selectSpace({ type: "pinned" });
    const q = useItems.getState().buildQuery();
    expect(q).toMatchObject({ text: "hello", kinds: ["link"], appNames: ["safari"], pinned: true, spaceId: null, limit: 60 });
    expect(getItems).toHaveBeenCalled();
  });

  it("cycles spaces with Tab order", () => {
    useItems.setState({ spaces: [{ id: 7, name: "Snippets", color: null, icon: null, position: 0, count: 0 }] });
    const s = useItems.getState();
    s.cycleSpace(1);
    expect(useItems.getState().space).toEqual({ type: "pinned" });
    s.cycleSpace(1);
    expect(useItems.getState().space).toEqual({ type: "space", id: 7 });
    s.cycleSpace(1);
    expect(useItems.getState().space).toEqual({ type: "all" });
  });
});
