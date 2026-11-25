import {
  LayoutDashboard,
  FolderKanban,
  Variable,
  KeyRound,
  Boxes,
  PlayCircle,
  BarChart3,
  ScrollText,
  Plus,
  ChevronLeft,
  ChevronRight,
  Moon,
  Sun,
  Globe,
  Settings,
  LogOut,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  type SettingsPageId,
  visibleSettingsNav,
} from "@/components/settings/settingsNav";
import { cn } from "@/lib/utils";
import type { Theme } from "@/lib/theme";

export type NavSection =
  | "dashboard"
  | "workflows"
  | "apis"
  | "variables"
  | "credentials"
  | "nodes"
  | "runs"
  | "insights"
  | "audit"
  | "settings";

const TOP_NAV: { id: NavSection; label: string; icon: typeof LayoutDashboard }[] = [
  { id: "dashboard", label: "Overview", icon: LayoutDashboard },
  { id: "workflows", label: "Projects", icon: FolderKanban },
  { id: "apis", label: "API Projects", icon: Globe },
];

const MID_NAV: { id: NavSection; label: string; icon: typeof LayoutDashboard }[] = [
  { id: "variables", label: "Variables", icon: Variable },
  { id: "credentials", label: "Credentials", icon: KeyRound },
  { id: "nodes", label: "Nodes", icon: Boxes },
  { id: "runs", label: "Executions", icon: PlayCircle },
  { id: "insights", label: "Insights", icon: BarChart3 },
  { id: "audit", label: "Audit", icon: ScrollText },
];

export function MainSidebar({
  section,
  onNavigate,
  onNewWorkflow,
  collapsed,
  onToggleCollapse,
  theme,
  onToggleTheme,
  actorName,
  role,
  onProfile,
  onOpenSettings,
  onLogout,
}: {
  section: NavSection;
  onNavigate: (s: NavSection) => void;
  onNewWorkflow: () => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
  theme: Theme;
  onToggleTheme: () => void;
  actorName: string;
  role?: string;
  onProfile?: () => void;
  onOpenSettings?: (page: SettingsPageId) => void;
  onLogout?: () => void;
}) {
  const isAdmin = role === "admin";
  const settingsNav = visibleSettingsNav(isAdmin);
  const initials = actorName
    .split(/\s+/)
    .map((p) => p[0])
    .join("")
    .slice(0, 2)
    .toUpperCase() || "DF";

  function NavBtn({
    id,
    label,
    icon: Icon,
  }: {
    id: NavSection;
    label: string;
    icon: typeof LayoutDashboard;
  }) {
    const active = section === id;
    return (
      <button
        type="button"
        title={label}
        onClick={() => onNavigate(id)}
        className={cn(
          "flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-[13px] transition-colors",
          active
            ? "bg-accent font-medium text-foreground"
            : "text-muted-foreground hover:bg-muted hover:text-foreground",
          collapsed && "justify-center px-0",
        )}
      >
        <Icon className="h-4 w-4 shrink-0" strokeWidth={1.75} />
        {!collapsed ? <span className="truncate">{label}</span> : null}
      </button>
    );
  }

  return (
    <aside
      className={cn(
        "flex h-full shrink-0 flex-col border-r bg-[var(--sidebar)] transition-[width] duration-200",
        collapsed ? "w-[52px]" : "w-[220px]",
      )}
    >
      <div
        className={cn(
          "flex items-center border-b px-3 py-3",
          collapsed ? "flex-col gap-2" : "justify-between",
        )}
      >
        <div className={cn("flex items-center gap-2", collapsed && "flex-col")}>
          <div
            className="flex h-7 w-7 items-center justify-center rounded-md text-[11px] font-bold text-white"
            style={{ background: "var(--brand)" }}
            title="DataFlow"
          >
            DF
          </div>
          {!collapsed ? (
            <span className="text-sm font-semibold tracking-tight">DataFlow</span>
          ) : null}
        </div>
        <button
          type="button"
          onClick={onNewWorkflow}
          className="flex h-7 w-7 items-center justify-center rounded-md border text-muted-foreground hover:bg-muted hover:text-foreground"
          title="New workflow"
        >
          <Plus className="h-4 w-4" />
        </button>
      </div>

      <div className="flex flex-1 flex-col gap-4 overflow-y-auto p-2">
        <div className="flex flex-col gap-0.5">
          {TOP_NAV.map((item) => (
            <NavBtn key={item.id} {...item} />
          ))}
          {!collapsed ? (
            <div className="mt-3 px-2.5 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              Projects
            </div>
          ) : null}
          {!collapsed ? (
            <button
              type="button"
              className="mt-1 flex items-center gap-2 rounded-md px-2.5 py-2 text-[13px] text-muted-foreground hover:bg-muted"
              onClick={onNewWorkflow}
            >
              <Plus className="h-3.5 w-3.5" />
              Add project
            </button>
          ) : null}
        </div>

        <div className="mt-auto flex flex-col gap-0.5 border-t pt-2">
          {MID_NAV.map((item) => (
            <NavBtn key={item.id} {...item} />
          ))}
        </div>
      </div>

      <div className="space-y-1 border-t p-2">
        <button
          type="button"
          onClick={onToggleTheme}
          className={cn(
            "flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-[13px] text-muted-foreground hover:bg-muted hover:text-foreground",
            collapsed && "justify-center px-0",
          )}
          title={theme === "dark" ? "Light mode" : "Dark mode"}
        >
          {theme === "dark" ? (
            <Sun className="h-4 w-4" />
          ) : (
            <Moon className="h-4 w-4" />
          )}
          {!collapsed ? (theme === "dark" ? "Light mode" : "Dark mode") : null}
        </button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <button
              type="button"
              className={cn(
                "flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-[13px] text-muted-foreground outline-none hover:bg-muted hover:text-foreground",
                section === "settings" &&
                  "bg-accent font-medium text-foreground",
                collapsed && "justify-center px-0",
              )}
              title="Settings"
            >
              <Settings className="h-4 w-4" />
              {!collapsed ? "Settings" : null}
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            side="right"
            align="end"
            sideOffset={8}
            className="max-h-[min(80vh,720px)] w-[260px] overflow-y-auto p-2"
          >
            {settingsNav.map((group, gi) => (
              <div key={group.label ?? `g-${gi}`}>
                {gi > 0 ? <DropdownMenuSeparator className="my-2" /> : null}
                {group.label ? (
                  <DropdownMenuLabel className="px-2.5 text-[10px] font-medium uppercase tracking-wider">
                    {group.label}
                  </DropdownMenuLabel>
                ) : null}
                {group.items.map((item) => (
                  <DropdownMenuItem
                    key={item.id}
                    className="justify-between gap-2 px-2.5 py-2 text-[13px]"
                    onSelect={() => onOpenSettings?.(item.id)}
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
                  </DropdownMenuItem>
                ))}
              </div>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
        <button
          type="button"
          onClick={onProfile}
          className={cn(
            "flex w-full items-center gap-2 rounded-md px-2.5 py-2 text-left hover:bg-muted",
            collapsed && "justify-center px-0",
          )}
          title="Profile"
        >
          <div
            className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-[10px] font-semibold text-white"
            style={{ background: "var(--brand)" }}
          >
            {initials}
          </div>
          {!collapsed ? (
            <div className="min-w-0">
              <div className="truncate text-[13px] font-medium">{actorName}</div>
              <div className="text-[10px] text-muted-foreground">
                {role ?? "user"}
              </div>
            </div>
          ) : null}
        </button>
        {onLogout ? (
          <button
            type="button"
            onClick={onLogout}
            className={cn(
              "flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-[13px] text-muted-foreground hover:bg-muted hover:text-foreground",
              collapsed && "justify-center px-0",
            )}
            title="Sign out"
          >
            <LogOut className="h-4 w-4" />
            {!collapsed ? "Sign out" : null}
          </button>
        ) : null}
        <button
          type="button"
          onClick={onToggleCollapse}
          className={cn(
            "flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-[12px] text-muted-foreground hover:bg-muted",
            collapsed && "justify-center px-0",
          )}
        >
          {collapsed ? (
            <ChevronRight className="h-4 w-4" />
          ) : (
            <>
              <ChevronLeft className="h-4 w-4" />
              Collapse
            </>
          )}
        </button>
      </div>
    </aside>
  );
}
