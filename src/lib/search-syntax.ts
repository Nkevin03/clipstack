import type { ItemQuery, Kind } from "./ipc";

export type FilterKey = "type" | "app" | "tag" | "space" | "date" | "pinned";

export type Chip = { key: FilterKey; value: string };

const KEY_ALIASES: Record<string, FilterKey> = {
  type: "type", tipo: "type", kind: "type", is: "type",
  app: "app", from: "app", da: "app",
  tag: "tag", "#": "tag",
  space: "space", spazio: "space", in: "space",
  date: "date", data: "date", when: "date", quando: "date",
  pinned: "pinned", fissato: "pinned", pin: "pinned",
};

const KIND_ALIASES: Record<string, Kind> = {
  text: "text", testo: "text", txt: "text",
  rich: "rich", rtf: "rich", formattato: "rich",
  link: "link", links: "link", url: "link",
  image: "image", images: "image", img: "image", immagine: "image", immagini: "image", foto: "image",
  file: "file", files: "file",
  color: "color", colors: "color", colour: "color", colore: "color", colori: "color",
  code: "code", codice: "code",
};

const TOKEN = /(^|\s)([a-zA-Z#]+):("([^"]*)"|(\S+))(?=\s|$)/g;

export function normalizeChip(rawKey: string, value: string): Chip | null {
  const key = KEY_ALIASES[rawKey.toLowerCase()];
  if (!key || !value) return null;
  const v = value.trim();
  if (key === "type") {
    const k = KIND_ALIASES[v.toLowerCase()];
    return k ? { key, value: k } : null;
  }
  if (key === "pinned") {
    const yes = ["yes", "y", "si", "sì", "true", "1"].includes(v.toLowerCase());
    const no = ["no", "n", "false", "0"].includes(v.toLowerCase());
    return yes || no ? { key, value: yes ? "yes" : "no" } : null;
  }
  if (key === "date") return parseDate(v, Date.now()) !== null ? { key, value: v.toLowerCase() } : null;
  return { key, value: v };
}

/**
 * Split the search input into free text and filter chips.
 * When `final` is false, only `key:value` tokens followed by whitespace are extracted, so the user
 * can keep typing a value; when true (Enter / blur) every recognised token is extracted.
 */
export function extractFilters(input: string, final: boolean): { text: string; chips: Chip[] } {
  const chips: Chip[] = [];
  const text = input.replace(TOKEN, (match, lead: string, key: string, _v, quoted: string | undefined, bare: string | undefined, offset: number, whole: string) => {
    const end = offset + match.length;
    const complete = final || end < whole.length;
    if (!complete) return match;
    const chip = normalizeChip(key, quoted ?? bare ?? "");
    if (!chip) return match;
    chips.push(chip);
    return lead;
  });
  return { text: final ? text.replace(/\s+/g, " ").trim() : text.replace(/^\s+/, ""), chips };
}

/** `today`, `yesterday`, `7d`, `24h`, `2w`, `3m` (and Italian aliases) → unix ms lower bound. */
export function parseDate(value: string, now: number): number | null {
  const v = value.toLowerCase();
  const startOfDay = (t: number) => {
    const d = new Date(t);
    d.setHours(0, 0, 0, 0);
    return d.getTime();
  };
  if (v === "today" || v === "oggi") return startOfDay(now);
  if (v === "yesterday" || v === "ieri") return startOfDay(now) - 86_400_000;
  const m = v.match(/^(\d+)\s*(h|d|g|w|s|m)$/);
  if (!m) return null;
  const n = parseInt(m[1], 10);
  const unit = { h: 3_600_000, d: 86_400_000, g: 86_400_000, w: 7 * 86_400_000, s: 7 * 86_400_000, m: 30 * 86_400_000 }[m[2]]!;
  return now - n * unit;
}

/** Merge chips (from the search bar and from UI filters) into a backend query. */
export function applyChips(base: ItemQuery, text: string, chips: Chip[], now = Date.now()): ItemQuery {
  const q: ItemQuery = { ...base, text, kinds: [...base.kinds], appNames: [...base.appNames], tags: [...base.tags], spaceNames: [...base.spaceNames] };
  for (const c of chips) {
    switch (c.key) {
      case "type":
        if (!q.kinds.includes(c.value as Kind)) q.kinds.push(c.value as Kind);
        break;
      case "app":
        q.appNames.push(c.value);
        break;
      case "tag":
        q.tags.push(c.value);
        break;
      case "space":
        q.spaceNames.push(c.value);
        break;
      case "date": {
        const since = parseDate(c.value, now);
        if (since !== null) q.since = q.since === null ? since : Math.max(q.since, since);
        break;
      }
      case "pinned":
        q.pinned = c.value === "yes";
        break;
    }
  }
  return q;
}

export const chipLabel = (c: Chip) => `${c.key}:${c.value.includes(" ") ? `"${c.value}"` : c.value}`;
