import { useEffect, useState } from "react";
import { useDoclickStore } from "../store/useDoclickStore";

/// Tiny pill rendered on top of the overlay while the startup flow is
/// executing. Auto-fades 1.5s after `running` flips to false so the
/// user briefly sees the terminal status before it disappears.
export function StartupFlowBadge() {
  const runtime = useDoclickStore((s) => s.startupRuntime);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (runtime.running) {
      setVisible(true);
      return;
    }
    if (!visible) return;
    const t = window.setTimeout(() => setVisible(false), 1500);
    return () => window.clearTimeout(t);
  }, [runtime.running, visible]);

  if (!visible) return null;

  const steps = [runtime.accounts.status, runtime.ganymede.status];
  const settled = steps.filter((s) => s === "done" || s === "skipped" || s === "failed").length;
  const total = steps.length;
  const label = runtime.running ? `Démarrage… ${settled}/${total}` : "Démarrage terminé";

  return (
    <div className="pointer-events-none absolute right-2 top-2 z-50 rounded-full border border-amber-500/40 bg-amber-500/15 px-2 py-0.5 text-[0.625rem] font-medium text-amber-700 shadow-sm dark:text-amber-200">
      {label}
    </div>
  );
}
