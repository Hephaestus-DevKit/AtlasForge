import { useEffect, useRef, useState } from "react";
import { listWorkspaceRoots, addWorkspaceRoot, removeWorkspaceRoot, startScan, pickFolder, listScanErrors } from "../api/ipc";
import type { WorkspaceRoot, AddRootInput, ScanResult, ScanErrorRecord } from "../types";
import { Plus, Trash2, FolderTree, FolderOpen, Play, Scan, AlertTriangle } from "lucide-react";
import { ConfirmModal } from "../components/ConfirmModal";
import { ToastContainer, type ToastMessage } from "../components/Toast";

const DEFAULT_EXCLUDE_GLOBS = [
  "node_modules",
  ".git/objects",
  "dist",
  "build",
  ".env",
  "__pycache__",
  ".next",
  ".cache",
  "target",
  "*.pyc",
];

export function Assets() {
  const [roots, setRoots] = useState<WorkspaceRoot[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [scanning, setScanning] = useState(false);
  const [scanningRootId, setScanningRootId] = useState<string | null>(null);
  const [scanErrors, setScanErrors] = useState<Record<string, ScanErrorRecord[]>>({});
  const [expandedErrors, setExpandedErrors] = useState<string | null>(null);
  const [form, setForm] = useState<AddRootInput>({
    path: "",
    label: "",
    accessMode: "read_write",
    scanEnabled: true,
    includeGlobs: [],
    excludeGlobs: DEFAULT_EXCLUDE_GLOBS,
  });
  const [confirmAction, setConfirmAction] = useState<{ message: string; onConfirm: () => void } | null>(null);
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const toastCounter = useRef(0);

  function showToast(message: string, type: ToastMessage["type"] = "info") {
    const id = ++toastCounter.current;
    setToasts((prev) => [...prev, { id, message, type }]);
  }

  function dismissToast(id: number) {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }

  useEffect(() => {
    loadRoots();
  }, []);

  async function loadRoots() {
    try {
      setError(null);
      const loaded = await listWorkspaceRoots();
      setRoots(loaded);
      // Load scan errors for each root
      const errorsMap: Record<string, ScanErrorRecord[]> = {};
      for (const root of loaded) {
        try {
          errorsMap[root.id] = await listScanErrors(root.id);
        } catch {
          errorsMap[root.id] = [];
        }
      }
      setScanErrors(errorsMap);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load roots");
    }
  }

  async function handlePickFolder() {
    try {
      setError(null);
      const folder = await pickFolder();
      if (folder) {
        setForm((prev) => ({ ...prev, path: folder }));
      }
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to open folder picker");
    }
  }

  async function handleAdd() {
    try {
      setError(null);
      await addWorkspaceRoot(form);
      setShowForm(false);
      setForm({
        path: "",
        label: "",
        accessMode: "read_write",
        scanEnabled: true,
        includeGlobs: [],
        excludeGlobs: DEFAULT_EXCLUDE_GLOBS,
      });
      await loadRoots();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to add root");
    }
  }

  async function handleRemove(id: string) {
    setConfirmAction({
      message: "Remove this workspace root? Associated assets and repositories will also be removed.",
      onConfirm: async () => {
        setConfirmAction(null);
        try {
          setError(null);
          await removeWorkspaceRoot(id);
          showToast("Root removed", "success");
          await loadRoots();
        } catch (e: any) {
          setError(e?.toString() ?? "Failed to remove root");
        }
      },
    });
  }

  async function handleScanRoot(rootId: string) {
    try {
      setScanningRootId(rootId);
      setError(null);
      const result: ScanResult = await startScan([rootId]);
      await loadRoots();
      if (result.errors.length > 0) {
        setError(`Scan completed with ${result.errors.length} error(s): ${result.errors.slice(0, 3).join("; ")}`);
      }
    } catch (e: any) {
      setError(e?.toString() ?? "Scan failed");
    } finally {
      setScanningRootId(null);
    }
  }

  async function handleScanAll() {
    try {
      setScanning(true);
      setError(null);
      const result: ScanResult = await startScan();
      await loadRoots();
      if (result.errors.length > 0) {
        setError(`Scan completed with ${result.errors.length} error(s): ${result.errors.slice(0, 3).join("; ")}`);
      }
    } catch (e: any) {
      setError(e?.toString() ?? "Scan failed");
    } finally {
      setScanning(false);
    }
  }

  // First-run empty state
  if (roots.length === 0 && !showForm) {
    return (
      <div>
        <h1 style={{ fontSize: 26, fontWeight: 800, marginBottom: 24, letterSpacing: "-0.025em" }}>Assets</h1>
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
          <FolderOpen size={56} color="var(--color-primary)" strokeWidth={1.5} style={{ filter: "drop-shadow(0 0 10px rgba(99, 102, 241, 0.4))" }} />
          <h2 style={{ fontSize: 18, fontWeight: 700, color: "var(--text-primary)", marginTop: 20, marginBottom: 8 }}>
            No workspace roots yet
          </h2>
          <p style={{ color: "var(--text-secondary)", fontSize: 14, textAlign: "center", maxWidth: 460, marginBottom: 24, lineHeight: 1.6 }}>
            Add a directory to scan for repositories. AtlasForge will discover Git projects
            and build health profiles. Your files stay local — nothing is uploaded.
          </p>
          <button
            onClick={() => setShowForm(true)}
            className="btn btn-primary"
            style={{ padding: "10px 24px", fontSize: 14 }}
          >
            <Plus size={18} />
            Add Your First Root
          </button>
        </div>
      </div>
    );
  }

  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 28 }}>
        <h1 style={{ fontSize: 26, fontWeight: 800, letterSpacing: "-0.025em" }}>Assets</h1>
        <div style={{ display: "flex", gap: 10 }}>
          <button
            onClick={handleScanAll}
            disabled={scanning || roots.length === 0}
            className="btn btn-success"
            style={{
              background: scanning || roots.length === 0 ? "rgba(255,255,255,0.05)" : "var(--color-success)",
              color: "#fff",
              border: "none",
              boxShadow: scanning || roots.length === 0 ? "none" : "0 2px 10px rgba(16, 185, 129, 0.3)",
            }}
          >
            <Scan size={16} className={scanning ? "spin-slow" : ""} />
            {scanning ? "Scanning..." : "Scan All Roots"}
          </button>
          <button
            onClick={() => setShowForm(!showForm)}
            className="btn btn-primary"
          >
            <Plus size={16} />
            Add Root
          </button>
        </div>
      </div>

      {error && (
        <div className="badge badge-danger" style={{ display: "block", width: "100%", padding: 12, borderRadius: "var(--radius-sm)", marginBottom: 20, fontSize: 13 }}>
          {error}
        </div>
      )}

      {showForm && (
        <div className="card" style={{ marginBottom: 24, background: "rgba(16, 20, 38, 0.8)" }}>
          <h3 style={{ fontSize: 15, fontWeight: 700, marginBottom: 20, color: "var(--text-primary)" }}>Add Workspace Root</h3>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 16 }}>
            <div>
              <label htmlFor="af-root-path" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>Path</label>
              <div style={{ display: "flex", gap: 8 }}>
                <input
                  id="af-root-path"
                  value={form.path}
                  onChange={(e) => setForm({ ...form, path: e.target.value })}
                  placeholder="C:\Users\you\projects"
                  className="input-field"
                />
                <button
                  onClick={handlePickFolder}
                  className="btn btn-secondary"
                  style={{ display: "flex", alignItems: "center", gap: 6, padding: "8px 14px" }}
                  title="Browse for folder"
                >
                  <FolderOpen size={14} />
                  Browse
                </button>
              </div>
            </div>
            <div>
              <label htmlFor="af-root-label" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>Label</label>
              <input
                id="af-root-label"
                value={form.label}
                onChange={(e) => setForm({ ...form, label: e.target.value })}
                placeholder="My Projects"
                className="input-field"
              />
            </div>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 16 }}>
            <div>
              <label htmlFor="af-root-access" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>Access Mode</label>
              <select
                id="af-root-access"
                value={form.accessMode}
                onChange={(e) => setForm({ ...form, accessMode: e.target.value as any })}
                className="select-field"
                style={{ width: "100%" }}
              >
                <option value="read_write">Read & Write</option>
                <option value="read_only">Read Only</option>
              </select>
            </div>
            <div style={{ display: "flex", flexDirection: "column" }}>
              <span style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 10, fontWeight: 600 }}>Scan Enabled</span>
              <label htmlFor="af-root-scan" style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, cursor: "pointer", color: "var(--text-primary)" }}>
                <input
                  id="af-root-scan"
                  type="checkbox"
                  checked={form.scanEnabled}
                  onChange={(e) => setForm({ ...form, scanEnabled: e.target.checked })}
                  style={{ width: 16, height: 16, accentColor: "var(--color-primary)", cursor: "pointer" }}
                />
                Enable automatic background scanning
              </label>
            </div>
          </div>
          <div style={{ marginBottom: 20 }}>
            <label htmlFor="af-root-exclude" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>
              Exclude Globs (comma-separated)
            </label>
            <input
              id="af-root-exclude"
              value={form.excludeGlobs.join(", ")}
              onChange={(e) => setForm({ ...form, excludeGlobs: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })}
              className="input-field"
              style={{ fontFamily: "var(--font-mono)", fontSize: 12 }}
            />
          </div>
          <div style={{ display: "flex", gap: 10, justifyContent: "flex-end" }}>
            <button onClick={() => setShowForm(false)} className="btn btn-secondary">
              Cancel
            </button>
            <button
              onClick={handleAdd}
              disabled={!form.path.trim()}
              className="btn btn-primary"
            >
              Add Root
            </button>
          </div>
        </div>
      )}

      {roots.length === 0 ? (
        <p style={{ color: "var(--text-secondary)", fontSize: 14 }}>No workspace roots configured.</p>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
          {roots.map((root) => (
            <div
              key={root.id}
              className="card card-interactive"
              style={{ display: "flex", flexDirection: "column", gap: 12, padding: 20 }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12 }}>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ display: "flex", alignItems: "center", flexWrap: "wrap", gap: 8, marginBottom: 6 }}>
                    <FolderTree size={18} color="var(--color-primary)" style={{ filter: "drop-shadow(0 0 5px rgba(99, 102, 241, 0.3))" }} />
                    <span style={{ fontWeight: 700, fontSize: 15, color: "var(--text-primary)" }}>{root.label}</span>
                    <span
                      className={root.accessMode === "read_only" ? "badge badge-warning" : "badge badge-success"}
                    >
                      {root.accessMode === "read_only" ? "Read Only" : "Read & Write"}
                    </span>
                  </div>
                  <div style={{ fontSize: 12, color: "var(--text-secondary)", fontFamily: "var(--font-mono)", marginBottom: 8, wordBreak: "break-all" }}>
                    {root.path}
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 16, fontSize: 12 }}>
                    <span style={{ display: "flex", alignItems: "center", gap: 6, color: root.scanEnabled ? "var(--color-success-text)" : "var(--text-secondary)", fontWeight: 500 }}>
                      <span style={{ width: 6, height: 6, borderRadius: "50%", background: root.scanEnabled ? "var(--color-success)" : "var(--text-muted)", boxShadow: root.scanEnabled ? "0 0 6px var(--color-success)" : "none" }} />
                      {root.scanEnabled ? "Scanning Active" : "Scanning Paused"}
                    </span>
                    {root.lastScannedAt && (
                      <span style={{ color: "var(--text-muted)" }}>
                        Last scanned: {new Date(root.lastScannedAt).toLocaleString()}
                      </span>
                    )}
                  </div>
                  {root.excludeGlobs.length > 0 && (
                    <div style={{ marginTop: 8, fontSize: 11, color: "var(--text-muted)", wordBreak: "break-all" }}>
                      <span style={{ fontWeight: 600 }}>Exclude:</span> {root.excludeGlobs.join(", ")}
                    </div>
                  )}
                  {(scanErrors[root.id]?.length ?? 0) > 0 && (
                    <div style={{ marginTop: 12 }}>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          setExpandedErrors(expandedErrors === root.id ? null : root.id);
                        }}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 6,
                          background: "none",
                          border: "none",
                          cursor: "pointer",
                          fontSize: 12,
                          color: "var(--color-danger-text)",
                          fontWeight: 600,
                          padding: 0,
                          outline: "none",
                        }}
                      >
                        <AlertTriangle size={14} />
                        {scanErrors[root.id].length} scan error(s)
                        {expandedErrors === root.id ? " ▲" : " ▼"}
                      </button>
                      {expandedErrors === root.id && (
                        <div
                          onClick={(e) => e.stopPropagation()}
                          style={{ marginTop: 8, maxHeight: 200, overflowY: "auto", fontSize: 12, background: "var(--color-danger-bg)", border: "1px solid var(--color-danger-border)", borderRadius: "var(--radius-sm)", padding: 12 }}
                          className="scrollbar-custom"
                        >
                          {scanErrors[root.id].map((err) => (
                            <div key={err.id} style={{ marginBottom: 8, borderBottom: "1px solid rgba(239, 68, 68, 0.15)", paddingBottom: 8 }}>
                              <div style={{ fontWeight: 700, color: "var(--color-danger-text)", marginBottom: 2 }}>{err.errorType}</div>
                              {err.path && <div style={{ fontFamily: "var(--font-mono)", color: "var(--text-secondary)", fontSize: 11, marginBottom: 2 }}>{err.path}</div>}
                              <div style={{ color: "var(--text-primary)" }}>{err.message}</div>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                </div>
                <div style={{ display: "flex", gap: 8, flexShrink: 0 }}>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleScanRoot(root.id);
                    }}
                    disabled={scanningRootId === root.id || !root.scanEnabled}
                    title="Scan this root"
                    className="btn btn-secondary"
                    style={{ padding: "6px 12px" }}
                  >
                    <Play size={12} className={scanningRootId === root.id ? "spin-slow" : ""} />
                    {scanningRootId === root.id ? "Scanning" : "Scan"}
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleRemove(root.id);
                    }}
                    title="Remove root"
                    className="btn btn-danger"
                    style={{ padding: "6px 10px" }}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
      {confirmAction && (
        <ConfirmModal
          message={confirmAction.message}
          onConfirm={confirmAction.onConfirm}
          onCancel={() => setConfirmAction(null)}
        />
      )}
      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
}
