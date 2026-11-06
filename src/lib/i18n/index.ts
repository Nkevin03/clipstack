import { en, type Strings } from "./en";
import { it } from "./it";
import { useSettings } from "../../store/settings";

export type Lang = "it" | "en";
export type StringKey = keyof Strings;

const catalogs: Record<Lang, Strings> = { en, it };

export function resolveLang(setting: string | undefined, system?: string): Lang {
  if (setting === "it" || setting === "en") return setting;
  const sys = system ?? (typeof navigator !== "undefined" ? navigator.language : "en");
  return sys.toLowerCase().startsWith("it") ? "it" : "en";
}

export function translate(lang: Lang, key: StringKey, vars?: Record<string, string | number>): string {
  let s: string = catalogs[lang][key] ?? en[key] ?? key;
  if (vars) for (const [k, v] of Object.entries(vars)) s = s.split(`{${k}}`).join(String(v));
  return s;
}

export function useLang(): Lang {
  const language = useSettings((s) => s.settings?.language);
  return resolveLang(language);
}

export function useT() {
  const lang = useLang();
  return (key: StringKey, vars?: Record<string, string | number>) => translate(lang, key, vars);
}
