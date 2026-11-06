import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Panel } from "./components/Panel";
import { SettingsWindow } from "./components/Settings/SettingsWindow";
import { Onboarding } from "./components/Onboarding/Onboarding";
import { Hud } from "./components/Hud/Hud";
import { accentOf, useSettings } from "./store/settings";
import { useItems } from "./store/items";
import { useUi } from "./store/ui";
import { useTauriEvent } from "./hooks/useTauriEvent";
import { ipc, type Settings } from "./lib/ipc";
import { resolveLang } from "./lib/i18n";
import { readableOn } from "./lib/format";

function useRoute() {
  const [hash, setHash] = useState(window.location.hash);
  useEffect(() => {
    const on = () => setHash(window.location.hash);
    window.addEventListener("hashchange", on);
    return () => window.removeEventListener("hashchange", on);
  }, []);
  return hash.replace(/^#/, "");
}

/** Theme, accent, density and card size → <html> attributes / CSS variables. */
function useAppearance() {
  const settings = useSettings((s) => s.settings);
  const systemAccent = useSettings((s) => s.systemAccent);
  const [dark, setDark] = useState(() => window.matchMedia("(prefers-color-scheme: dark)").matches);
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const on = () => setDark(mq.matches);
    mq.addEventListener("change", on);
    return () => mq.removeEventListener("change", on);
  }, []);
  useEffect(() => {
    const root = document.documentElement;
    const theme = settings?.theme === "light" || settings?.theme === "dark" ? settings.theme : dark ? "dark" : "light";
    root.dataset.theme = theme;
    root.dataset.size = settings?.cardSize ?? "medium";
    root.dataset.density = settings?.density ?? "comfortable";
    const accent = accentOf(settings, systemAccent);
    root.style.setProperty("--cs-accent", accent);
    root.style.setProperty("--cs-accent-text", readableOn(accent) === "dark" ? "#111" : "#fff");
    root.lang = resolveLang(settings?.language);
  }, [settings, systemAccent, dark]);
}

export default function App() {
  const route = useRoute();
  const load = useSettings((s) => s.load);
  const replace = useSettings((s) => s.replace);
  const ready = useSettings((s) => s.settings !== null);
  const label = getCurrentWindow().label;
  useAppearance();

  useEffect(() => {
    void load();
    if (label === "panel") {
      const st = useItems.getState();
      void st.refresh();
      void st.loadMeta();
      ipc.getMonitoringState().then((p) => useUi.getState().setPaused(p));
      ipc.checkAccessibility().then((v) => useUi.getState().setAccessibility(v));
    }
  }, [load, label]);

  useTauriEvent<Settings>("settings:changed", (s) => {
    replace(s);
    if (label === "panel") void useItems.getState().loadMeta();
  });
  useTauriEvent("clipboard:new-item", () => {
    if (label === "panel") {
      void useItems.getState().refresh();
      void useItems.getState().loadMeta();
    }
  });
  useTauriEvent("items:changed", () => {
    if (label === "panel") {
      void useItems.getState().refresh();
      void useItems.getState().loadMeta();
    }
  });
  useTauriEvent<{ paused: boolean }>("monitoring:state", (p) => useUi.getState().setPaused(p.paused));
  useTauriEvent("paste:needs-permission", () => useUi.getState().setAccessibility(false));

  if (!ready) return null;
  if (label === "hud" || route.startsWith("/hud")) return <Hud />;
  if (label === "settings" || route.startsWith("/settings")) return <SettingsWindow initial={route.split("/")[2] ?? "general"} />;
  if (label === "onboarding" || route.startsWith("/onboarding")) return <Onboarding />;
  return <Panel />;
}
