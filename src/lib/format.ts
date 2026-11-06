import type { Lang } from "./i18n";

export function timeAgo(ms: number, lang: Lang, now = Date.now()): string {
  const s = Math.max(0, Math.round((now - ms) / 1000));
  const it = lang === "it";
  if (s < 45) return it ? "adesso" : "just now";
  const m = Math.round(s / 60);
  if (m < 60) return it ? `${m} min fa` : `${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 24) return it ? `${h} h fa` : `${h}h ago`;
  const d = Math.round(h / 24);
  if (d === 1) return it ? "ieri" : "yesterday";
  if (d < 30) return it ? `${d} giorni fa` : `${d} days ago`;
  return new Date(ms).toLocaleDateString(it ? "it-IT" : "en-US", { day: "numeric", month: "short", year: "numeric" });
}

export function bytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v < 10 ? v.toFixed(1) : Math.round(v)} ${units[i]}`;
}

export function domain(url: string): string {
  try {
    const u = new URL(url);
    return u.protocol === "mailto:" ? u.pathname : u.hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

export const basename = (p: string) => p.split("/").filter(Boolean).pop() ?? p;

// ---- colours -----------------------------------------------------------------------------------

export type RGBA = { r: number; g: number; b: number; a: number };

export function parseColor(input: string): RGBA | null {
  const s = input.trim().toLowerCase();
  let m = s.match(/^#([0-9a-f]{3,8})$/);
  if (m) {
    let h = m[1];
    if (h.length === 3 || h.length === 4) h = h.split("").map((c) => c + c).join("");
    if (h.length !== 6 && h.length !== 8) return null;
    const n = (i: number) => parseInt(h.slice(i, i + 2), 16);
    return { r: n(0), g: n(2), b: n(4), a: h.length === 8 ? n(6) / 255 : 1 };
  }
  m = s.match(/^(rgba?|hsla?)\((.+)\)$/);
  if (!m) return null;
  const parts = m[2].split(/[\s,/]+/).filter(Boolean);
  if (parts.length < 3) return null;
  const num = (p: string, max: number) => (p.endsWith("%") ? (parseFloat(p) / 100) * max : parseFloat(p));
  const alpha = parts[3] !== undefined ? num(parts[3], 1) : 1;
  if (m[1].startsWith("rgb")) {
    return { r: num(parts[0], 255), g: num(parts[1], 255), b: num(parts[2], 255), a: alpha };
  }
  const h = parseFloat(parts[0]);
  const sat = num(parts[1], 1);
  const l = num(parts[2], 1);
  const { r, g, b } = hslToRgb(h, sat, l);
  return { r, g, b, a: alpha };
}

export function hslToRgb(h: number, s: number, l: number) {
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return { r: Math.round(f(0) * 255), g: Math.round(f(8) * 255), b: Math.round(f(4) * 255) };
}

export function rgbToHsl({ r, g, b }: RGBA) {
  const rn = r / 255, gn = g / 255, bn = b / 255;
  const max = Math.max(rn, gn, bn), min = Math.min(rn, gn, bn);
  const l = (max + min) / 2;
  let h = 0, s = 0;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    h = max === rn ? (gn - bn) / d + (gn < bn ? 6 : 0) : max === gn ? (bn - rn) / d + 2 : (rn - gn) / d + 4;
    h *= 60;
  }
  return { h: Math.round(h), s: Math.round(s * 100), l: Math.round(l * 100) };
}

export function colorFormats(c: RGBA) {
  const hex2 = (n: number) => Math.round(n).toString(16).padStart(2, "0").toUpperCase();
  const hex = `#${hex2(c.r)}${hex2(c.g)}${hex2(c.b)}${c.a < 1 ? hex2(c.a * 255) : ""}`;
  const a = c.a < 1 ? `, ${+c.a.toFixed(2)}` : "";
  const { h, s, l } = rgbToHsl(c);
  return {
    hex,
    rgb: `rgb${a ? "a" : ""}(${Math.round(c.r)}, ${Math.round(c.g)}, ${Math.round(c.b)}${a})`,
    hsl: `hsl${a ? "a" : ""}(${h}, ${s}%, ${l}%${a})`,
  };
}

/** Relative luminance → whether white or black text reads better on `hex`. */
export function readableOn(hex: string | null | undefined): "light" | "dark" {
  const c = hex ? parseColor(hex) : null;
  if (!c) return "light";
  const lin = (v: number) => {
    const x = v / 255;
    return x <= 0.03928 ? x / 12.92 : Math.pow((x + 0.055) / 1.055, 2.4);
  };
  const L = 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
  return L > 0.45 ? "dark" : "light";
}

/** Accent-insensitive, case-insensitive ranges of `terms` inside `text` for highlighting. */
export function highlightRanges(text: string, terms: string[]): [number, number][] {
  const fold = (s: string) => s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
  // Folding can change length; map folded indices back to original indices.
  const map: number[] = [];
  let folded = "";
  for (let i = 0; i < text.length; i++) {
    const f = fold(text[i]);
    for (let j = 0; j < f.length; j++) map.push(i);
    folded += f;
  }
  const out: [number, number][] = [];
  for (const raw of terms) {
    const t = fold(raw.trim());
    if (t.length < 1) continue;
    let from = 0;
    while (true) {
      const i = folded.indexOf(t, from);
      if (i < 0) break;
      out.push([map[i], map[i + t.length - 1] + 1]);
      from = i + t.length;
    }
  }
  out.sort((a, b) => a[0] - b[0]);
  const merged: [number, number][] = [];
  for (const r of out) {
    const last = merged[merged.length - 1];
    if (last && r[0] <= last[1]) last[1] = Math.max(last[1], r[1]);
    else merged.push([...r]);
  }
  return merged;
}
