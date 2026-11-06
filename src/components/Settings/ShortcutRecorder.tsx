import { useEffect, useState } from "react";
import { useT } from "../../lib/i18n";

const SYMBOL: Record<string, string> = { CommandOrControl: "⌘", Command: "⌘", Super: "⌘", Shift: "⇧", Alt: "⌥", Option: "⌥", Control: "⌃", Space: "Space", Enter: "↩", Backspace: "⌫", Escape: "⎋" };

export function prettyShortcut(acc: string | null | undefined): string {
  if (!acc) return "—";
  const parts = acc.split("+");
  const key = parts[parts.length - 1];
  const mods = new Set(parts.slice(0, -1).map((p) => SYMBOL[p] ?? p));
  // Apple order: ⌃ ⌥ ⇧ ⌘
  const ordered = ["⌃", "⌥", "⇧", "⌘"].filter((m) => mods.has(m)).join("");
  const k = SYMBOL[key] ?? (key.startsWith("Key") ? key.slice(3) : key.startsWith("Digit") ? key.slice(5) : key);
  return ordered + k;
}

/** KeyboardEvent → Tauri accelerator (`CommandOrControl+Shift+V`). Requires a modifier. */
export function toAccelerator(e: KeyboardEvent): string | null {
  const mods: string[] = [];
  if (e.metaKey) mods.push("CommandOrControl");
  if (e.ctrlKey) mods.push("Control");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  const code = e.code;
  if (["MetaLeft", "MetaRight", "ShiftLeft", "ShiftRight", "AltLeft", "AltRight", "ControlLeft", "ControlRight"].includes(code)) return null;
  let key: string | null = null;
  if (/^Key[A-Z]$/.test(code)) key = code.slice(3);
  else if (/^Digit[0-9]$/.test(code)) key = code.slice(5);
  else if (/^F([1-9]|1[0-9]|20)$/.test(code)) key = code;
  else if (code === "Space") key = "Space";
  else if (["Backquote", "Minus", "Equal", "BracketLeft", "BracketRight", "Backslash", "Semicolon", "Quote", "Comma", "Period", "Slash"].includes(code)) key = code;
  if (!key) return null;
  if (!mods.length && !key.startsWith("F")) return null;
  return [...mods, key].join("+");
}

export function ShortcutRecorder({ value, onChange, allowClear }: { value: string | null; onChange: (v: string | null) => Promise<unknown> | void; allowClear?: boolean }) {
  const t = useT();
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!recording) return;
    const onKey = async (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape" && !e.metaKey) {
        setRecording(false);
        return;
      }
      const acc = toAccelerator(e);
      if (!acc) return;
      setRecording(false);
      try {
        await onChange(acc);
        setError(null);
      } catch (err) {
        setError(String(err));
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onChange]);
  return (
    <div className="flex flex-col items-end gap-1">
      <div className="flex items-center gap-1.5">
        <button
          onClick={() => setRecording(true)}
          className="h-7 min-w-[110px] rounded-[7px] px-3 text-[13px] font-semibold tracking-wide"
          style={{ background: recording ? "var(--cs-accent-soft)" : "var(--cs-field)", boxShadow: recording ? "inset 0 0 0 1.5px var(--cs-accent)" : "inset 0 0 0 0.5px var(--cs-border-strong)" }}
          aria-label={t("recordShortcut")}
        >
          {recording ? <span className="text-[12px] font-normal">{t("pressKeys")}</span> : prettyShortcut(value)}
        </button>
        {allowClear && value && (
          <button onClick={() => onChange(null)} className="text-[11.5px]" style={{ color: "var(--cs-text-muted)" }}>
            {t("clearShortcut")}
          </button>
        )}
      </div>
      {error && <div className="max-w-[260px] text-right text-[11px]" style={{ color: "var(--cs-danger)" }}>{error}</div>}
    </div>
  );
}
