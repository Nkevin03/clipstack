import { create } from "zustand";
import type { Space } from "../lib/ipc";

export type Modal =
  | { type: "preview"; id: number }
  | { type: "tags"; id: number }
  | { type: "space"; ids: number[] }
  | { type: "actions"; id: number }
  | { type: "edit"; id: number }
  | { type: "space-edit"; space?: Space }
  | { type: "confirm"; message: string; onConfirm: () => void };

export type Toast = { id: number; text: string; action?: { label: string; run: () => void }; tone?: "error" };

export type ContextMenu = { x: number; y: number; id: number } | null;

type UiState = {
  modal: Modal | null;
  toasts: Toast[];
  contextMenu: ContextMenu;
  paused: boolean;
  accessibility: boolean;
  visibleTick: number;
  openModal: (m: Modal) => void;
  closeModal: () => void;
  toast: (t: Omit<Toast, "id">, ms?: number) => void;
  dismissToast: (id: number) => void;
  setContextMenu: (c: ContextMenu) => void;
  setPaused: (v: boolean) => void;
  setAccessibility: (v: boolean) => void;
  bumpVisible: () => void;
};

let toastSeq = 1;

export const useUi = create<UiState>((set, get) => ({
  modal: null,
  toasts: [],
  contextMenu: null,
  paused: false,
  accessibility: true,
  visibleTick: 0,
  openModal: (modal) => set({ modal, contextMenu: null }),
  closeModal: () => set({ modal: null }),
  toast: (t, ms = 3200) => {
    const id = toastSeq++;
    set({ toasts: [...get().toasts.slice(-2), { ...t, id }] });
    window.setTimeout(() => get().dismissToast(id), ms);
  },
  dismissToast: (id) => set({ toasts: get().toasts.filter((t) => t.id !== id) }),
  setContextMenu: (contextMenu) => set({ contextMenu }),
  setPaused: (paused) => set({ paused }),
  setAccessibility: (accessibility) => set({ accessibility }),
  bumpVisible: () => set({ visibleTick: get().visibleTick + 1 }),
}));
