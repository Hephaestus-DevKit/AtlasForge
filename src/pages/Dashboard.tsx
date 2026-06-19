import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { listWorkspaceRoots, listRepositories, listJobs, startScan, listNotifications } from "../api/ipc";
import type { WorkspaceRoot, Repository, Job, Notification } from "../types";
import { FolderTree, GitBranch, ListTodo, AlertTriangle, Bell, Plus, Scan } from "lucide-react";

export function Dashboard() {
  const [roots, setRoots] = useState<WorkspaceRoot[]>([]);
  const [repos, setRepos] = useState<Repository[]>([]);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    refresh();
  }, []);

  async function refresh() {
    try {
      setError(null);
      const [r, rp, j, n] = await Promise.all([
        listWorkspaceRoots(),
        listRepositories(),
        listJobs(10),
        listNotifications(true, 5),
      ]);
      setRoots(r);
      setRepos(rp);
      setJobs(j);
      setNotifications(n);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load data");
    }
  }

  async function handleScan() {
    try {
      setScanning(true);
      setError(null);
      const result = await startScan();
      await refresh();
      if (result.errors.length > 0) {
        setError(`Scan completed with ${result.errors.length} error(s)`);
      }
    } catch (e: any) {
      setError(e?.toString() ?? "Scan failed");
    } finally {
      setScanning(false);
    }
  }

  const dirtyRepos = repos.filter((r) => r.dirtyState);
  const runningJobs = jobs.filter((j) => j.status === "running");
  const unreadCount = notifications.filter((n) => !n.read).length;

  // First-run empty state: no roots configured
  if (roots.length === 0) {
    return (
      <div>
        <h1 style={{ fontSize: 24, fontWeight: 700, marginBottom: 24 }}>Dashboard</h1>
        {error && (
          <div style={{ padding: 12, background: "#fef2f2", border: "1px solid #fca5a5", borderRadius: 6, marginBottom: 16, color: "#991b1b" }}>
            {error}
          </div>
        )}
        <div style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          padding: "60px 20px",
          background: "#fff",
          borderRadius: 8,
          border: "1px dashed #cbd5e1",
        }}>
          <FolderTree size={48} color="#94a3b8" strokeWidth={1.5} />
          <h2 style={{ fontSize: 18, fontWeight: 600, color: "#334155", marginTop: 16, marginBottom: 8 }}>
            Welcome to AtlasForge
          </h2>
          <p style={{ color: "#64748b", fontSize: 14, textAlign: "center", maxWidth: 440, marginBottom: 20 }}>
            Add a workspace root to start discovering repositories, running health audits,
            and managing your projects. Everything stays on your machine.
          </p>
          <div style={{ display: "flex", gap: 12 }}>
            <button
              onClick={() => navigate("/assets")}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                padding: "10px 24px",
                background: "#3b82f6",
                color: "#fff",
                border: "none",
                borderRadius: 6,
                cursor: "pointer",
                fontSize: 15,
                fontWeight: 600,
              }}
            >
              <Plus size={18} />
              Add Workspace Root
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 24 }}>
        <h1 style={{ fontSize: 24, fontWeight: 700 }}>Dashboard</h1>
        <button
          onClick={handleScan}
          disabled={scanning}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            padding: "8px 16px",
            background: scanning ? "#94a3b8" : "#10b981",
            color: "#fff",
            border: "none",
            borderRadius: 6,
            cursor: scanning ? "not-allowed" : "pointer",
            fontSize: 14,
            fontWeight: 600,
          }}
        >
          <Scan size={16} />
          {scanning ? "Scanning All Roots..." : "Scan All Roots"}
        </button>
      </div>

      {error && (
        <div style={{ padding: 12, background: "#fef2f2", border: "1px solid #fca5a5", borderRadius: 6, marginBottom: 16, color: "#991b1b" }}>
          {error}
        </div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 16, marginBottom: 24 }}>
        <StatCard icon={FolderTree} label="Workspace Roots" value={roots.length} color="#3b82f6" />
        <StatCard icon={GitBranch} label="Repositories" value={repos.length} color="#10b981" />
        <StatCard icon={AlertTriangle} label="Dirty Repos" value={dirtyRepos.length} color="#f59e0b" />
        <StatCard icon={ListTodo} label="Running Jobs" value={runningJobs.length} color="#8b5cf6" />
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
        {/* Recent Jobs */}
        <div style={{ background: "#fff", borderRadius: 8, padding: 20, border: "1px solid #e2e8f0" }}>
          <h2 style={{ fontSize: 16, fontWeight: 600, marginBottom: 12, display: "flex", alignItems: "center", gap: 8 }}>
            <ListTodo size={16} /> Recent Jobs
          </h2>
          {jobs.length === 0 ? (
            <p style={{ color: "#94a3b8", fontSize: 13 }}>No jobs yet. Click "Scan All Roots" to start.</p>
          ) : (
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
              <thead>
                <tr style={{ borderBottom: "1px solid #e2e8f0" }}>
                  <th style={{ textAlign: "left", padding: "6px 0", color: "#64748b" }}>Type</th>
                  <th style={{ textAlign: "left", padding: "6px 0", color: "#64748b" }}>Status</th>
                  <th style={{ textAlign: "left", padding: "6px 0", color: "#64748b" }}>Created</th>
                </tr>
              </thead>
              <tbody>
                {jobs.slice(0, 5).map((job) => (
                  <tr key={job.id} style={{ borderBottom: "1px solid #f1f5f9" }}>
                    <td style={{ padding: "6px 0" }}>{job.type}</td>
                    <td style={{ padding: "6px 0" }}><StatusBadge status={job.status} /></td>
                    <td style={{ padding: "6px 0", color: "#64748b", fontSize: 12 }}>
                      {new Date(job.createdAt).toLocaleString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {/* Notifications */}
        <div style={{ background: "#fff", borderRadius: 8, padding: 20, border: "1px solid #e2e8f0" }}>
          <h2 style={{ fontSize: 16, fontWeight: 600, marginBottom: 12, display: "flex", alignItems: "center", gap: 8 }}>
            <Bell size={16} /> Notifications
            {unreadCount > 0 && (
              <span style={{ background: "#ef4444", color: "#fff", borderRadius: 10, padding: "1px 6px", fontSize: 11, fontWeight: 600 }}>
                {unreadCount}
              </span>
            )}
          </h2>
          {notifications.length === 0 ? (
            <p style={{ color: "#94a3b8", fontSize: 13 }}>No unread notifications.</p>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {notifications.map((n) => (
                <div key={n.id} style={{ padding: 8, background: "#f8fafc", borderRadius: 6, fontSize: 13 }}>
                  <div style={{ fontWeight: 600, marginBottom: 2 }}>{n.title}</div>
                  <div style={{ color: "#64748b", fontSize: 12 }}>{n.message}</div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Roots quick view */}
      {roots.length > 0 && (
        <div style={{ marginTop: 16, background: "#fff", borderRadius: 8, padding: 20, border: "1px solid #e2e8f0" }}>
          <h2 style={{ fontSize: 16, fontWeight: 600, marginBottom: 12, display: "flex", alignItems: "center", gap: 8 }}>
            <FolderTree size={16} /> Workspace Roots
          </h2>
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {roots.map((root) => (
              <div key={root.id} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 0", fontSize: 13 }}>
                <span style={{
                  width: 8, height: 8, borderRadius: "50%",
                  background: root.scanEnabled ? "#10b981" : "#94a3b8",
                  display: "inline-block",
                }} />
                <span style={{ fontWeight: 500 }}>{root.label}</span>
                <span style={{ color: "#94a3b8", fontSize: 12, fontFamily: "monospace" }}>{root.path}</span>
                <span style={{
                  padding: "1px 6px", borderRadius: 3, fontSize: 10, fontWeight: 600,
                  background: root.accessMode === "read_only" ? "#fef3c7" : "#dcfce7",
                  color: root.accessMode === "read_only" ? "#92400e" : "#166534",
                }}>
                  {root.accessMode === "read_only" ? "RO" : "RW"}
                </span>
                {root.lastScannedAt && (
                  <span style={{ color: "#94a3b8", fontSize: 11 }}>
                    scanned {new Date(root.lastScannedAt).toLocaleString()}
                  </span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function StatCard({ icon: Icon, label, value, color }: { icon: any; label: string; value: number; color: string }) {
  return (
    <div style={{ background: "#fff", borderRadius: 8, padding: 16, border: "1px solid #e2e8f0" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
        <Icon size={18} color={color} />
        <span style={{ fontSize: 12, color: "#64748b", fontWeight: 500 }}>{label}</span>
      </div>
      <div style={{ fontSize: 28, fontWeight: 700, color }}>{value}</div>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, { bg: string; fg: string }> = {
    pending: { bg: "#fef3c7", fg: "#92400e" },
    running: { bg: "#dbeafe", fg: "#1e40af" },
    completed: { bg: "#dcfce7", fg: "#166534" },
    failed: { bg: "#fef2f2", fg: "#991b1b" },
    cancelled: { bg: "#f1f5f9", fg: "#475569" },
  };
  const c = colors[status] ?? { bg: "#f1f5f9", fg: "#475569" };
  return (
    <span style={{ padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600, background: c.bg, color: c.fg }}>
      {status}
    </span>
  );
}
