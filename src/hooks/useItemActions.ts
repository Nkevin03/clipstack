import { useCallback } from "react";
import { ipc, type Item } from "../lib/ipc";
import { useItems } from "../store/items";
import { useUi } from "../store/ui";
import { useT } from "../lib/i18n";

/** All user-facing operations on items, shared by keyboard, mouse and menus. */
export function useItemActions() {
  const t = useT();
  const toast = useUi((s) => s.toast);
  const setAccessibility = useUi((s) => s.setAccessibility);

  const fail = useCallback(
    (e: unknown) => {
      console.error(e);
      toast({ text: String(e) === "duplicate" ? t("duplicate") : `${t("error")}: ${String(e)}`, tone: "error" });
    },
    [t, toast],
  );

  const paste = useCallback(
    async (plain = false, ids?: number[]) => {
      const sel = ids ?? useItems.getState().selectedIds();
      if (!sel.length) return;
      try {
        const out = sel.length > 1 ? await ipc.pasteMany(sel, plain) : await ipc.pasteItem(sel[0], plain);
        if (out.needsPermission) setAccessibility(false);
        useItems.getState().clearMulti();
      } catch (e) {
        fail(e);
      }
    },
    [fail, setAccessibility],
  );

  const pasteIndex = useCallback(
    (n: number) => {
      const it = useItems.getState().items[n];
      if (it) void paste(false, [it.id]);
    },
    [paste],
  );

  const copyOnly = useCallback(
    async (ids?: number[]) => {
      const sel = ids ?? useItems.getState().selectedIds();
      if (!sel.length) return;
      try {
        await ipc.copyItem(sel, false);
        toast({ text: t("copied") });
      } catch (e) {
        fail(e);
      }
    },
    [fail, t, toast],
  );

  const togglePin = useCallback(
    async (ids?: number[]) => {
      const st = useItems.getState();
      const sel = ids ?? st.selectedIds();
      if (!sel.length) return;
      const items = st.items.filter((i) => sel.includes(i.id));
      const value = !items.every((i) => i.pinned);
      try {
        await ipc.pinItem(sel, value);
        toast({ text: value ? t("pinnedToast") : t("unpinnedToast") });
      } catch (e) {
        fail(e);
      }
    },
    [fail, t, toast],
  );

  const undo = useCallback(async () => {
    try {
      const restored = await ipc.undoDelete();
      if (restored.length) toast({ text: t("restored") });
    } catch (e) {
      fail(e);
    }
  }, [fail, t, toast]);

  const remove = useCallback(
    async (ids?: number[]) => {
      const sel = ids ?? useItems.getState().selectedIds();
      if (!sel.length) return;
      try {
        await ipc.deleteItems(sel);
        useItems.getState().clearMulti();
        toast(
          { text: sel.length > 1 ? t("deletedN", { n: sel.length }) : t("deleted"), action: { label: t("undo"), run: () => void undo() } },
          6000,
        );
      } catch (e) {
        fail(e);
      }
    },
    [fail, t, toast, undo],
  );

  const moveTo = useCallback(
    async (ids: number[], spaceId: number | null) => {
      try {
        await ipc.moveToSpace(ids, spaceId);
        const space = useItems.getState().spaces.find((s) => s.id === spaceId);
        toast({ text: space ? t("moved", { name: space.name }) : t("removedFromSpace") });
      } catch (e) {
        fail(e);
      }
    },
    [fail, t, toast],
  );

  const openLink = useCallback(
    (item: Item) => {
      ipc.openUrl(item.preview.trim()).catch(fail);
    },
    [fail],
  );

  const reveal = useCallback((item: Item) => ipc.revealInFinder(item.id).catch(fail), [fail]);

  return { paste, pasteIndex, copyOnly, togglePin, remove, undo, moveTo, openLink, reveal, fail };
}
