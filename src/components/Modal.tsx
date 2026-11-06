import { motion } from "framer-motion";
import { useUi } from "../store/ui";

export function ModalShell({ children, width = 420, label, onClose }: { children: React.ReactNode; width?: number | string; label: string; onClose?: () => void }) {
  const close = useUi((s) => s.closeModal);
  return (
    <motion.div
      className="absolute inset-0 z-50 flex items-center justify-center p-4"
      style={{ background: "rgb(0 0 0 / 0.18)", borderRadius: "var(--cs-radius-panel)" }}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.12 }}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) (onClose ?? close)();
      }}
    >
      <motion.div
        role="dialog"
        aria-modal="true"
        aria-label={label}
        initial={{ opacity: 0, scale: 0.97, y: 6 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.97, y: 6 }}
        transition={{ type: "spring", stiffness: 500, damping: 36 }}
        className="flex max-h-full flex-col overflow-hidden rounded-[14px]"
        style={{ width, maxWidth: "100%", background: "var(--cs-surface)", boxShadow: "var(--cs-shadow-pop)", backdropFilter: "blur(30px)" }}
      >
        {children}
      </motion.div>
    </motion.div>
  );
}

/** Keyboard-navigable list used by pickers and the actions menu. */
export function useListKeys(count: number, index: number, setIndex: (i: number) => void, onEnter: (i: number, e: React.KeyboardEvent) => void) {
  return (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setIndex(Math.min(index + 1, count - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setIndex(Math.max(index - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (count > 0) onEnter(index, e);
    }
  };
}
