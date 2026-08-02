import { useEffect, useState, type ReactNode } from "react";
import {
  LayoutDashboard,
  FolderTree,
  GitBranch,
  ListTodo,
  BookOpen,
  Zap,
  Settings,
  Sun,
  Moon,
} from "lucide-react";
import type { AppRoute } from "../routing";

const navItems: ReadonlyArray<{ to: AppRoute; label: string; icon: typeof LayoutDashboard }> = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard },
  { to: "/assets", label: "Assets", icon: FolderTree },
  { to: "/repositories", label: "Repositories", icon: GitBranch },
  { to: "/tasks", label: "Tasks", icon: ListTodo },
  { to: "/knowledge", label: "Knowledge", icon: BookOpen },
  { to: "/automations", label: "Automations", icon: Zap },
  { to: "/settings", label: "Settings", icon: Settings },
];

export function Layout({ currentRoute, children }: { currentRoute: AppRoute; children: ReactNode }) {
  const [theme, setTheme] = useState(() => {
    return localStorage.getItem("theme") || "dark";
  });

  useEffect(() => {
    if (theme === "light") {
      document.body.classList.add("light-theme");
    } else {
      document.body.classList.remove("light-theme");
    }
    localStorage.setItem("theme", theme);
  }, [theme]);

  return (
    <div style={{ display: "flex", height: "100vh", width: "100vw", background: "var(--bg-app)", overflow: "hidden" }}>
      <nav
        style={{
          width: 240,
          minWidth: 240,
          background: "var(--bg-sidebar)",
          borderRight: "1px solid var(--border-color)",
          display: "flex",
          flexDirection: "column",
          padding: "20px 0",
          zIndex: 10,
          boxShadow: "4px 0 20px rgba(0,0,0,0.3)",
        }}
      >
        <div
          style={{
            padding: "0 24px 24px",
            fontSize: 20,
            fontWeight: 800,
            letterSpacing: "-0.025em",
            background: "linear-gradient(135deg, var(--text-primary), var(--color-accent))",
            WebkitBackgroundClip: "text",
            WebkitTextFillColor: "transparent",
            borderBottom: "1px solid var(--border-color)",
            marginBottom: 16,
            display: "flex",
            alignItems: "center",
            gap: 8,
          }}
        >
          <div style={{
            width: 10,
            height: 10,
            borderRadius: "50%",
            background: "var(--color-primary)",
            boxShadow: "0 0 10px var(--color-primary)",
          }} />
          AtlasForge
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 4, padding: "0 12px" }}>
          {navItems.map((item) => (
            <a
              key={item.to}
              href={`#${item.to}`}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 12,
                padding: "10px 16px",
                color: currentRoute === item.to ? "var(--text-primary)" : "var(--text-secondary)",
                background: currentRoute === item.to ? "rgba(99, 102, 241, 0.08)" : "transparent",
                border: currentRoute === item.to ? "1px solid rgba(99, 102, 241, 0.2)" : "1px solid transparent",
                textDecoration: "none",
                fontSize: 14,
                fontWeight: currentRoute === item.to ? 600 : 500,
                borderRadius: "var(--radius-sm)",
                transition: "all var(--transition-fast)",
              }}
              className={currentRoute === item.to ? "" : "sidebar-link-hover"}
            >
              <item.icon size={18} color={currentRoute === item.to ? "var(--color-primary)" : "var(--text-secondary)"} style={{ transition: "color var(--transition-fast)" }} />
              {item.label}
            </a>
          ))}
        </div>
        
        <div style={{ marginTop: "auto", padding: "0 12px" }}>
          <button
            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: 10,
              width: "100%",
              padding: "10px 16px",
              color: "var(--text-secondary)",
              background: "rgba(255,255,255,0.02)",
              border: "1px solid var(--border-color)",
              borderRadius: "var(--radius-sm)",
              cursor: "pointer",
              fontSize: 13,
              fontWeight: 600,
              transition: "all var(--transition-fast)",
            }}
            className="theme-toggle-btn"
          >
            {theme === "dark" ? <Sun size={16} /> : <Moon size={16} />}
            {theme === "dark" ? "Day Mode" : "Dark Space"}
          </button>
        </div>

        <style>{`
          .sidebar-link-hover:hover {
            color: var(--text-primary) !important;
            background: var(--bg-button-secondary) !important;
          }
          .sidebar-link-hover:hover svg {
            color: var(--text-primary) !important;
          }
          .theme-toggle-btn:hover {
            color: var(--text-primary) !important;
            background: var(--bg-button-secondary-hover) !important;
            border-color: var(--border-color-hover) !important;
          }
        `}</style>
      </nav>
      <main
        style={{
          flex: 1,
          overflow: "auto",
          padding: "28px 40px",
          background: "var(--bg-app)",
          minWidth: 0,
          position: "relative",
        }}
        className="scrollbar-custom"
      >
        <div className="fade-in" style={{ maxWidth: 1400, margin: "0 auto", height: "100%" }}>
          {children}
        </div>
      </main>
    </div>
  );
}

