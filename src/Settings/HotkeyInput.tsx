import { X } from "lucide-react";
import { useEffect, useState } from "react";
import { cn } from "../lib/utils";

interface Props {
  value: string | null;
  onChange: (next: string | null) => void;
  placeholder?: string;
  className?: string;
}

/// Records a shortcut when focused. Captures key combos, mouse side buttons
/// (Mouse4/Mouse5), middle/scroll click (Mouse3) and wheel scroll
/// (WheelUp/WheelDown), with optional Ctrl/Shift/Alt/Meta modifiers. Clear
/// with the inline × button.
export function HotkeyInput({ value, onChange, placeholder, className = "" }: Props) {
  const [recording, setRecording] = useState(false);
  const [draft, setDraft] = useState<string | null>(value);

  useEffect(() => setDraft(value), [value]);

  const commit = (accel: string) => {
    setDraft(accel);
    onChange(accel);
    setRecording(false);
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();

    const k = e.key;
    if (k === "Escape") {
      setRecording(false);
      return;
    }
    if (k === "Control" || k === "Shift" || k === "Alt" || k === "Meta") return;

    const main = normalizeKey(k);
    if (!main) return;
    commit(buildAccel(e, main));
  };

  // Mouse buttons. button: 0=left, 1=middle, 2=right, 3=back (XButton1), 4=forward (XButton2).
  // Left/right are reserved (left starts recording / is broadcast trigger;
  // right is the OS context menu and used by Dofus).
  const onMouseDown = (e: React.MouseEvent<HTMLInputElement>) => {
    if (!recording) return;
    if (e.button === 0 || e.button === 2) return;
    const trig = mouseButtonName(e.button);
    if (!trig) return;
    e.preventDefault();
    e.stopPropagation();
    commit(buildAccel(e, trig));
  };

  const onWheel = (e: React.WheelEvent<HTMLInputElement>) => {
    if (!recording) return;
    if (e.deltaY === 0) return;
    e.preventDefault();
    e.stopPropagation();
    commit(buildAccel(e, e.deltaY < 0 ? "WheelUp" : "WheelDown"));
  };

  return (
    <div className={cn("relative inline-flex items-center", className)}>
      <input
        readOnly
        value={recording ? "Appuyer…" : (draft ?? "")}
        placeholder={placeholder ?? "Cliquer pour définir"}
        onClick={() => setRecording(true)}
        onContextMenu={(e) => e.preventDefault()}
        onBlur={() => setRecording(false)}
        onKeyDown={onKeyDown}
        onMouseDown={onMouseDown}
        onWheel={onWheel}
        className={cn(
          "h-8 w-44 cursor-pointer rounded-md border border-input bg-transparent pl-2 pr-7 text-sm outline-none placeholder:text-muted-foreground",
          recording ? "ring-2 ring-ring ring-offset-2 ring-offset-background" : "",
        )}
      />
      {draft && (
        <button
          type="button"
          aria-label="Effacer le raccourci"
          title="Effacer le raccourci"
          // mousedown (not click) so we clear before the input's onClick reopens recording.
          onMouseDown={(e) => {
            e.preventDefault();
            e.stopPropagation();
            setDraft(null);
            onChange(null);
            setRecording(false);
          }}
          className="absolute right-1 top-1/2 inline-flex h-5 w-5 -translate-y-1/2 items-center justify-center rounded text-muted-foreground hover:bg-muted/60 hover:text-foreground"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      )}
    </div>
  );
}

function buildAccel(
  e: { ctrlKey: boolean; shiftKey: boolean; altKey: boolean; metaKey: boolean },
  trigger: string,
): string {
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.shiftKey) parts.push("Shift");
  if (e.altKey) parts.push("Alt");
  if (e.metaKey) parts.push("Meta");
  parts.push(trigger);
  return parts.join("+");
}

function mouseButtonName(button: number): string | null {
  switch (button) {
    case 1:
      return "Mouse3";
    case 3:
      return "Mouse4";
    case 4:
      return "Mouse5";
    default:
      return null;
  }
}

function normalizeKey(k: string): string | null {
  if (k.length === 1) {
    if (/[a-zA-Z]/.test(k)) return k.toUpperCase();
    if (/[0-9]/.test(k)) return k;
    return null;
  }
  if (/^F([1-9]|1[0-2])$/.test(k)) return k;
  switch (k) {
    case " ":
      return "Space";
    case "Enter":
      return "Enter";
    case "Tab":
      return "Tab";
    case "Backspace":
      return "Backspace";
    case "Delete":
      return "Delete";
    case "ArrowLeft":
      return "Left";
    case "ArrowRight":
      return "Right";
    case "ArrowUp":
      return "Up";
    case "ArrowDown":
      return "Down";
    default:
      return null;
  }
}
