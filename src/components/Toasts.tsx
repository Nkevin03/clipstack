import { AnimatePresence, motion } from "framer-motion";
import { useUi } from "../store/ui";

export function Toasts() {
  const toasts = useUi((s) => s.toasts);
  const dismiss = useUi((s) => s.dismissToast);
  return (
    <div aria-live="polite" className="pointer-events-none absolute bottom-3 left-1/2 z-[70] flex -translate-x-1/2 flex-col items-center gap-1.5">
      <AnimatePresence>
        {toasts.map((t) => (
          <motion.div
            key={t.id}
            layout
            initial={{ opacity: 0, y: 8, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 4, scale: 0.98 }}
            transition={{ type: "spring", stiffness: 500, damping: 34 }}
            className="pointer-events-auto flex items-center gap-3 rounded-full py-1.5 pl-3.5 pr-1.5 text-[12px] font-medium"
            style={{ background: t.tone === "error" ? "var(--cs-danger)" : "rgb(30 30 34 / 0.92)", color: "#fff", boxShadow: "var(--cs-shadow-pop)" }}
          >
            <span className={t.action ? "" : "pr-2"}>{t.text}</span>
            {t.action && (
              <button
                onClick={() => {
                  t.action!.run();
                  dismiss(t.id);
                }}
                className="rounded-full px-2.5 py-0.5 text-[11.5px] font-semibold"
                style={{ background: "rgb(255 255 255 / 0.16)" }}
              >
                {t.action.label} <span className="opacity-60">⌘Z</span>
              </button>
            )}
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}
