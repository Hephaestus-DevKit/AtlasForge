import { useEffect, useState } from "react";
import { listWorkspaceRoots, listRepositories, listJobs, startScan, listNotifications } from "../api/ipc";
import type { WorkspaceRoot, Repository, Job, Notification } from "../types";
import { FolderTree, GitBranch, ListTodo, AlertTriangle, Bell, Plus, Scan, type LucideIcon } from "lucide-react";
import { navigateTo } from "../routing";
import { errorMessage } from "../utils/errors";

export function Dashboard() {
  const [roots, setRoots] = useState<WorkspaceRoot[]>([]);
  const [repos, setRepos] = useState<Repository[]>([]);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);

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
    } catch (e) {
      setError(errorMessage(e, "Failed to load data"));
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
    } catch (e) {
      setError(errorMessage(e, "Scan failed"));
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
        <h1 style={{ fontSize: 26, fontWeight: 800, marginBottom: 24, letterSpacing: "-0.025em" }}>Dashboard</h1>
        {error && (
          <div className="badge badge-danger" style={{ display: "block", width: "100%", padding: 12, borderRadius: "var(--radius-sm)", marginBottom: 16, fontSize: 13 }}>
            {error}
          </div>
        )}
        <div style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          padding: "80px 24px",
          background: "var(--bg-card)",
          borderRadius: "var(--radius-lg)",
          border: "1px dashed var(--border-color)",
          boxShadow: "var(--shadow-lg)",
        }}>
          <FolderTree size={56} color="var(--color-primary)" strokeWidth={1.5} style={{ filter: "drop-shadow(0 0 10px rgba(99, 102, 241, 0.4))" }} />
          <h2 style={{ fontSize: 18, fontWeight: 700, color: "var(--text-primary)", marginTop: 20, marginBottom: 8 }}>
            Welcome to AtlasForge
          </h2>
          <p style={{ color: "var(--text-secondary)", fontSize: 14, textAlign: "center", maxWidth: 460, marginBottom: 24, lineHeight: 1.6 }}>
            Add a workspace root to start discovering repositories, running health audits,
            and managing your projects. Everything stays on your machine.
          </p>
          <button
            onClick={() => navigateTo("/assets")}
            className="btn btn-primary"
            style={{ padding: "10px 24px", fontSize: 14 }}
          >
            <Plus size={18} />
            Add Workspace Root
          </button>
        </div>
      </div>
    );
  }

  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 28 }}>
        <h1 style={{ fontSize: 26, fontWeight: 800, letterSpacing: "-0.025em" }}>Dashboard</h1>
        <button
          onClick={handleScan}
          disabled={scanning}
          className="btn btn-success"
          style={{
            background: scanning ? "rgba(255,255,255,0.05)" : "var(--color-success)",
            color: "#fff",
            border: "none",
            boxShadow: scanning ? "none" : "0 2px 10px rgba(16, 185, 129, 0.3)",
          }}
        >
          <Scan size={16} className={scanning ? "spin-slow" : ""} />
          {scanning ? "Scanning All Roots..." : "Scan All Roots"}
        </button>
      </div>

      {error && (
        <div className="badge badge-danger" style={{ display: "block", width: "100%", padding: 12, borderRadius: "var(--radius-sm)", marginBottom: 20, fontSize: 13 }}>
          {error}
        </div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 20, marginBottom: 28 }}>
        <StatCard icon={FolderTree} label="Workspace Roots" value={roots.length} color="var(--color-primary)" glowColor="rgba(99, 102, 241, 0.15)" />
        <StatCard icon={GitBranch} label="Repositories" value={repos.length} color="var(--color-success-text)" glowColor="rgba(16, 185, 129, 0.15)" />
        <StatCard icon={AlertTriangle} label="Dirty Repos" value={dirtyRepos.length} color="var(--color-warning-text)" glowColor="rgba(245, 158, 11, 0.15)" />
        <StatCard icon={ListTodo} label="Running Jobs" value={runningJobs.length} color="var(--color-accent)" glowColor="rgba(168, 85, 247, 0.15)" />
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 20, marginBottom: 20 }}>
        {/* Recent Jobs */}
        <div className="card">
          <h2 style={{ fontSize: 16, fontWeight: 700, marginBottom: 16, display: "flex", alignItems: "center", gap: 8 }}>
            <ListTodo size={18} color="var(--color-primary)" /> Recent Jobs
          </h2>
          {jobs.length === 0 ? (
            <p style={{ color: "var(--text-secondary)", fontSize: 13 }}>No jobs yet. Click "Scan All Roots" to start.</p>
          ) : (
            <div style={{ overflowX: "auto" }}>
              <table className="custom-table" style={{ fontSize: 13 }}>
                <thead>
                  <tr>
                    <th style={{ padding: "8px 12px" }}>Type</th>
                    <th style={{ padding: "8px 12px" }}>Status</th>
                    <th style={{ padding: "8px 12px" }}>Created</th>
                  </tr>
                </thead>
                <tbody>
                  {jobs.slice(0, 5).map((job) => (
                    <tr key={job.id} className="table-row-interactive">
                      <td style={{ padding: "10px 12px", fontWeight: 600 }}>{job.type}</td>
                      <td style={{ padding: "10px 12px" }}><StatusBadge status={job.status} /></td>
                      <td style={{ padding: "10px 12px", color: "var(--text-secondary)", fontSize: 12 }}>
                        {new Date(job.createdAt).toLocaleString()}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>

        {/* Notifications */}
        <div className="card">
          <h2 style={{ fontSize: 16, fontWeight: 700, marginBottom: 16, display: "flex", alignItems: "center", gap: 8 }}>
            <Bell size={18} color="var(--color-accent)" /> Notifications
            {unreadCount > 0 && (
              <span className="badge badge-danger" style={{ borderRadius: 12, padding: "2px 8px", fontSize: 10, filter: "drop-shadow(0 0 5px rgba(239, 68, 68, 0.4))" }}>
                {unreadCount} new
              </span>
            )}
          </h2>
          {notifications.length === 0 ? (
            <p style={{ color: "var(--text-secondary)", fontSize: 13 }}>No unread notifications.</p>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {notifications.map((n) => (
                <div key={n.id} style={{ padding: 12, background: "rgba(255, 255, 255, 0.02)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", fontSize: 13 }}>
                  <div style={{ fontWeight: 700, marginBottom: 2, color: "var(--text-primary)" }}>{n.title}</div>
                  <div style={{ color: "var(--text-secondary)", fontSize: 12, lineHeight: 1.4 }}>{n.message}</div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Roots quick view */}
      {roots.length > 0 && (
        <div className="card">
          <h2 style={{ fontSize: 16, fontWeight: 700, marginBottom: 16, display: "flex", alignItems: "center", gap: 8 }}>
            <FolderTree size={18} color="var(--color-primary)" /> Workspace Roots
          </h2>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {roots.map((root) => (
              <div key={root.id} style={{ display: "flex", alignItems: "center", flexWrap: "wrap", gap: 10, padding: 10, background: "rgba(255, 255, 255, 0.01)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", fontSize: 13 }}>
                <span style={{
                  width: 8, height: 8, borderRadius: "50%",
                  background: root.scanEnabled ? "var(--color-success)" : "var(--text-muted)",
                  boxShadow: root.scanEnabled ? "0 0 8px var(--color-success)" : "none",
                  display: "inline-block",
                }} />
                <span style={{ fontWeight: 600, color: "var(--text-primary)" }}>{root.label}</span>
                <span style={{ color: "var(--text-secondary)", fontSize: 12, fontFamily: "var(--font-mono)", flex: 1, minWidth: 150 }}>{root.path}</span>
                <span className={root.accessMode === "read_only" ? "badge badge-warning" : "badge badge-success"}>
                  {root.accessMode === "read_only" ? "RO" : "RW"}
                </span>
                {root.lastScannedAt && (
                  <span style={{ color: "var(--text-muted)", fontSize: 11 }}>
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

function StatCard({ icon: Icon, label, value, color, glowColor }: { icon: LucideIcon; label: string; value: number; color: string; glowColor: string }) {
  return (
    <div className="card card-interactive" style={{ display: "flex", flexDirection: "column", justifyContent: "space-between", height: 110 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <div style={{
          padding: 6,
          borderRadius: 6,
          background: glowColor,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}>
          <Icon size={16} color={color} />
        </div>
        <span style={{ fontSize: 12, color: "var(--text-secondary)", fontWeight: 600 }}>{label}</span>
      </div>
      <div style={{ fontSize: 32, fontWeight: 800, color: "var(--text-primary)", letterSpacing: "-0.03em" }}>{value}</div>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const badgeClasses: Record<string, string> = {
    pending: "badge badge-warning",
    running: "badge badge-info pulse-glow",
    completed: "badge badge-success",
    failed: "badge badge-danger",
    cancelled: "badge badge-neutral",
  };
  const cls = badgeClasses[status] ?? "badge badge-neutral";
  return (
    <span className={cls}>
      {status}
    </span>
  );
}
