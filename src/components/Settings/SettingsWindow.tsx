import { useEffect, useState } from "react";
import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { Database, Info, Keyboard, Layers, Palette, Settings2, Shield, History as HistoryIcon, Trash2 } from "lucide-react";
import { ipc, type AppInfo, type ExcludedApp, type Space, type Tag } from "../../lib/ipc";
import { useSettings } from "../../store/settings";
import { useT } from "../../lib/i18n";
import { useTauriEvent } from "../../hooks/useTauriEvent";
import { Button, Field, NumberInput, Section, Segmented, Select, Toggle } from "./Controls";
import { ShortcutRecorder } from "./ShortcutRecorder";
import { SPACE_COLORS, SPACE_ICONS, SpaceIcon } from "../SpaceIcon";

const SECTIONS = [
  ["general", "set_general", Settings2],
  ["shortcuts", "set_shortcuts", Keyboard],
  ["appearance", "set_appearance", Palette],
  ["history", "set_history", HistoryIcon],
  ["privacy", "set_privacy", Shield],
  ["organize", "set_organize", Layers],
  ["data", "set_data", Database],
  ["about", "set_about", Info],
] as const;

type SectionId = (typeof SECTIONS)[number][0];

function useNotice() {
  const [msg, setMsg] = useState<{ text: string; error?: boolean } | null>(null);
  useEffect(() => {
    if (!msg) return;
    const id = window.setTimeout(() => setMsg(null), 3500);
    return () => window.clearTimeout(id);
  }, [msg]);
  return [msg, setMsg] as const;
}

export function SettingsWindow({ initial }: { initial: string }) {
  const t = useT();
  const [section, setSection] = useState<SectionId>((SECTIONS.find((s) => s[0] === initial)?.[0] ?? "general") as SectionId);
  useTauriEvent<string>("route", (r) => {
    const s = r.split("/")[2];
    if (SECTIONS.some((x) => x[0] === s)) setSection(s as SectionId);
  });
  const settings = useSettings((s) => s.settings);
  if (!settings) return null;
  return (
    <div className="flex h-full">
      <aside className="flex w-[196px] shrink-0 flex-col gap-0.5 px-2.5 pt-12" data-tauri-drag-region>
        {SECTIONS.map(([id, key, Icon]) => (
          <button
            key={id}
            onClick={() => setSection(id)}
            aria-current={section === id}
            className="flex h-8 items-center gap-2.5 rounded-[7px] px-2.5 text-left text-[13px]"
            style={{ background: section === id ? "var(--cs-surface-active)" : undefined, fontWeight: section === id ? 600 : 400 }}
          >
            <Icon size={15} strokeWidth={2} style={{ color: section === id ? "var(--cs-accent)" : "var(--cs-text-muted)" }} />
            {t(key)}
          </button>
        ))}
      </aside>
      <main className="cs-scroll flex-1 overflow-y-auto px-7 pb-8 pt-11" style={{ background: "var(--cs-surface)" }}>
        <h1 className="mb-4 text-[20px] font-bold" data-tauri-drag-region>
          {t(SECTIONS.find((s) => s[0] === section)![1])}
        </h1>
        {section === "general" && <General />}
        {section === "shortcuts" && <Shortcuts />}
        {section === "appearance" && <Appearance />}
        {section === "history" && <HistorySection />}
        {section === "privacy" && <Privacy />}
        {section === "organize" && <Organize />}
        {section === "data" && <Data />}
        {section === "about" && <About />}
      </main>
    </div>
  );
}

function General() {
  const t = useT();
  const { settings, set } = useSettings();
  const [err, setErr] = useState<string | null>(null);
  if (!settings) return null;
  return (
    <>
      <Section>
        <Field label={t("launchAtLogin")}>
          <Toggle label={t("launchAtLogin")} checked={settings.launchAtLogin} onChange={(v) => set("launchAtLogin", v).catch((e) => setErr(String(e)))} />
        </Field>
        <Field label={t("language")}>
          <Select label={t("language")} value={settings.language} onChange={(v) => set("language", v)} options={[["auto", t("langAuto")], ["it", "Italiano"], ["en", "English"]]} />
        </Field>
        <Field label={t("theme")}>
          <Segmented label={t("theme")} value={settings.theme} onChange={(v) => set("theme", v)} options={[["auto", t("themeAuto")], ["light", t("themeLight")], ["dark", t("themeDark")]]} />
        </Field>
        <Field label={t("soundOnCopy")}>
          <Toggle label={t("soundOnCopy")} checked={settings.soundOnCopy} onChange={(v) => set("soundOnCopy", v)} />
        </Field>
      </Section>
      {err && <p className="text-[12px]" style={{ color: "var(--cs-danger)" }}>{err}</p>}
      <Section hint={t("autoPasteHint")}>
        <Field label={t("autoPaste")}>
          <Toggle label={t("autoPaste")} checked={settings.autoPaste} onChange={(v) => set("autoPaste", v)} />
        </Field>
        <Field label={t("pasteStack")} hint={t("pasteStackHint")}>
          <Toggle label={t("pasteStack")} checked={settings.pasteStack} onChange={(v) => set("pasteStack", v)} />
        </Field>
        <Field label={t("moveToTop")}>
          <Toggle label={t("moveToTop")} checked={settings.moveToTopAfterPaste} onChange={(v) => set("moveToTopAfterPaste", v)} />
        </Field>
        <Field label={t("separator")}>
          <Select label={t("separator")} value={settings.multiSeparator} onChange={(v) => set("multiSeparator", v)} options={[["newline", t("sepNewline")], ["space", t("sepSpace")], ["comma", t("sepComma")], ["none", t("sepNone")]]} />
        </Field>
      </Section>
      <AccessibilityRow />
    </>
  );
}

function AccessibilityRow() {
  const t = useT();
  const [ok, setOk] = useState<boolean | null>(null);
  useEffect(() => {
    const check = () => ipc.checkAccessibility().then(setOk);
    void check();
    window.addEventListener("focus", check);
    return () => window.removeEventListener("focus", check);
  }, []);
  return (
    <Section>
      <Field label={t("accessibility")} hint={ok ? undefined : t("obAccessBody")}>
        <span className="text-[12px] font-medium" style={{ color: ok ? "#16a34a" : "#d97706" }}>
          {ok === null ? "…" : ok ? t("granted") : t("notGranted")}
        </span>
        {!ok && <Button onClick={() => ipc.requestAccessibility().then(setOk)}>{t("openSystemSettings")}</Button>}
      </Field>
    </Section>
  );
}

function Shortcuts() {
  const t = useT();
  const { settings, set } = useSettings();
  if (!settings) return null;
  const rows: [string, string][] = [
    ["← → ↑ ↓", "Naviga / Navigate"],
    ["↩", t("paste")],
    ["⇧↩", t("pastePlain")],
    ["⌘1…⌘9", `${t("paste")} 1–9`],
    ["⇧← ⇧→ / ⌘click", t("selected", { n: "N" })],
    ["Space", t("preview")],
    ["⌘F", "Search"],
    ["⎋", "Esc"],
    ["⌘D", `${t("pin")} / ${t("unpin")}`],
    ["⌘T", t("tagsAction")],
    ["⌘M", t("moveToSpace")],
    ["⌘E", t("edit")],
    ["⌘K", t("actions")],
    ["⌘⌫ / ⌘Z", `${t("delete")} / ${t("undo")}`],
    ["⇥ / ⇧⇥", t("spaces")],
    ["⌘B", t("toggleSidebar")],
    ["⌘,", t("settings")],
  ];
  return (
    <>
      <Section hint={settings.shortcut === "CommandOrControl+Shift+V" ? t("shortcutConflict") : undefined}>
        <Field label={t("openPanelShortcut")}>
          <ShortcutRecorder value={settings.shortcut} onChange={(v) => (v ? set("shortcut", v) : undefined)} />
        </Field>
        <Field label={t("pauseShortcut")}>
          <ShortcutRecorder value={settings.pauseShortcut} allowClear onChange={(v) => set("pauseShortcut", v)} />
        </Field>
      </Section>
      <Section title={t("inPanelShortcuts")}>
        {rows.map(([k, l]) => (
          <Field key={k} label={l}>
            <kbd className="rounded-[5px] px-1.5 py-0.5 text-[11.5px] font-semibold" style={{ background: "var(--cs-field)" }}>
              {k}
            </kbd>
          </Field>
        ))}
      </Section>
    </>
  );
}

function Appearance() {
  const t = useT();
  const { settings, set, systemAccent } = useSettings();
  if (!settings) return null;
  const accents = ["#6366F1", "#0EA5E9", "#10B981", "#F59E0B", "#EF4444", "#EC4899", "#8B5CF6", "#64748B"];
  return (
    <Section>
      <Field label={t("layout")}>
        <Segmented label={t("layout")} value={settings.layout} onChange={(v) => set("layout", v)} options={[["top", t("layoutTop")], ["bottom", t("layoutBottom")], ["center", t("layoutCenter")]]} />
      </Field>
      <Field label={t("cardSize")}>
        <Segmented label={t("cardSize")} value={settings.cardSize} onChange={(v) => set("cardSize", v)} options={[["small", t("sizeSmall")], ["medium", t("sizeMedium")], ["large", t("sizeLarge")]]} />
      </Field>
      <Field label={t("density")}>
        <Segmented label={t("density")} value={settings.density} onChange={(v) => set("density", v)} options={[["comfortable", t("densityComfortable")], ["compact", t("densityCompact")]]} />
      </Field>
      <Field label={t("accent")}>
        <div className="flex items-center gap-1.5" style={{ opacity: settings.useSystemAccent ? 0.35 : 1 }}>
          {accents.map((c) => (
            <button key={c} aria-label={c} disabled={settings.useSystemAccent} onClick={() => set("accent", c)} className="h-5 w-5 rounded-full" style={{ background: c, boxShadow: settings.accent.toUpperCase() === c ? `0 0 0 2px var(--cs-surface), 0 0 0 3.5px ${c}` : undefined }} />
          ))}
          <input aria-label={t("accent")} type="color" value={settings.accent} disabled={settings.useSystemAccent} onChange={(e) => set("accent", e.target.value.toUpperCase())} className="h-6 w-7 cursor-default rounded border-0 bg-transparent p-0" />
        </div>
      </Field>
      <Field label={t("useSystemAccent")} hint={systemAccent ?? undefined}>
        <Toggle label={t("useSystemAccent")} checked={settings.useSystemAccent} onChange={(v) => set("useSystemAccent", v)} />
      </Field>
    </Section>
  );
}

function HistorySection() {
  const t = useT();
  const { settings, set } = useSettings();
  if (!settings) return null;
  const MB = 1024 * 1024;
  return (
    <Section hint={t("neverExpireNote")}>
      <Field label={t("maxItems")}>
        <NumberInput label={t("maxItems")} value={settings.maxItems} min={50} max={100000} onChange={(v) => set("maxItems", v)} suffix={t("items")} />
      </Field>
      <Field label={t("retention")} hint={settings.retentionDays === 0 ? t("forever") : undefined}>
        <NumberInput label={t("retention")} value={settings.retentionDays} min={0} max={3650} onChange={(v) => set("retentionDays", v)} suffix={t("days")} />
      </Field>
      <Field label={t("maxText")}>
        <NumberInput label={t("maxText")} value={Math.round(settings.maxTextBytes / 1024)} min={16} max={20480} onChange={(v) => set("maxTextBytes", v * 1024)} suffix="KB" />
      </Field>
      <Field label={t("maxImage")}>
        <NumberInput label={t("maxImage")} value={Math.round(settings.maxImageBytes / MB)} min={1} max={200} onChange={(v) => set("maxImageBytes", v * MB)} suffix="MB" />
      </Field>
    </Section>
  );
}

function Privacy() {
  const t = useT();
  const { settings, set } = useSettings();
  const [apps, setApps] = useState<ExcludedApp[]>([]);
  const [paused, setPaused] = useState(false);
  const [notice, setNotice] = useNotice();
  const reload = () => ipc.listExcludedApps().then(setApps);
  useEffect(() => {
    void reload();
    ipc.getMonitoringState().then(setPaused);
  }, []);
  useTauriEvent<{ paused: boolean }>("monitoring:state", (p) => setPaused(p.paused));
  if (!settings) return null;
  const addApp = async () => {
    const path = await open({ directory: false, multiple: false, defaultPath: "/Applications", filters: [{ name: "Applications", extensions: ["app"] }] });
    if (typeof path === "string") {
      try {
        await ipc.addExcludedApp({ path });
        await reload();
      } catch (e) {
        setNotice({ text: String(e), error: true });
      }
    }
  };
  return (
    <>
      <Section>
        <Field label={t("pauseRecording")}>
          <Toggle label={t("pauseRecording")} checked={paused} onChange={(v) => ipc.pauseMonitoring(v).then(() => setPaused(v))} />
        </Field>
      </Section>
      <Section title={t("excludedApps")} hint={t("excludedHint")}>
        {apps.map((a) => (
          <Field key={a.bundleId} label={a.name ?? a.bundleId} hint={a.bundleId}>
            <Button onClick={() => ipc.removeExcludedApp(a.bundleId).then(reload)}>{t("remove")}</Button>
          </Field>
        ))}
        <Field label="">
          <Button onClick={addApp}>{t("addApp")}</Button>
        </Field>
      </Section>
      {notice && <p className="mb-3 text-[12px]" style={{ color: notice.error ? "var(--cs-danger)" : undefined }}>{notice.text}</p>}
      <Section title={t("sensitiveData")}>
        <Field label={t("sensitiveData")}>
          <Select label={t("sensitiveData")} value={settings.sensitivePolicy} onChange={(v) => set("sensitivePolicy", v)} options={[["skip", t("sensSkip")], ["expire", t("sensExpire")], ["keep", t("sensKeep")]]} />
          {settings.sensitivePolicy === "expire" && <NumberInput label={t("minutes")} value={settings.sensitiveExpireMinutes} min={1} max={1440} onChange={(v) => set("sensitiveExpireMinutes", v)} suffix={t("minutes")} />}
        </Field>
      </Section>
      <Section hint={t("networkHint")}>
        <Field label={t("network")}>
          <Toggle label={t("network")} checked={settings.fetchLinkMeta} onChange={(v) => set("fetchLinkMeta", v)} />
        </Field>
        <Field label={t("ocr")}>
          <Toggle label={t("ocr")} checked={settings.ocrEnabled} onChange={(v) => set("ocrEnabled", v)} />
        </Field>
        <Field label={t("encrypt")} hint={t("encryptHint")}>
          <Toggle label={t("encrypt")} checked={settings.encryptDb} onChange={(v) => set("encryptDb", v).catch((e) => setNotice({ text: String(e), error: true }))} />
        </Field>
      </Section>
      <p className="text-[11.5px]" style={{ color: "var(--cs-text-faint)" }}>{t("privacyNote")}</p>
    </>
  );
}

function Organize() {
  const t = useT();
  const [tags, setTags] = useState<Tag[]>([]);
  const [spaces, setSpaces] = useState<Space[]>([]);
  const reload = () => Promise.all([ipc.listTags().then(setTags), ipc.listSpaces().then(setSpaces)]);
  useEffect(() => {
    void reload();
  }, []);
  useTauriEvent("items:changed", () => void reload());
  const TAG_COLORS = ["#EF4444", "#F59E0B", "#10B981", "#0EA5E9", "#6366F1", "#EC4899", "#8B5CF6", "#64748B"];
  return (
    <>
      <Section title={t("manageTags")}>
        {!tags.length && <Field label={t("noTags")}>{null}</Field>}
        {tags.map((tg) => (
          <Field key={tg.id} label="">
            <div className="flex flex-1 items-center gap-2" style={{ marginRight: "auto" }}>
              <select aria-label={t("changeColor")} value={tg.color ?? ""} onChange={(e) => ipc.updateTag(tg.id, tg.name, e.target.value).then(reload)} className="h-6 w-6 appearance-none rounded-full text-transparent" style={{ background: tg.color ?? "var(--cs-accent)" }}>
                {TAG_COLORS.map((c) => (
                  <option key={c} value={c} style={{ color: "#000" }}>
                    {c}
                  </option>
                ))}
              </select>
              <input defaultValue={tg.name} aria-label={t("rename")} onBlur={(e) => e.target.value.trim() && e.target.value !== tg.name && ipc.updateTag(tg.id, e.target.value, tg.color).then(reload)} className="h-7 w-44 rounded-[7px] bg-transparent px-2 text-[13px] outline-none focus:bg-[var(--cs-field)]" />
              <span className="text-[11.5px]" style={{ color: "var(--cs-text-faint)" }}>{tg.count}</span>
            </div>
            {tags.length > 1 && (
              <Select
                label={t("mergeInto")}
                value={"" as string}
                onChange={(v) => v && ipc.mergeTags([tg.id], Number(v)).then(reload)}
                options={[["", t("mergeInto")], ...tags.filter((x) => x.id !== tg.id).map((x) => [String(x.id), x.name] as [string, string])]}
              />
            )}
            <button aria-label={t("delete")} onClick={() => ipc.deleteTag(tg.id).then(reload)} className="rounded p-1" style={{ color: "var(--cs-danger)" }}>
              <Trash2 size={14} />
            </button>
          </Field>
        ))}
      </Section>
      <Section title={t("manageSpaces")}>
        {spaces.map((s, i) => (
          <Field key={s.id} label="">
            <div className="flex flex-1 items-center gap-2">
              <span style={{ color: s.color ?? undefined }}><SpaceIcon name={s.icon} size={15} /></span>
              <input defaultValue={s.name} aria-label={t("rename")} onBlur={(e) => e.target.value.trim() && e.target.value !== s.name && ipc.renameSpace(s.id, e.target.value, s.color, s.icon).then(reload)} className="h-7 w-44 rounded-[7px] bg-transparent px-2 text-[13px] outline-none focus:bg-[var(--cs-field)]" />
              <span className="text-[11.5px]" style={{ color: "var(--cs-text-faint)" }}>{s.count}</span>
            </div>
            <Select label={t("changeColor")} value={s.color ?? SPACE_COLORS[0]} onChange={(c) => ipc.renameSpace(s.id, s.name, c, s.icon).then(reload)} options={SPACE_COLORS.map((c) => [c, c] as [string, string])} />
            <Select label="Icon" value={s.icon ?? "folder"} onChange={(ic) => ipc.renameSpace(s.id, s.name, s.color, ic).then(reload)} options={Object.keys(SPACE_ICONS).map((k) => [k, k] as [string, string])} />
            <button aria-label="↑" disabled={i === 0} onClick={() => { const ids = spaces.map((x) => x.id); [ids[i - 1], ids[i]] = [ids[i], ids[i - 1]]; void ipc.reorderSpaces(ids).then(reload); }} className="px-1 disabled:opacity-30">↑</button>
            <button aria-label="↓" disabled={i === spaces.length - 1} onClick={() => { const ids = spaces.map((x) => x.id); [ids[i + 1], ids[i]] = [ids[i], ids[i + 1]]; void ipc.reorderSpaces(ids).then(reload); }} className="px-1 disabled:opacity-30">↓</button>
            <button aria-label={t("delete")} onClick={async () => (await ask(t("deleteSpaceConfirm", { name: s.name }), { kind: "warning" })) && ipc.deleteSpace(s.id).then(reload)} className="rounded p-1" style={{ color: "var(--cs-danger)" }}>
              <Trash2 size={14} />
            </button>
          </Field>
        ))}
        <Field label="">
          <Button onClick={() => ipc.createSpace(t("newSpace"), SPACE_COLORS[spaces.length % SPACE_COLORS.length], "folder").then(reload)}>+ {t("newSpace")}</Button>
        </Field>
      </Section>
    </>
  );
}

function Data() {
  const t = useT();
  const [keepPinned, setKeepPinned] = useState(true);
  const [notice, setNotice] = useNotice();
  const [busy, setBusy] = useState(false);
  const run = async (fn: () => Promise<string>) => {
    setBusy(true);
    try {
      setNotice({ text: await fn() });
    } catch (e) {
      setNotice({ text: String(e), error: true });
    } finally {
      setBusy(false);
    }
  };
  const stamp = new Date().toISOString().slice(0, 10);
  return (
    <>
      <Section>
        <Field label={t("export")}>
          <Button disabled={busy} onClick={() => run(async () => {
            const path = await save({ defaultPath: `Clipstack-${stamp}.zip`, filters: [{ name: "Clipstack backup", extensions: ["zip"] }] });
            if (!path) return "";
            return t("exportDone", { n: await ipc.exportData(path) });
          })}>{t("export")}</Button>
        </Field>
        <Field label={t("import")}>
          <Button disabled={busy} onClick={() => run(async () => {
            const path = await open({ multiple: false, filters: [{ name: "Clipstack backup", extensions: ["zip"] }] });
            if (typeof path !== "string") return "";
            return t("importDone", { n: await ipc.importData(path) });
          })}>{t("import")}</Button>
        </Field>
      </Section>
      <Section>
        <Field label={t("keepPinned")}>
          <Toggle label={t("keepPinned")} checked={keepPinned} onChange={setKeepPinned} />
        </Field>
        <Field label={t("clearHistory")}>
          <Button tone="danger" disabled={busy} onClick={async () => (await ask(t("clearConfirm"), { kind: "warning" })) && run(async () => t("clearDone", { n: await ipc.clearHistory(keepPinned) }))}>{t("clearHistory")}</Button>
        </Field>
      </Section>
      {notice?.text && <p className="text-[12px]" style={{ color: notice.error ? "var(--cs-danger)" : "#16a34a" }}>{notice.text}</p>}
    </>
  );
}

function About() {
  const t = useT();
  const [info, setInfo] = useState<AppInfo | null>(null);
  useEffect(() => {
    ipc.appInfo().then(setInfo);
  }, []);
  return (
    <>
      <div className="mb-5 flex items-center gap-4">
        <img src="/app-icon.png" alt="" className="h-16 w-16" />
        <div>
          <div className="text-[18px] font-bold">Clipstack</div>
          <div className="text-[12.5px]" style={{ color: "var(--cs-text-muted)" }}>
            {t("version")} {info?.version}
          </div>
        </div>
      </div>
      <Section>
        <Field label={t("dataLocation")} hint={info?.dataDir}>{null}</Field>
        <Field label={t("encrypt")}>
          <span className="text-[12px]">{info?.encrypted ? "✓" : "—"}</span>
        </Field>
        <Field label={t("accessibility")}>
          <span className="text-[12px]">{info?.accessibility ? t("granted") : t("notGranted")}</span>
        </Field>
      </Section>
      <Section title={t("licenses")}>
        <div className="p-3 text-[12px] leading-relaxed" style={{ color: "var(--cs-text-muted)" }}>{t("licensesText")}</div>
      </Section>
      <p className="text-[11.5px]" style={{ color: "var(--cs-text-faint)" }}>{t("privacyNote")}</p>
    </>
  );
}

