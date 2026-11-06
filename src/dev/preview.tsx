/**
 * Development-only design preview: renders the real UI with a mocked Tauri backend so it can be
 * inspected in a normal browser (`pnpm dev` → http://localhost:1420/preview.html?layout=top&theme=dark).
 * Not part of the production build (only index.html is bundled).
 */
import React from "react";
import ReactDOM from "react-dom/client";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Item, Settings, Space, Tag, SourceAppInfo } from "../lib/ipc";
import "../styles/index.css";

const params = new URLSearchParams(location.search);
const now = Date.now();
const svgIcon = (bg: string, glyph: string) =>
  `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect x="4" y="4" width="56" height="56" rx="14" fill="${bg}"/><text x="32" y="42" font-size="28" text-anchor="middle" fill="white" font-family="-apple-system">${glyph}</text></svg>`)}`;
const photo = `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="400" height="260"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#f6d365"/><stop offset="1" stop-color="#fda085"/></linearGradient></defs><rect width="400" height="260" fill="url(#g)"/><circle cx="300" cy="80" r="40" fill="#fff8"/><path d="M0 220 L120 120 L200 190 L280 140 L400 230 V260 H0Z" fill="#7a4e3a"/></svg>`)}`;

const apps: Record<string, { name: string; color: string; icon: string }> = {
  "com.apple.Safari": { name: "Safari", color: "#1E88E5", icon: svgIcon("#1E88E5", "S") },
  "com.microsoft.VSCode": { name: "Code", color: "#2F2F3A", icon: svgIcon("#007ACC", "{}") },
  "com.apple.Notes": { name: "Note", color: "#F5C543", icon: svgIcon("#F5C543", "N") },
  "com.figma.Desktop": { name: "Figma", color: "#A259FF", icon: svgIcon("#A259FF", "F") },
  "com.apple.finder": { name: "Finder", color: "#3FA2F7", icon: svgIcon("#3FA2F7", "☺") },
  "com.tinyspeck.slackmacgap": { name: "Slack", color: "#4A154B", icon: svgIcon("#4A154B", "#") },
};

let id = 0;
const mk = (p: Partial<Item> & { kind: Item["kind"]; app: string; ago: number }): Item => {
  const a = apps[p.app];
  const preview = p.preview ?? "";
  return {
    id: ++id,
    preview,
    charCount: preview.length,
    hasRich: false,
    filePaths: [],
    imagePath: null,
    thumbPath: null,
    width: null,
    height: null,
    byteSize: preview.length,
    sourceBundleId: p.app,
    sourceAppName: a.name,
    sourceColor: a.color,
    sourceIconPath: a.icon,
    linkTitle: null,
    linkFaviconPath: null,
    language: null,
    pinned: false,
    spaceId: null,
    useCount: 0,
    createdAt: now - p.ago,
    lastUsedAt: now - p.ago,
    expiresAt: null,
    tagIds: [],
    ...p,
  } as Item;
};

const items: Item[] = [
  mk({ kind: "text", app: "com.apple.Notes", ago: 60_000, preview: "Ciao Marco,\nti confermo la riunione di giovedì alle 10:30 in sala Volta.\nPorto io la presentazione aggiornata con i numeri del Q3.\n\nA presto!", tagIds: [1] }),
  mk({ kind: "link", app: "com.apple.Safari", ago: 5 * 60_000, preview: "https://developer.apple.com/documentation/appkit/nspanel", linkTitle: "NSPanel | Apple Developer Documentation", pinned: true }),
  mk({ kind: "code", app: "com.microsoft.VSCode", ago: 12 * 60_000, language: "rust", preview: "fn main() {\n    let items = load_history()?;\n    for item in items.iter().take(5) {\n        println!(\"{}\", item.title);\n    }\n    Ok(())\n}", tagIds: [2] }),
  mk({ kind: "image", app: "com.figma.Desktop", ago: 40 * 60_000, imagePath: photo, thumbPath: photo, width: 1600, height: 1040, byteSize: 812_000 }),
  mk({ kind: "color", app: "com.figma.Desktop", ago: 2 * 3_600_000, preview: "#6366F1" }),
  mk({ kind: "file", app: "com.apple.finder", ago: 3 * 3_600_000, filePaths: ["/Users/kevin/Documents/Preventivo 2026.pdf", "/Users/kevin/Documents/Logo.png"], preview: "", byteSize: 2_400_000 }),
  mk({ kind: "rich", app: "com.tinyspeck.slackmacgap", ago: 26 * 3_600_000, preview: "Release checklist: build universal, notarize, upload the DMG and update the changelog.", hasRich: false, spaceId: 1 }),
  mk({ kind: "text", app: "com.apple.Safari", ago: 3 * 86_400_000, preview: "Via Giuseppe Garibaldi 12, 20121 Milano MI" }),
  mk({ kind: "code", app: "com.microsoft.VSCode", ago: 4 * 86_400_000, language: "json", preview: '{\n  "name": "clipstack",\n  "version": "0.1.0",\n  "private": true\n}' }),
];

const spaces: Space[] = [
  { id: 1, name: "Snippets", color: "#10B981", icon: "code", position: 0, count: 4 },
  { id: 2, name: "Presentazione", color: "#F59E0B", icon: "slides", position: 1, count: 7 },
  { id: 3, name: "Da leggere", color: "#EC4899", icon: "read", position: 2, count: 12 },
];
const tags: Tag[] = [
  { id: 1, name: "lavoro", color: "#0EA5E9", count: 9 },
  { id: 2, name: "rust", color: "#F59E0B", count: 3 },
];
const sourceApps: SourceAppInfo[] = Object.entries(apps).map(([bundleId, a], i) => ({ bundleId, name: a.name, color: a.color, iconPath: a.icon, count: 10 - i, lastSeen: now - i * 1000 }));
const settings: Settings = {
  shortcut: "CommandOrControl+Shift+V", pauseShortcut: null, sequenceShortcut: "CommandOrControl+Alt+Shift+C", sequenceOrder: params.get("order") ?? "lastFirst", sequenceCycle: params.get("cycle") === "1", pastePeek: true, pastePeekCount: 3, layout: (params.get("layout") as Settings["layout"]) ?? "top",
  theme: params.get("theme") ?? "auto", language: params.get("lang") ?? "it", accent: "#6366F1", useSystemAccent: false,
  cardSize: params.get("size") ?? "medium", density: "comfortable", soundOnCopy: false, launchAtLogin: false, autoPaste: true,
  moveToTopAfterPaste: true, multiSeparator: "newline", maxTextBytes: 1048576, maxImageBytes: 26214400, maxItems: 5000,
  retentionDays: 90, sensitivePolicy: "expire", sensitiveExpireMinutes: 10, fetchLinkMeta: false, ocrEnabled: true,
  encryptDb: false, onboardingDone: true, sidebarCollapsed: params.get("sidebar") === "0",
};

const win = params.get("window") ?? "panel";
if (win === "hud") document.body.style.background = "#9aa3b5";
mockWindows(win);
mockIPC(
  (cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case "get_items": {
        const q = a.query as { text: string; kinds: string[] };
        return items.filter((i) => (!q.text || i.preview.toLowerCase().includes(q.text.toLowerCase())) && (!q.kinds.length || q.kinds.includes(i.kind)));
      }
      case "get_item": {
        const it = items.find((i) => i.id === a.id);
        return it ? { item: it, text: it.preview, html: null, ocrText: null } : null;
      }
      case "get_counts": return { all: 1243, pinned: 18 };
      case "list_spaces": return spaces;
      case "list_tags": return tags;
      case "list_source_apps": return sourceApps;
      case "get_settings": return settings;
      case "system_accent": return "#007AFF";
      case "get_monitoring_state": return params.get("paused") === "1";
      case "check_accessibility": return params.get("ax") !== "0";
      case "list_excluded_apps": return [{ bundleId: "com.1password.1password", name: "1Password" }, { bundleId: "com.bitwarden.desktop", name: "Bitwarden" }];
      case "app_info": return { version: "0.1.0", dataDir: "~/Library/Application Support/com.clipstack.app", encrypted: false, accessibility: true, systemLanguage: "it" };
      case "file_icon": return null;
      case "peek_state": return { visible: params.get("peek") === "1", items: items.slice(0, Number(params.get("n") ?? 3)) };
      case "sequence_state": {
        const n = Number(params.get("seq") ?? 3);
        return { items: n < 0 ? [] : [items[0], items[4], items[3], items[2], items[7]].slice(0, n), pasted: 0, finished: params.get("seqdone") === "1", accessibility: params.get("ax") !== "0", cycle: params.get("cycle") === "1", lastFirst: params.get("order") !== "firstFirst", active: params.get("peek") !== "1" };
      }
      case "run_action": return { text: String(items[2].preview).toUpperCase(), stats: { chars: 120, charsNoSpaces: 98, words: 20, lines: 7 }, newItemId: null };
      default: return null;
    }
  },
  { shouldMockEvents: true },
);
(window as unknown as { __TAURI_INTERNALS__: { convertFileSrc: (p: string) => string } }).__TAURI_INTERNALS__.convertFileSrc = (p) => p;

const { default: App } = await import("../App");
const { useUi } = await import("../store/ui");
const { useItems } = await import("../store/items");
if (win === "settings") location.hash = `/settings/${params.get("section") ?? "general"}`;
if (win === "onboarding") location.hash = "/onboarding";
if (win === "hud") location.hash = "/hud";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <div style={{ position: win === "hud" ? "relative" : "fixed", width: win === "hud" ? 300 : undefined, margin: win === "hud" ? 12 : undefined, inset: win === "panel" ? 8 : 0, background: win === "panel" ? "rgb(40 40 46 / 0.25)" : undefined, backdropFilter: "blur(30px)", borderRadius: 16 }}>
      <App />
    </div>
  </React.StrictMode>,
);

const modal = params.get("modal");
if (modal) {
  setTimeout(() => {
    if (modal === "context") useUi.getState().setContextMenu({ x: 420, y: 120, id: 1 });
    else if (modal === "tags") useUi.getState().openModal({ type: "tags", id: 1 });
    else if (modal === "actions") useUi.getState().openModal({ type: "actions", id: 3 });
    else if (modal === "preview") useUi.getState().openModal({ type: "preview", id: 3 });
    else if (modal === "toast") useUi.getState().toast({ text: "Elemento eliminato", action: { label: "Annulla", run: () => {} } }, 60000);
  }, 600);
}
if (params.get("search")) setTimeout(() => useItems.getState().setInput(params.get("search")!), 500);
if (params.get("multi")) setTimeout(() => useItems.getState().move(2, true), 500);
