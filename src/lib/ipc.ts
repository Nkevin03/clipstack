import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import type { Item } from "./bindings/Item";
import type { ItemDetail } from "./bindings/ItemDetail";
import type { ItemQuery } from "./bindings/ItemQuery";
import type { Counts } from "./bindings/Counts";
import type { Space } from "./bindings/Space";
import type { Tag } from "./bindings/Tag";
import type { SourceAppInfo } from "./bindings/SourceAppInfo";
import type { Settings } from "./bindings/Settings";
import type { ExcludedApp } from "./bindings/ExcludedApp";
import type { Action } from "./bindings/Action";
import type { ActionMode } from "./bindings/ActionMode";
import type { ActionResult } from "./bindings/ActionResult";
import type { AppInfo } from "./bindings/AppInfo";

export type { Item, ItemDetail, ItemQuery, Counts, Space, Tag, SourceAppInfo, Settings, ExcludedApp, Action, ActionMode, ActionResult, AppInfo };
export type { Kind } from "./bindings/Kind";
export type { Layout } from "./bindings/Layout";
export type { Sort } from "./bindings/Sort";

export type PasteOutcome = { pasted: boolean; needsPermission: boolean };

/** Local file → URL loadable by the webview (asset protocol, scoped to the app data dir). */
export const fileSrc = (path: string | null | undefined) => (path ? convertFileSrc(path) : undefined);

export const ipc = {
  getItems: (query: ItemQuery) => invoke<Item[]>("get_items", { query }),
  getItem: (id: number) => invoke<ItemDetail | null>("get_item", { id }),
  getCounts: () => invoke<Counts>("get_counts"),
  pasteItem: (id: number, plain: boolean) => invoke<PasteOutcome>("paste_item", { id, plain }),
  pasteMany: (ids: number[], plain: boolean, separator?: string) =>
    invoke<PasteOutcome>("paste_many", { ids, plain, separator: separator ?? null }),
  copyItem: (ids: number[], plain = false) => invoke<void>("copy_item", { ids, plain }),
  pinItem: (ids: number[], value: boolean) => invoke<void>("pin_item", { ids, value }),
  deleteItems: (ids: number[]) => invoke<number>("delete_items", { ids }),
  undoDelete: () => invoke<number[]>("undo_delete"),
  updateItemText: (id: number, text: string) => invoke<Item | null>("update_item_text", { id, text }),
  moveToSpace: (ids: number[], spaceId: number | null) => invoke<void>("move_to_space", { ids, spaceId }),
  setItemTags: (id: number, tagIds: number[]) => invoke<void>("set_item_tags", { id, tagIds }),
  runAction: (id: number, action: Action, mode: ActionMode) => invoke<ActionResult>("run_action", { id, action, mode }),
  listSpaces: () => invoke<Space[]>("list_spaces"),
  createSpace: (name: string, color?: string | null, icon?: string | null) =>
    invoke<number>("create_space", { name, color: color ?? null, icon: icon ?? null }),
  renameSpace: (id: number, name: string, color?: string | null, icon?: string | null) =>
    invoke<void>("rename_space", { id, name, color: color ?? null, icon: icon ?? null }),
  deleteSpace: (id: number) => invoke<void>("delete_space", { id }),
  reorderSpaces: (ids: number[]) => invoke<void>("reorder_spaces", { ids }),
  listTags: () => invoke<Tag[]>("list_tags"),
  createTag: (name: string, color?: string | null) => invoke<number>("create_tag", { name, color: color ?? null }),
  updateTag: (id: number, name: string, color?: string | null) => invoke<void>("update_tag", { id, name, color: color ?? null }),
  deleteTag: (id: number) => invoke<void>("delete_tag", { id }),
  mergeTags: (from: number[], into: number) => invoke<void>("merge_tags", { from, into }),
  listSourceApps: () => invoke<SourceAppInfo[]>("list_source_apps"),
  getSettings: () => invoke<Settings>("get_settings"),
  setSetting: <K extends keyof Settings>(key: K, value: Settings[K]) => invoke<Settings>("set_setting", { key, value }),
  setShortcut: (accelerator: string) => invoke<Settings>("set_shortcut", { accelerator }),
  checkAccessibility: () => invoke<boolean>("check_accessibility"),
  requestAccessibility: () => invoke<boolean>("request_accessibility"),
  pauseMonitoring: (value: boolean) => invoke<void>("pause_monitoring", { value }),
  getMonitoringState: () => invoke<boolean>("get_monitoring_state"),
  listExcludedApps: () => invoke<ExcludedApp[]>("list_excluded_apps"),
  addExcludedApp: (args: { bundleId?: string; path?: string; name?: string }) =>
    invoke<ExcludedApp>("add_excluded_app", { bundleId: args.bundleId ?? null, path: args.path ?? null, name: args.name ?? null }),
  removeExcludedApp: (bundleId: string) => invoke<void>("remove_excluded_app", { bundleId }),
  exportData: (path: string) => invoke<number>("export_data", { path }),
  importData: (path: string) => invoke<number>("import_data", { path }),
  clearHistory: (keepPinned: boolean) => invoke<number>("clear_history", { keepPinned }),
  hidePanel: () => invoke<void>("hide_panel"),
  showPanel: () => invoke<void>("show_panel"),
  openSettings: (section?: string) => invoke<void>("open_settings", { section: section ?? null }),
  finishOnboarding: () => invoke<void>("finish_onboarding"),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  revealInFinder: (id: number) => invoke<void>("reveal_in_finder", { id }),
  fileIcon: (path: string) => invoke<string | null>("file_icon", { path }),
  systemAccent: () => invoke<string | null>("system_accent"),
  appInfo: () => invoke<AppInfo>("app_info"),
};

export const emptyQuery = (): ItemQuery => ({
  text: "",
  kinds: [],
  apps: [],
  appNames: [],
  tags: [],
  spaceId: null,
  spaceNames: [],
  pinned: null,
  since: null,
  sort: "recent",
  offset: 0,
  limit: 60,
});
