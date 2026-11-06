import { describe, expect, it } from "vitest";
import { colorFormats, highlightRanges, parseColor, readableOn, timeAgo, bytes } from "./format";

describe("colors", () => {
  it("parses and converts", () => {
    expect(parseColor("#6366F1")).toEqual({ r: 99, g: 102, b: 241, a: 1 });
    expect(parseColor("#fff")).toEqual({ r: 255, g: 255, b: 255, a: 1 });
    expect(colorFormats(parseColor("rgb(255, 0, 0)")!)).toEqual({ hex: "#FF0000", rgb: "rgb(255, 0, 0)", hsl: "hsl(0, 100%, 50%)" });
    expect(colorFormats(parseColor("hsl(240, 100%, 50%)")!).hex).toBe("#0000FF");
    expect(parseColor("nope")).toBeNull();
    expect(readableOn("#FFFFFF")).toBe("dark");
    expect(readableOn("#1E1E1E")).toBe("light");
  });
});

describe("highlightRanges", () => {
  it("is accent and case insensitive", () => {
    expect(highlightRanges("Il Caffè è buono", ["caffe"])).toEqual([[3, 8]]);
    expect(highlightRanges("abc abc", ["b", "bc"])).toEqual([[1, 3], [5, 7]]);
  });
});

describe("format", () => {
  it("timeAgo and bytes", () => {
    const now = 10_000_000;
    expect(timeAgo(now - 120_000, "it", now)).toBe("2 min fa");
    expect(timeAgo(now - 3 * 3_600_000, "en", now)).toBe("3h ago");
    expect(bytes(1536)).toBe("1.5 KB");
  });
});

import { prettyShortcut, toAccelerator } from "../components/Settings/ShortcutRecorder";

describe("shortcuts", () => {
  it("pretty-prints in Apple modifier order", () => {
    expect(prettyShortcut("CommandOrControl+Alt+Shift+C")).toBe("⌥⇧⌘C");
    expect(prettyShortcut("CommandOrControl+Shift+V")).toBe("⇧⌘V");
    expect(prettyShortcut(null)).toBe("—");
  });
  it("converts key events to accelerators", () => {
    const ev = { metaKey: true, ctrlKey: false, altKey: true, shiftKey: true, code: "KeyC" } as KeyboardEvent;
    expect(toAccelerator(ev)).toBe("CommandOrControl+Alt+Shift+C");
    expect(toAccelerator({ ...ev, metaKey: false, altKey: false, shiftKey: false } as KeyboardEvent)).toBeNull();
  });
});
