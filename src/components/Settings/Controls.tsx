export function Section({ title, children, hint }: { title?: string; children: React.ReactNode; hint?: string }) {
  return (
    <section className="mb-5">
      {title && <h2 className="cs-caps mb-1.5 px-1">{title}</h2>}
      <div className="overflow-hidden rounded-[10px]" style={{ background: "var(--cs-surface-2)", boxShadow: "inset 0 0 0 0.5px var(--cs-border-strong)" }}>
        {children}
      </div>
      {hint && <p className="mt-1.5 px-1 text-[11.5px] leading-snug" style={{ color: "var(--cs-text-faint)" }}>{hint}</p>}
    </section>
  );
}

export function Field({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <div className="flex min-h-[40px] items-center gap-3 px-3 py-1.5 [&+&]:border-t" style={{ borderColor: "var(--cs-border)" }}>
      <div className="min-w-0 flex-1">
        <div className="text-[13px]">{label}</div>
        {hint && <div className="text-[11px] leading-snug" style={{ color: "var(--cs-text-faint)" }}>{hint}</div>}
      </div>
      <div className="flex shrink-0 items-center gap-2">{children}</div>
    </div>
  );
}

export function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className="relative h-[22px] w-[38px] rounded-full transition-colors"
      style={{ background: checked ? "var(--cs-accent)" : "var(--cs-border-strong)" }}
    >
      <span className="absolute top-[2px] h-[18px] w-[18px] rounded-full bg-white shadow transition-all" style={{ left: checked ? 18 : 2 }} />
    </button>
  );
}

export function Select<T extends string>({ value, options, onChange, label }: { value: T; options: [T, string][]; onChange: (v: T) => void; label: string }) {
  return (
    <select
      aria-label={label}
      value={value}
      onChange={(e) => onChange(e.target.value as T)}
      className="h-7 rounded-[7px] px-2 text-[12.5px] outline-none"
      style={{ background: "var(--cs-field)", boxShadow: "inset 0 0 0 0.5px var(--cs-border-strong)" }}
    >
      {options.map(([v, l]) => (
        <option key={v} value={v}>
          {l}
        </option>
      ))}
    </select>
  );
}

export function Segmented<T extends string>({ value, options, onChange, label }: { value: T; options: [T, string][]; onChange: (v: T) => void; label: string }) {
  return (
    <div role="radiogroup" aria-label={label} className="flex rounded-[8px] p-[2px]" style={{ background: "var(--cs-field)" }}>
      {options.map(([v, l]) => (
        <button
          key={v}
          role="radio"
          aria-checked={value === v}
          onClick={() => onChange(v)}
          className="h-6 rounded-[6px] px-2.5 text-[12px] transition-all"
          style={{ background: value === v ? "var(--cs-surface)" : "transparent", boxShadow: value === v ? "0 1px 2px rgb(0 0 0 / 0.15)" : undefined, fontWeight: value === v ? 600 : 400 }}
        >
          {l}
        </button>
      ))}
    </div>
  );
}

export function NumberInput({ value, onChange, min, max, label, suffix }: { value: number; onChange: (v: number) => void; min: number; max: number; label: string; suffix?: string }) {
  return (
    <label className="flex items-center gap-1.5 text-[12.5px]">
      <input
        type="number"
        aria-label={label}
        min={min}
        max={max}
        defaultValue={value}
        key={value}
        onBlur={(e) => {
          const v = Math.max(min, Math.min(max, Math.round(Number(e.target.value) || 0)));
          if (v !== value) onChange(v);
          else e.target.value = String(value);
        }}
        onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
        className="h-7 w-20 rounded-[7px] px-2 text-right outline-none"
        style={{ background: "var(--cs-field)", boxShadow: "inset 0 0 0 0.5px var(--cs-border-strong)" }}
      />
      {suffix && <span style={{ color: "var(--cs-text-muted)" }}>{suffix}</span>}
    </label>
  );
}

export function Button({ children, onClick, tone, disabled }: { children: React.ReactNode; onClick: () => void; tone?: "primary" | "danger"; disabled?: boolean }) {
  return (
    <button
      disabled={disabled}
      onClick={onClick}
      className="h-7 rounded-[7px] px-3 text-[12.5px] font-medium disabled:opacity-40"
      style={{
        background: tone === "primary" ? "var(--cs-accent)" : tone === "danger" ? "var(--cs-danger)" : "var(--cs-field)",
        color: tone ? "#fff" : undefined,
        boxShadow: tone ? undefined : "inset 0 0 0 0.5px var(--cs-border-strong)",
      }}
    >
      {children}
    </button>
  );
}
