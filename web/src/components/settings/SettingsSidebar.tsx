import { ArrowLeft } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  type SettingsPageId,
  visibleSettingsNav,
} from "@/components/settings/settingsNav";
import { cn } from "@/lib/utils";

export function SettingsSidebar({
  page,
  isAdmin,
  onSelect,
  onBack,
}: {
  page: SettingsPageId;
  isAdmin: boolean;
  onSelect: (page: SettingsPageId) => void;
  onBack: () => void;
}) {
  const nav = visibleSettingsNav(isAdmin);

  return (
    <aside className="flex h-full w-[200px] shrink-0 flex-col border-r bg-[var(--sidebar)]">
      <div className="flex items-center gap-1 border-b px-1.5 py-1.5">
        <button
          type="button"
          onClick={onBack}
          className="flex h-8 w-full items-center gap-1 rounded-md px-1 text-sm font-medium hover:bg-muted"
          title="Back"
        >
          <span className="grid h-5 w-5 place-items-center">
            <ArrowLeft className="h-4 w-4" />
          </span>
          Settings
        </button>
      </div>

      <nav className="flex flex-1 flex-col gap-4 overflow-y-auto px-1.5 pt-3">
        {nav.map((group, gi) => (
          <div key={group.label ?? `g-${gi}`} className="flex flex-col">
            {group.label ? (
              <div className="mb-0.5 px-1.5 py-0.5 text-xs font-medium text-muted-foreground">
                {group.label}
              </div>
            ) : null}
            {group.items.map((item) => {
              const active = page === item.id;
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => onSelect(item.id)}
                  className={cn(
                    "flex w-full items-center justify-between gap-2 rounded-md px-1.5 py-1.5 text-left text-sm transition-colors",
                    active
                      ? "bg-accent font-medium text-foreground"
                      : "text-foreground/80 hover:bg-muted hover:text-foreground",
                  )}
                >
                  <span className="truncate">{item.label}</span>
                  {item.badge ? (
                    <Badge
                      variant="secondary"
                      className="h-5 shrink-0 px-1.5 text-[10px] font-medium"
                    >
                      {item.badge}
                    </Badge>
                  ) : null}
                </button>
              );
            })}
          </div>
        ))}
      </nav>
    </aside>
  );
}
