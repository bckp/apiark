import { useTranslation } from "react-i18next";
import {
  FolderOpen,
  Globe,
  Clock,
  Settings,
  Terminal,
} from "lucide-react";

export type ActivityView = "collections" | "environments" | "history";

interface ActivityBarProps {
  activeView: ActivityView;
  onViewChange: (view: ActivityView) => void;
  onOpenSettings: () => void;
  onToggleConsole: () => void;
}

const TOP_ITEMS: { id: ActivityView; icon: typeof FolderOpen; labelKey: string; color: string; glow: string }[] = [
  { id: "collections", icon: FolderOpen, labelKey: "sidebar.collections", color: "text-blue-400", glow: "bg-blue-400/10" },
  { id: "environments", icon: Globe, labelKey: "sidebar.environments", color: "text-emerald-400", glow: "bg-emerald-400/10" },
  { id: "history", icon: Clock, labelKey: "history.title", color: "text-amber-400", glow: "bg-amber-400/10" },
];

export function ActivityBar({
  activeView,
  onViewChange,
  onOpenSettings,
  onToggleConsole,
}: ActivityBarProps) {
  const { t } = useTranslation();
  return (
    <div className="flex w-12 shrink-0 flex-col items-center border-r border-[var(--color-border)] bg-[var(--color-activity-bar)] py-3">
      {/* Logo */}
      <div className="mb-4 flex h-8 w-8 items-center justify-center overflow-hidden rounded-lg">
        <img src="/app-icon.png" alt="ApiArk" className="h-8 w-8 rounded-lg" />
      </div>

      {/* Top nav items */}
      <div className="flex flex-col items-center gap-1">
        {TOP_ITEMS.map((item) => (
          <ActivityBarButton
            key={item.id}
            icon={item.icon}
            label={t(item.labelKey)}
            active={activeView === item.id}
            activeColor={item.color}
            activeGlow={item.glow}
            onClick={() => onViewChange(item.id)}
          />
        ))}
      </div>

      <div className="mx-auto my-3 h-px w-6 bg-[var(--color-border)]" />

      {/* Spacer */}
      <div className="flex-1" />

      {/* Bottom actions */}
      <div className="flex flex-col items-center gap-1">
        <ActivityBarButton icon={Terminal} label={`${t("console.title")} (Ctrl+\`)`} onClick={onToggleConsole} activeColor="text-orange-400" activeGlow="bg-orange-400/10" />
        <ActivityBarButton icon={Settings} label={`${t("settings.title")} (Ctrl+,)`} onClick={onOpenSettings} />
      </div>
    </div>
  );
}

function ActivityBarButton({
  icon: Icon,
  label,
  active,
  activeColor,
  activeGlow,
  onClick,
}: {
  icon: typeof FolderOpen;
  label: string;
  active?: boolean;
  activeColor?: string;
  activeGlow?: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      title={label}
      className={`relative flex h-9 w-9 items-center justify-center rounded-lg transition-all ${
        active
          ? `${activeGlow ?? "bg-[var(--color-accent-glow)]"} ${activeColor ?? "text-[var(--color-accent)]"}`
          : "text-[var(--color-text-muted)] hover:bg-[var(--color-elevated)] hover:text-[var(--color-text-secondary)]"
      }`}
    >
      {active && (
        <span className={`absolute left-0 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-r ${activeColor ? activeColor.replace("text-", "bg-") : "bg-[var(--color-accent)]"}`} />
      )}
      <Icon className="h-[18px] w-[18px]" strokeWidth={active ? 2 : 1.5} />
    </button>
  );
}
