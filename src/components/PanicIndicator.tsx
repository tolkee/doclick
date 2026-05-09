import { useDoclickStore } from "../store/useDoclickStore";

/// Red pulsing frame shown ONLY while a broadcast is mid-flight (replicating).
/// "Armed but idle" is conveyed by the green BroadcastToggle, not by this frame.
export function PanicIndicator() {
  const live = useDoclickStore((s) => s.broadcastLive);
  if (!live) return null;
  return <div className="pointer-events-none absolute inset-0 broadcast-live-frame rounded-xl" />;
}
