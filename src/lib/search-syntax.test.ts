import { describe, expect, it } from "vitest";
import { applyChips, extractFilters, parseDate } from "./search-syntax";
import { emptyQuery } from "./ipc";

describe("extractFilters", () => {
  it("extracts complete tokens while typing", () => {
    expect(extractFilters("type:image ", false)).toEqual({ text: "", chips: [{ key: "type", value: "image" }] });
    expect(extractFilters("hello type:ima", false)).toEqual({ text: "hello type:ima", chips: [] });
    expect(extractFilters("hello type:image", true)).toEqual({ text: "hello", chips: [{ key: "type", value: "image" }] });
  });

  it("supports aliases, quotes and multiple filters", () => {
    const r = extractFilters('tipo:immagine app:safari tag:"da leggere" foo', true);
    expect(r.text).toBe("foo");
    expect(r.chips).toEqual([
      { key: "type", value: "image" },
      { key: "app", value: "safari" },
      { key: "tag", value: "da leggere" },
    ]);
  });

  it("leaves unknown keys and invalid values as text", () => {
    expect(extractFilters("http://x.com type:banana note:x", true)).toEqual({ text: "http://x.com type:banana note:x", chips: [] });
  });

  it("pinned and date", () => {
    expect(extractFilters("pinned:yes date:7d", true).chips).toEqual([
      { key: "pinned", value: "yes" },
      { key: "date", value: "7d" },
    ]);
    expect(extractFilters("date:sometime", true).chips).toEqual([]);
  });
});

describe("parseDate", () => {
  const now = new Date(2026, 10, 6, 15, 30).getTime();
  it("handles relative units", () => {
    expect(parseDate("7d", now)).toBe(now - 7 * 86_400_000);
    expect(parseDate("24h", now)).toBe(now - 24 * 3_600_000);
    expect(parseDate("today", now)).toBe(new Date(2026, 10, 6).getTime());
    expect(parseDate("ieri", now)).toBe(new Date(2026, 10, 5).getTime());
    expect(parseDate("nope", now)).toBeNull();
  });
});

describe("applyChips", () => {
  it("builds a backend query", () => {
    const now = 1_000_000_000_000;
    const q = applyChips(emptyQuery(), "abc", [
      { key: "type", value: "link" },
      { key: "app", value: "chrome" },
      { key: "tag", value: "work" },
      { key: "space", value: "snippets" },
      { key: "pinned", value: "no" },
      { key: "date", value: "1d" },
    ], now);
    expect(q).toMatchObject({ text: "abc", kinds: ["link"], appNames: ["chrome"], tags: ["work"], spaceNames: ["snippets"], pinned: false, since: now - 86_400_000 });
  });
});
