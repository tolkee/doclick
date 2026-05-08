import { useDoclickStore } from "../store/useDoclickStore";

export function BroadcastToggle() {
  const enabled = useDoclickStore((s) => s.broadcastEnabled);
  const live = useDoclickStore((s) => s.broadcastLive);
  const toggle = useDoclickStore((s) => s.toggleBroadcast);

  // Three states:
  //   off:    gray
  //   armed:  green with pulsing ring (broadcast enabled, not dispatching → safe to click)
  //   live:   red pulsing (dispatch in flight → DO NOT click)
  const stateClasses = !enabled
    ? "bg-zinc-700 text-zinc-200 hover:bg-zinc-600"
    : live
      ? "bg-red-600 text-white ring-2 ring-red-300 animate-pulse"
      : "bg-emerald-600 text-white hover:bg-emerald-500";

  const tooltip = !enabled ? "Broadcast OFF" : live ? "Diffusion…" : "Broadcast ON";
  const armed = enabled && !live;

  return (
    <button
      onClick={toggle}
      title={tooltip}
      className={`relative flex items-center justify-center w-11 h-11 rounded-full transition-colors ${stateClasses}`}
      aria-label="Broadcast"
    >
      <span className="relative inline-flex h-3 w-3 items-center justify-center">
        {armed && (
          <span className="absolute inline-flex h-full w-full rounded-full bg-white/70 animate-ping" />
        )}
        <span className="relative inline-flex w-3 h-3 rounded-full bg-white/90" />
      </span>
    </button>
  );
}
