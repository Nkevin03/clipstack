import { useEffect } from "react";
import { ipc } from "../lib/ipc";
import { useItems } from "../store/items";
import { useUi } from "../store/ui";
import { useSettings } from "../store/settings";
import { useItemActions } from "./useItemActions";

/**
 * Global key handling for the panel (BRIEF §6). Typing printable characters focuses the search
 * field; modal dialogs handle their own keys and stop propagation.
 */
export function useKeyboardNav(searchRef: React.RefObject<HTMLInputElement | null>, columns: () => number) {
  const actions = useItemActions();

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const ui = useUi.getState();
      const items = useItems.getState();
      const cmd = e.metaKey;
      const target = e.target as HTMLElement | null;
      const inSearch = target === searchRef.current;
      const inOtherField = !inSearch && !!target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable);

      if (ui.modal) {
        if (ui.modal.type === "preview" && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          ui.closeModal();
          if (e.key === "Enter") void actions.paste(e.shiftKey);
          return;
        }
        if (e.key === "Escape") {
          e.preventDefault();
          ui.closeModal();
        }
        return; // modals own the keyboard
      }
      if (ui.contextMenu) {
        if (e.key === "Escape") {
          e.preventDefault();
          ui.setContextMenu(null);
        }
        return;
      }
      if (inOtherField) return;

      const cur = items.current();
      const key = e.key;

      if (key === "Escape") {
        e.preventDefault();
        if (items.multi.length) items.clearMulti();
        else if (items.input || items.chips.length) items.clearSearch();
        else void ipc.hidePanel();
        return;
      }
      if (key === "Enter") {
        e.preventDefault();
        if (inSearch && searchRef.current) items.setInput(searchRef.current.value, true);
        void actions.paste(e.shiftKey);
        return;
      }
      if (cmd && /^[1-9]$/.test(key)) {
        e.preventDefault();
        actions.pasteIndex(parseInt(key, 10) - 1);
        return;
      }
      const horizontal = useSettings.getState().settings?.layout !== "center";
      const step = columns();
      const nav: Record<string, number> = horizontal
        ? { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -step, ArrowDown: step }
        : { ArrowUp: -1, ArrowDown: 1 };
      if (key in nav && !(inSearch && (key === "ArrowLeft" || key === "ArrowRight") && searchRef.current?.value && !e.shiftKey)) {
        e.preventDefault();
        items.move(nav[key], e.shiftKey);
        return;
      }
      if (key === "Tab") {
        e.preventDefault();
        items.cycleSpace(e.shiftKey ? -1 : 1);
        return;
      }
      if (cmd) {
        switch (key.toLowerCase()) {
          case "f":
            e.preventDefault();
            searchRef.current?.focus();
            searchRef.current?.select();
            return;
          case "d":
            e.preventDefault();
            void actions.togglePin();
            return;
          case "t":
            e.preventDefault();
            if (cur) ui.openModal({ type: "tags", id: cur.id });
            return;
          case "m":
            e.preventDefault();
            if (cur) ui.openModal({ type: "space", ids: items.selectedIds() });
            return;
          case "e":
            e.preventDefault();
            if (cur && cur.kind !== "image" && cur.kind !== "file") ui.openModal({ type: "edit", id: cur.id });
            return;
          case "k":
            e.preventDefault();
            if (cur && cur.kind !== "image" && cur.kind !== "file") ui.openModal({ type: "actions", id: cur.id });
            return;
          case "z":
            if (!inSearch || !searchRef.current?.value) {
              e.preventDefault();
              void actions.undo();
            }
            return;
          case "b":
            e.preventDefault();
            void useSettings.getState().set("sidebarCollapsed", !useSettings.getState().settings?.sidebarCollapsed);
            return;
          case ",":
            e.preventDefault();
            void ipc.openSettings();
            return;
          case "c":
            if (!inSearch || !window.getSelection()?.toString()) {
              e.preventDefault();
              void actions.copyOnly();
            }
            return;
          case "backspace":
            if (!inSearch || !searchRef.current?.value) {
              e.preventDefault();
              void actions.remove();
            }
            return;
        }
        return;
      }
      if (key === " " && !inSearch) {
        e.preventDefault();
        if (cur) ui.openModal({ type: "preview", id: cur.id });
        return;
      }
      if (key === " " && inSearch && !searchRef.current?.value) {
        e.preventDefault();
        if (cur) ui.openModal({ type: "preview", id: cur.id });
        return;
      }
      if (key === "Backspace" && inSearch && !searchRef.current?.value) {
        items.popChip();
        return;
      }
      // Start typing anywhere → search.
      if (!inSearch && key.length === 1 && !e.ctrlKey && !e.altKey) {
        searchRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [actions, columns, searchRef]);
}
