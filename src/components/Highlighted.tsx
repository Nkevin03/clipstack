import { highlightRanges } from "../lib/format";

/** Text with search terms marked (accent/case-insensitive). */
export function Highlighted({ text, terms }: { text: string; terms: string[] }) {
  if (!terms.length) return <>{text}</>;
  const ranges = highlightRanges(text, terms);
  if (!ranges.length) return <>{text}</>;
  const out: React.ReactNode[] = [];
  let last = 0;
  ranges.forEach(([a, b], i) => {
    if (a > last) out.push(text.slice(last, a));
    out.push(
      <mark key={i} className="cs-hit">
        {text.slice(a, b)}
      </mark>,
    );
    last = b;
  });
  if (last < text.length) out.push(text.slice(last));
  return <>{out}</>;
}
