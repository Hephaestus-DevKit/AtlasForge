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
        <h1 style={{ fontSize: 24, fontWeight: 700, marginBottom: 24 }}>Assets</h1>
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
          <FolderOpen size={48} color="#94a3b8" strokeWidth={1.5} />
          <h2 style={{ fontSize: 18, fontWeight: 600, color: "#334155", marginTop: 16, marginBottom: 8 }}>
            No workspace roots yet
          </h2>
          <p style={{ color: "#64748b", fontSize: 14, textAlign: "center", maxWidth: 420, marginBottom: 20 }}>
            Add a directory to scan for repositories. AtlasForge will discover Git projects
            and build health profiles. Your files stay local — nothing is uploaded.
          </p>
          <button
            onClick={() => setShowForm(true)}
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
            Add Your First Root
          </button>
        </div>
      </div>
    );
  }

  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 24 }}>
        <h1 style={{ fontSize: 24, fontWeight: 700 }}>Assets</h1>
        <div style={{ display: "flex", gap: 8 }}>
          <button
            onClick={handleScanAll}
            disabled={scanning || roots.length === 0}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              padding: "8px 16px",
              background: scanning || roots.length === 0 ? "#94a3b8" : "#10b981",
              color: "#fff",
              border: "none",
              borderRadius: 6,
              cursor: scanning || roots.length === 0 ? "not-allowed" : "pointer",
              fontSize: 14,
              fontWeight: 600,
            }}
          >
            <Scan size={16} />
            {scanning ? "Scanning..." : "Scan All Roots"}
          </button>
          <button
            onClick={() => setShowForm(!showForm)}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              padding: "8px 16px",
              background: "#3b82f6",
              color: "#fff",
              border: "none",
              borderRadius: 6,
              cursor: "pointer",
              fontSize: 14,
              fontWeight: 600,
            }}
          >
            <Plus size={16} />
            Add Root
          </button>
        </div>
      </div>

      {error && (
        <div style={{ padding: 12, background: "#fef2f2", border: "1px solid #fca5a5", borderRadius: 6, marginBottom: 16, color: "#991b1b" }}>
          {error}
        </div>
      )}

      {showForm && (
        <div style={{ background: "#fff", borderRadius: 8, padding: 20, border: "1px solid #e2e8f0", marginBottom: 16 }}>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 16 }}>Add Workspace Root</h3>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 12 }}>
            <div>
              <label htmlFor="af-root-path" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>Path</label>
              <div style={{ display: "flex", gap: 6 }}>
                <input
                  id="af-root-path"
                  value={form.path}
                  onChange={(e) => setForm({ ...form, path: e.target.value })}
                  placeholder="C:\Users\you\projects"
                  style={{ flex: 1, padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 13 }}
                />
                <button
                  onClick={handlePickFolder}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 4,
                    padding: "8px 12px",
                    border: "1px solid #e2e8f0",
                    borderRadius: 4,
                    background: "#f8fafc",
                    cursor: "pointer",
                    fontSize: 12,
                    whiteSpace: "nowrap",
                  }}
                  title="Browse for folder"
                >
                  <FolderOpen size={14} />
                  Browse
                </button>
              </div>
            </div>
            <div>
              <label htmlFor="af-root-label" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>Label</label>
              <input
                id="af-root-label"
                value={form.label}
                onChange={(e) => setForm({ ...form, label: e.target.value })}
                placeholder="My Projects"
                style={{ width: "100%", padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 13 }}
              />
            </div>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 12 }}>
            <div>
              <label htmlFor="af-root-access" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>Access Mode</label>
              <select
                id="af-root-access"
                value={form.accessMode}
                onChange={(e) => setForm({ ...form, accessMode: e.target.value as any })}
                style={{ width: "100%", padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 13 }}
              >
                <option value="read_write">Read & Write</option>
                <option value="read_only">Read Only</option>
              </select>
            </div>
            <div>
              <label htmlFor="af-root-scan" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>Scan Enabled</label>
              <input
                id="af-root-scan"
                type="checkbox"
                checked={form.scanEnabled}
                onChange={(e) => setForm({ ...form, scanEnabled: e.target.checked })}
                style={{ marginTop: 8 }}
              />
            </div>
          </div>
          <div style={{ marginBottom: 12 }}>
            <label htmlFor="af-root-exclude" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>
              Exclude Globs (comma-separated)
            </label>
            <input
              id="af-root-exclude"
              value={form.excludeGlobs.join(", ")}
              onChange={(e) => setForm({ ...form, excludeGlobs: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })}
              style={{ width: "100%", padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 13, fontFamily: "monospace" }}
            />
          </div>
          <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
            <button onClick={() => setShowForm(false)} style={{ padding: "8px 16px", border: "1px solid #e2e8f0", borderRadius: 6, background: "#fff", cursor: "pointer", fontSize: 13 }}>
              Cancel
            </button>
            <button
              onClick={handleAdd}
              disabled={!form.path.trim()}
              style={{ padding: "8px 16px", border: "none", borderRadius: 6, background: form.path.trim() ? "#3b82f6" : "#94a3b8", color: "#fff", cursor: form.path.trim() ? "pointer" : "not-allowed", fontSize: 13, fontWeight: 600 }}
            >
              Add Root
            </button>
          </div>
        </div>
      )}

      {roots.length === 0 ? (
        <p style={{ color: "#94a3b8", fontSize: 14 }}>No workspace roots configured.</p>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {roots.map((root) => (
            <div
              key={root.id}
              style={{ background: "#fff", borderRadius: 8, padding: 16, border: "1px solid #e2e8f0" }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                <div style={{ flex: 1 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                    <FolderTree size={16} color="#3b82f6" />
                    <span style={{ fontWeight: 600, fontSize: 14 }}>{root.label}</span>
                    <span
                      style={{
                        padding: "2px 8px",
                        borderRadius: 4,
                        fontSize: 11,
                        fontWeight: 600,
                        background: root.accessMode === "read_only" ? "#fef3c7" : "#dcfce7",
                        color: root.accessMode === "read_only" ? "#92400e" : "#166534",
                      }}
                    >
                      {root.accessMode === "read_only" ? "Read Only" : "Read & Write"}
                    </span>
                  </div>
                  <div style={{ fontSize: 12, color: "#64748b", fontFamily: "monospace", marginBottom: 6 }}>
                    {root.path}
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 12, fontSize: 12 }}>
                    <span style={{ color: root.scanEnabled ? "#10b981" : "#475569" }}>
                      {root.scanEnabled ? "Scanning" : "Paused"}
                    </span>
                    {root.lastScannedAt && (
                      <span style={{ fontSize: 11, color: "#94a3b8" }}>
                        Last scanned: {new Date(root.lastScannedAt).toLocaleString()}
                      </span>
                    )}
                  </div>
                  {root.excludeGlobs.length > 0 && (
                    <div style={{ marginTop: 6, fontSize: 11, color: "#94a3b8" }}>
                      Exclude: {root.excludeGlobs.join(", ")}
                    </div>
                  )}
                  {(scanErrors[root.id]?.length ?? 0) > 0 && (
                    <div style={{ marginTop: 8 }}>
                      <button
                        onClick={() => setExpandedErrors(expandedErrors === root.id ? null : root.id)}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 4,
                          background: "none",
                          border: "none",
                          cursor: "pointer",
                          fontSize: 12,
                          color: "#b45309",
                          padding: 0,
                        }}
                      >
                        <AlertTriangle size={12} />
                        {scanErrors[root.id].length} scan error(s)
                        {expandedErrors === root.id ? " ▲" : " ▼"}
                      </button>
                      {expandedErrors === root.id && (
                        <div style={{ marginTop: 6, maxHeight: 160, overflowY: "auto", fontSize: 11, background: "#fef3c7", borderRadius: 4, padding: 8 }}>
                          {scanErrors[root.id].map((err) => (
                            <div key={err.id} style={{ marginBottom: 4, borderBottom: "1px solid #fde68a", paddingBottom: 4 }}>
                              <div style={{ fontWeight: 600, color: "#92400e" }}>{err.errorType}</div>
                              {err.path && <div style={{ fontFamily: "monospace", color: "#78350f" }}>{err.path}</div>}
                              <div style={{ color: "#92400e" }}>{err.message}</div>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                </div>
                <div style={{ display: "flex", gap: 6 }}>
                  <button
                    onClick={() => handleScanRoot(root.id)}
                    disabled={scanningRootId === root.id || !root.scanEnabled}
                    title="Scan this root"
                    style={{
                      padding: "6px 10px",
                      border: "1px solid #d1d5db",
                      borderRadius: 4,
                      background: "#fff",
                      color: "#374151",
                      cursor: scanningRootId === root.id || !root.scanEnabled ? "not-allowed" : "pointer",
                      fontSize: 12,
                      display: "flex",
                      alignItems: "center",
                      gap: 4,
                    }}
                  >
                    <Play size={12} />
                    {scanningRootId === root.id ? "Scanning" : "Scan"}
                  </button>
                  <button
                    onClick={() => handleRemove(root.id)}
                    title="Remove root"
                    style={{ padding: "6px 10px", border: "1px solid #fca5a5", borderRadius: 4, background: "#fff", color: "#991b1b", cursor: "pointer", fontSize: 12, display: "flex", alignItems: "center", gap: 4 }}
                  >
                    <Trash2 size={12} />
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
