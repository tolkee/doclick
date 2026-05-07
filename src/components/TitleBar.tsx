import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ArrowLeft, Minus, Square, Copy, X } from "lucide-react";

interface TitleBarProps {
  title?: string;
  /// When provided, render a back arrow on the left and call this handler
  /// when clicked. The arrow replaces the leading drag-region — the rest
  /// of the bar (around the title) is still draggable.
  onBack?: () => void;
  /// Default true. Pass `false` in overlay view: maximizing an
  /// always-on-top transparent panel to fullscreen is a footgun.
  showMaximize?: boolean;
}

export function TitleBar({ title, onBack, showMaximize = true }: TitleBarProps) {
  const win = getCurrentWindow();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!showMaximize) return;
    let active = true;
    win.isMaximized().then((m) => {
      if (active) setMaximized(m);
    });
    const unlistenP = win.onResized(() => {
      win.isMaximized().then((m) => {
        if (active) setMaximized(m);
      });
    });
    return () => {
      active = false;
      unlistenP.then((off) => off());
    };
  }, [win, showMaximize]);

  return (
    <div
      data-tauri-drag-region
      className="flex h-9 shrink-0 items-center justify-between bg-background/70 backdrop-blur-md"
    >
      <div className="flex items-center gap-2 px-3">
        {onBack && (
          <button
            type="button"
            data-tauri-drag-region="false"
            onClick={onBack}
            aria-label="Retour"
            title="Retour"
            className="flex h-6 w-6 items-center justify-center rounded-md text-foreground/80 hover:bg-foreground/10"
          >
            <ArrowLeft className="h-3.5 w-3.5" strokeWidth={2} />
          </button>
        )}
        <img
          src="/logo.png"
          alt=""
          className="pointer-events-none h-4 w-4 shrink-0 select-none"
          draggable={false}
        />
        {title && (
          <span className="pointer-events-none text-xs font-medium text-foreground/80">
            {title}
          </span>
        )}
      </div>
      <div className="ml-auto flex items-center">
        <TitleBarButton onClick={() => win.minimize()} ariaLabel="Minimiser">
          <Minus className="h-3.5 w-3.5" strokeWidth={2} />
        </TitleBarButton>
        {showMaximize && (
          <TitleBarButton
            onClick={async () => {
              if (await win.isMaximized()) await win.unmaximize();
              else await win.maximize();
            }}
            ariaLabel={maximized ? "Restaurer" : "Agrandir"}
          >
            {maximized ? (
              <Copy className="h-3 w-3 -scale-x-100" strokeWidth={2} />
            ) : (
              <Square className="h-3 w-3" strokeWidth={2} />
            )}
          </TitleBarButton>
        )}
        <TitleBarButton
          onClick={() => win.close()}
          ariaLabel="Fermer"
          danger
        >
          <X className="h-4 w-4" strokeWidth={2} />
        </TitleBarButton>
      </div>
    </div>
  );
}

function TitleBarButton({
  onClick,
  ariaLabel,
  danger,
  children,
}: {
  onClick: () => void;
  ariaLabel: string;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={ariaLabel}
      title={ariaLabel}
      onClick={onClick}
      className={`flex h-9 w-11 items-center justify-center text-foreground/90 transition-colors ${
        danger ? "hover:bg-red-600 hover:text-white" : "hover:bg-foreground/10"
      }`}
    >
      {children}
    </button>
  );
}
