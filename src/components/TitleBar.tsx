import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, Copy, X } from "lucide-react";

export function TitleBar({ title }: { title?: string }) {
  const win = getCurrentWindow();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
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
  }, [win]);

  return (
    <div
      data-tauri-drag-region
      className="flex h-9 shrink-0 items-center justify-between bg-background/70 backdrop-blur-md"
    >
      <div className="pointer-events-none flex items-center gap-2 px-3">
        <img
          src="/logo.png"
          alt=""
          className="h-4 w-4 shrink-0 select-none"
          draggable={false}
        />
        {title && (
          <span className="text-xs font-medium text-foreground/80">
            {title}
          </span>
        )}
      </div>
      <div className="ml-auto flex items-center">
        <TitleBarButton onClick={() => win.minimize()} ariaLabel="Minimiser">
          <Minus className="h-3.5 w-3.5" strokeWidth={2} />
        </TitleBarButton>
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
