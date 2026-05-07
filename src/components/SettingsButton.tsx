import { Settings as SettingsIcon } from "lucide-react";

interface Props {
  onOpenSettings: () => void;
}

export function SettingsButton({ onOpenSettings }: Props) {
  return (
    <button
      onClick={onOpenSettings}
      className="w-9 h-9 rounded-md bg-foreground/5 hover:bg-foreground/10 text-foreground/80 flex items-center justify-center transition-colors"
      title="Paramètres"
      aria-label="Paramètres"
    >
      <SettingsIcon className="h-4 w-4" />
    </button>
  );
}
