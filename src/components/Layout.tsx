import { Outlet, NavLink } from "react-router-dom";
import { open } from "@tauri-apps/plugin-shell";
import { LayoutDashboard, ScrollText, Settings, Github, Route } from "lucide-react";
import { cn } from "@/lib/utils";
import StatusBar from "./StatusBar";

const navItems = [
  { to: "/dashboard", label: "概览", icon: LayoutDashboard },
  { to: "/logs", label: "日志", icon: ScrollText },
  { to: "/mappings/claude", label: "映射", icon: Route },
  { to: "/settings", label: "设置", icon: Settings },
];

export default function Layout() {
  return (
    <div className="flex h-screen overflow-hidden bg-background">
      {/* Sidebar */}
      <aside className="flex w-16 flex-col items-center border-r border-border bg-card py-4">
        {/* Logo */}
        <button
          type="button"
          onClick={() => open("https://github.com/pluto037/copilot-router").catch(() => {})}
          title="打开 GitHub 仓库"
          className="mb-6 flex h-9 w-9 items-center justify-center rounded-lg bg-primary/10 transition-colors hover:bg-primary/20"
        >
          <Github className="h-5 w-5 text-primary" />
        </button>

        {/* Nav */}
        <nav className="flex flex-1 flex-col items-center gap-1">
          {navItems.map(({ to, label, icon: Icon }) => (
            <NavLink
              key={to}
              to={to}
              title={label}
              className={({ isActive }) =>
                cn(
                  "flex h-10 w-10 items-center justify-center rounded-lg transition-colors",
                  isActive
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:bg-secondary hover:text-foreground"
                )
              }
            >
              <Icon className="h-5 w-5" />
            </NavLink>
          ))}
        </nav>
      </aside>

      {/* Main content */}
      <div className="flex flex-1 flex-col overflow-hidden">
        {/* Page */}
        <main className="flex-1 overflow-hidden">
          <Outlet />
        </main>

        {/* Status bar */}
        <StatusBar />
      </div>
    </div>
  );
}
