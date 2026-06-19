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
    <div style={{ display: "flex", height: "100vh", width: "100vw" }}>
      <nav
        style={{
          width: 220,
          minWidth: 220,
          background: "#1e293b",
          color: "#e2e8f0",
          display: "flex",
          flexDirection: "column",
          padding: "16px 0",
        }}
      >
        <div
          style={{
            padding: "0 20px 20px",
            fontSize: 18,
            fontWeight: 700,
            color: "#f8fafc",
            borderBottom: "1px solid #334155",
            marginBottom: 8,
          }}
        >
          AtlasForge
        </div>
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.end}
            style={({ isActive }) => ({
              display: "flex",
              alignItems: "center",
              gap: 10,
              padding: "10px 20px",
              color: isActive ? "#f8fafc" : "#94a3b8",
              background: isActive ? "#334155" : "transparent",
              textDecoration: "none",
              fontSize: 14,
              fontWeight: isActive ? 600 : 400,
              borderLeft: isActive ? "3px solid #3b82f6" : "3px solid transparent",
            })}
          >
            <item.icon size={18} />
            {item.label}
          </NavLink>
        ))}
      </nav>
      <main
        style={{
          flex: 1,
          overflow: "auto",
          padding: 24,
          background: "#f8fafc",
          minWidth: 0,
        }}
      >
        <Outlet />
      </main>
    </div>
  );
}
