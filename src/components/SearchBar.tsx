import { forwardRef } from "react";
import { Search, X } from "lucide-react";
import { useItems } from "../store/items";
import { chipLabel } from "../lib/search-syntax";
import { useT } from "../lib/i18n";

export const SearchBar = forwardRef<HTMLInputElement>(function SearchBar(_, ref) {
  const t = useT();
  const { input, chips, setInput, removeChip, clearSearch } = useItems();
  return (
    <div
      className="flex h-9 min-w-0 flex-1 items-center gap-1.5 rounded-[10px] px-2.5"
      style={{ background: "var(--cs-field)", boxShadow: "inset 0 0 0 0.5px var(--cs-border)" }}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) {
          e.preventDefault();
          (ref as React.RefObject<HTMLInputElement>).current?.focus();
        }
      }}
    >
      <Search size={15} strokeWidth={2.2} style={{ color: "var(--cs-text-faint)" }} className="shrink-0" />
      {chips.map((c, i) => (
        <span key={`${c.key}-${c.value}-${i}`} className="flex shrink-0 items-center gap-1 rounded-[6px] py-[2px] pl-1.5 pr-1 text-[11.5px] font-medium" style={{ background: "var(--cs-accent-soft)", color: "var(--cs-text)" }}>
          <span style={{ color: "var(--cs-accent)" }}>{c.key}:</span>
          {c.value}
          <button aria-label={`${t("removeFilter")} ${chipLabel(c)}`} onClick={() => removeChip(i)} className="rounded p-[1px] hover:bg-[var(--cs-surface-hover)]">
            <X size={11} strokeWidth={2.5} />
          </button>
        </span>
      ))}
      <input
        ref={ref}
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onBlur={(e) => e.target.value !== input || setInput(e.target.value, true)}
        placeholder={chips.length ? "" : t("searchPlaceholder")}
        aria-label={t("searchPlaceholder")}
        spellCheck={false}
        autoCorrect="off"
        autoCapitalize="off"
        className="h-full min-w-[80px] flex-1 bg-transparent text-[13px] outline-none placeholder:text-[var(--cs-text-faint)]"
      />
      {(input || chips.length > 0) && (
        <button aria-label="Clear" onClick={clearSearch} className="shrink-0 rounded-full p-0.5" style={{ color: "var(--cs-text-faint)" }}>
          <X size={14} strokeWidth={2.4} />
        </button>
      )}
    </div>
  );
});
