import { useT } from "../lib/i18n";
import { ipc } from "../lib/ipc";

type Variant = "empty" | "noResults" | "paused" | "emptySpace";

/** Original line illustrations: stacked cards, a magnifier, a pause mark, an open tray. */
function Illustration({ variant }: { variant: Variant }) {
  const stroke = "var(--cs-text-faint)";
  const accent = "var(--cs-accent)";
  return (
    <svg width="92" height="72" viewBox="0 0 92 72" fill="none" aria-hidden>
      <rect x="26" y="6" width="44" height="52" rx="9" stroke={stroke} strokeWidth="2" opacity="0.5" transform="rotate(8 48 32)" />
      <rect x="18" y="12" width="44" height="52" rx="9" fill="var(--cs-surface)" stroke={stroke} strokeWidth="2" />
      <rect x="18" y="12" width="44" height="12" rx="9" fill={accent} opacity="0.85" />
      {variant === "noResults" ? (
        <>
          <circle cx="40" cy="42" r="9" stroke={accent} strokeWidth="2.5" />
          <path d="M47 49l7 7" stroke={accent} strokeWidth="2.5" strokeLinecap="round" />
        </>
      ) : variant === "paused" ? (
        <>
          <rect x="33" y="34" width="5" height="18" rx="2.5" fill={accent} />
          <rect x="43" y="34" width="5" height="18" rx="2.5" fill={accent} />
        </>
      ) : variant === "emptySpace" ? (
        <path d="M28 44h8l3 5h4l3-5h8M28 44l4-10h16l4 10v8H28z" stroke={accent} strokeWidth="2.2" strokeLinejoin="round" />
      ) : (
        <>
          <rect x="26" y="33" width="26" height="3.5" rx="1.75" fill={stroke} opacity="0.6" />
          <rect x="26" y="41" width="18" height="3.5" rx="1.75" fill={stroke} opacity="0.4" />
          <rect x="26" y="49" width="22" height="3.5" rx="1.75" fill={stroke} opacity="0.25" />
          <path d="M76 18l2 4 4 2-4 2-2 4-2-4-4-2 4-2z" fill={accent} />
        </>
      )}
    </svg>
  );
}

export function EmptyState({ variant }: { variant: Variant }) {
  const t = useT();
  const title = { empty: t("emptyTitle"), noResults: t("noResultsTitle"), paused: t("pausedTitle"), emptySpace: t("emptySpaceTitle") }[variant];
  const body = { empty: t("emptyBody"), noResults: t("noResultsBody"), paused: t("pausedBody"), emptySpace: t("emptySpaceBody") }[variant];
  return (
    <div className="flex h-full w-full flex-col items-center justify-center gap-2 text-center">
      <Illustration variant={variant} />
      <div className="mt-1 text-[13.5px] font-semibold">{title}</div>
      <div className="max-w-[300px] text-[12px]" style={{ color: "var(--cs-text-muted)" }}>
        {body}
      </div>
      {variant === "paused" && (
        <button onClick={() => ipc.pauseMonitoring(false)} className="mt-1 h-7 rounded-full px-3 text-[12px] font-semibold" style={{ background: "var(--cs-accent)", color: "var(--cs-accent-text)" }}>
          {t("resume")}
        </button>
      )}
    </div>
  );
}
