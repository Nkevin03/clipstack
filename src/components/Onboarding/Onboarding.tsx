import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { CheckCircle2, Keyboard, ShieldCheck } from "lucide-react";
import { ipc } from "../../lib/ipc";
import { useSettings } from "../../store/settings";
import { useT } from "../../lib/i18n";
import { useTauriEvent } from "../../hooks/useTauriEvent";
import { ShortcutRecorder, prettyShortcut } from "../Settings/ShortcutRecorder";

function Dots({ step }: { step: number }) {
  return (
    <div className="flex gap-1.5" aria-hidden>
      {[0, 1, 2].map((i) => (
        <span key={i} className="h-1.5 rounded-full transition-all" style={{ width: i === step ? 18 : 6, background: i === step ? "var(--cs-accent)" : "var(--cs-border-strong)" }} />
      ))}
    </div>
  );
}

export function Onboarding() {
  const t = useT();
  const { settings, set } = useSettings();
  const [step, setStep] = useState(0);
  const [trusted, setTrusted] = useState(false);
  const [tried, setTried] = useState(false);

  useEffect(() => {
    if (step !== 1) return;
    const check = () => ipc.checkAccessibility().then(setTrusted);
    void check();
    const id = window.setInterval(check, 1000);
    return () => window.clearInterval(id);
  }, [step]);
  useTauriEvent("panel:shown", () => step === 2 && setTried(true));

  if (!settings) return null;
  const screens = [
    <div key="0" className="flex flex-col items-center gap-4 text-center">
      <img src="/app-icon.png" alt="" className="h-24 w-24 drop-shadow-xl" />
      <h1 className="text-[24px] font-bold">{t("obWelcomeTitle")}</h1>
      <p className="max-w-[420px] text-[14px] leading-relaxed" style={{ color: "var(--cs-text-muted)" }}>{t("obWelcomeBody")}</p>
    </div>,
    <div key="1" className="flex flex-col items-center gap-4 text-center">
      <div className="flex h-20 w-20 items-center justify-center rounded-[22px]" style={{ background: "var(--cs-accent-soft)" }}>
        {trusted ? <CheckCircle2 size={40} style={{ color: "#16a34a" }} /> : <ShieldCheck size={40} style={{ color: "var(--cs-accent)" }} />}
      </div>
      <h1 className="text-[22px] font-bold">{t("obAccessTitle")}</h1>
      <p className="max-w-[440px] text-[13.5px] leading-relaxed" style={{ color: "var(--cs-text-muted)" }}>{t("obAccessBody")}</p>
      {trusted ? (
        <div className="text-[13px] font-semibold" style={{ color: "#16a34a" }}>{t("obGranted")}</div>
      ) : (
        <button onClick={() => ipc.requestAccessibility().then(setTrusted)} className="h-9 rounded-[9px] px-4 text-[13px] font-semibold" style={{ background: "var(--cs-accent)", color: "#fff" }}>
          {t("obGrant")}
        </button>
      )}
      <p className="max-w-[420px] text-[11.5px]" style={{ color: "var(--cs-text-faint)" }}>{t("obAccessDev")}</p>
    </div>,
    <div key="2" className="flex flex-col items-center gap-4 text-center">
      <div className="flex h-20 w-20 items-center justify-center rounded-[22px]" style={{ background: "var(--cs-accent-soft)" }}>
        {tried ? <CheckCircle2 size={40} style={{ color: "#16a34a" }} /> : <Keyboard size={40} style={{ color: "var(--cs-accent)" }} />}
      </div>
      <h1 className="text-[22px] font-bold">{t("obTryTitle")}</h1>
      <p className="text-[13.5px]" style={{ color: "var(--cs-text-muted)" }}>{t("obTryBody")}</p>
      <div className="text-[30px] font-bold tracking-wider">{prettyShortcut(settings.shortcut)}</div>
      {tried && <div className="text-[13px] font-semibold" style={{ color: "#16a34a" }}>{t("obDetected")}</div>}
      <div className="flex items-center gap-2 text-[12.5px]">
        <span style={{ color: "var(--cs-text-muted)" }}>{t("obChange")}</span>
        <ShortcutRecorder value={settings.shortcut} onChange={(v) => (v ? set("shortcut", v) : undefined)} />
      </div>
      {settings.shortcut === "CommandOrControl+Shift+V" && <p className="max-w-[440px] text-[11.5px]" style={{ color: "var(--cs-text-faint)" }}>{t("shortcutConflict")}</p>}
    </div>,
  ];
  return (
    <div className="flex h-full flex-col px-10 pb-6 pt-12" style={{ background: "var(--cs-surface)" }}>
      <div className="relative flex flex-1 items-center justify-center" data-tauri-drag-region>
        <AnimatePresence mode="wait">
          <motion.div key={step} initial={{ opacity: 0, x: 24 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -24 }} transition={{ duration: 0.2 }}>
            {screens[step]}
          </motion.div>
        </AnimatePresence>
      </div>
      <div className="flex items-center justify-between">
        <button onClick={() => setStep(Math.max(0, step - 1))} className="h-8 px-3 text-[13px]" style={{ visibility: step ? "visible" : "hidden", color: "var(--cs-text-muted)" }}>
          {t("obBack")}
        </button>
        <Dots step={step} />
        {step < 2 ? (
          <button onClick={() => setStep(step + 1)} className="h-8 rounded-[8px] px-4 text-[13px] font-semibold" style={{ background: step === 1 && !trusted ? "var(--cs-field)" : "var(--cs-accent)", color: step === 1 && !trusted ? undefined : "#fff" }}>
            {step === 1 && !trusted ? t("obSkip") : t("obNext")}
          </button>
        ) : (
          <button onClick={() => ipc.finishOnboarding()} className="h-8 rounded-[8px] px-4 text-[13px] font-semibold" style={{ background: "var(--cs-accent)", color: "#fff" }}>
            {t("obDone")}
          </button>
        )}
      </div>
    </div>
  );
}
