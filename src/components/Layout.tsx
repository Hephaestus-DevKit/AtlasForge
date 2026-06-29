import { NavLink, Outlet } from "react-router-dom";
import {
  LayoutDashboard,
  FolderTree,
  GitBranch,
  ListTodo,
  BookOpen,
  Zap,
  Settings,
} from "lucide-react";

const navItems = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard, end: true },
  { to: "/assets", label: "Assets", icon: FolderTree },
  { to: "/repositories", label: "Repositories", icon: GitBranch },
  { to: "/tasks", label: "Tasks", icon: ListTodo },
  { to: "/knowledge", label: "Knowledge", icon: BookOpen },
  { to: "/automations", label: "Automations", icon: Zap },
  { to: "/settings", label: "Settings", icon: Settings },
];

export function Layout() {
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
            <NavLink
              key={item.to}
              to={item.to}
              end={item.end}
              style={({ isActive }) => ({
                display: "flex",
                alignItems: "center",
                gap: 12,
                padding: "10px 16px",
                color: isActive ? "var(--text-primary)" : "var(--text-secondary)",
                background: isActive ? "rgba(99, 102, 241, 0.08)" : "transparent",
                border: isActive ? "1px solid rgba(99, 102, 241, 0.2)" : "1px solid transparent",
                textDecoration: "none",
                fontSize: 14,
                fontWeight: isActive ? 600 : 500,
                borderRadius: "var(--radius-sm)",
                transition: "all var(--transition-fast)",
              })}
              className={({ isActive }) => isActive ? "" : "sidebar-link-hover"}
            >
              {({ isActive }) => (
                <>
                  <item.icon size={18} color={isActive ? "var(--color-primary)" : "var(--text-secondary)"} style={{ transition: "color var(--transition-fast)" }} />
                  {item.label}
                </>
              )}
            </NavLink>
          ))}
        </div>
        
        {/* Style tag for custom hover behavior that React inline styles cannot do easily */}
        <style>{`
          .sidebar-link-hover:hover {
            color: var(--text-primary) !important;
            background: rgba(255, 255, 255, 0.03) !important;
          }
          .sidebar-link-hover:hover svg {
            color: var(--text-primary) !important;
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
          <Outlet />
        </div>
      </main>
    </div>
  );
}
