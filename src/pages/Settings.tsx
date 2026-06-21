import { useEffect, useRef, useState } from "react";
import {
  listAiProviders,
  detectLocalProviders,
  upsertAiProvider,
  deleteAiProvider,
  probeAiProvider,
  checkGhAuth,
} from "../api/ipc";
import type { AiProvider, GhAuthStatus, ProviderProbe } from "../types";
import { Cpu, Github, Plus, Trash2, Wifi, WifiOff, RefreshCw, Activity } from "lucide-react";
import { ConfirmModal } from "../components/ConfirmModal";
import { ToastContainer, type ToastMessage } from "../components/Toast";

const EMPTY_PROVIDER: AiProvider = {
  id: "",
  name: "",
  adapterType: "openai_compatible",
  baseUrl: "http://localhost:11434",
  defaultModel: "",
  apiKeyRef: null,
  enabled: true,
  availableModels: [],
  isLocal: false,
  isDefault: false,
  config: {},
};

export function Settings() {
  const [providers, setProviders] = useState<AiProvider[]>([]);
  const [ghAuth, setGhAuth] = useState<GhAuthStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showProviderForm, setShowProviderForm] = useState(false);
  const [form, setForm] = useState<AiProvider>(EMPTY_PROVIDER);
  const [detecting, setDetecting] = useState(false);
  const [probingProviderId, setProbingProviderId] = useState<string | null>(null);
  const [providerProbes, setProviderProbes] = useState<Record<string, ProviderProbe>>({});
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
    loadData();
  }, []);

  async function loadData() {
    try {
      setError(null);
      const [p, g] = await Promise.all([listAiProviders(), checkGhAuth()]);
      setProviders(p);
      setGhAuth(g);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load settings");
    }
  }

  async function handleDetect() {
    try {
      setDetecting(true);
      setError(null);
      const detected = await detectLocalProviders();
      if (detected.length === 0) {
        showToast("No local AI providers detected. Make sure Ollama or LM Studio is running.", "error");
      } else {
        for (const p of detected) {
          await upsertAiProvider(p);
        }
        showToast(`Detected ${detected.length} provider(s).`, "success");
        await loadData();
      }
    } catch (e: any) {
      setError(e?.toString() ?? "Detection failed");
    } finally {
      setDetecting(false);
    }
  }

  async function handleAddProvider() {
    try {
      setError(null);
      const provider = {
        ...form,
        id: form.id || crypto.randomUUID(),
        apiKeyRef: form.apiKeyRef?.trim() || null,
      };
      await upsertAiProvider(provider);
      setShowProviderForm(false);
      setForm(EMPTY_PROVIDER);
      await loadData();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to save provider");
    }
  }

  async function handleDeleteProvider(id: string) {
    setConfirmAction({
      message: "Delete this AI provider?",
      onConfirm: async () => {
        setConfirmAction(null);
        try {
          setError(null);
          await deleteAiProvider(id);
          showToast("Provider deleted", "success");
          await loadData();
        } catch (e: any) {
          setError(e?.toString() ?? "Failed to delete provider");
        }
      },
    });
  }

  async function handleProbeProvider(id: string) {
    try {
      setProbingProviderId(id);
      const probe = await probeAiProvider(id);
      setProviderProbes((current) => ({ ...current, [id]: probe }));
      showToast(probe.message, probe.reachable ? "success" : "error");
    } catch (e: any) {
      setError(e?.toString() ?? "Provider probe failed");
    } finally {
      setProbingProviderId(null);
    }
  }

  return (
    <div>
      <h1 style={{ fontSize: 24, fontWeight: 700, marginBottom: 24 }}>Settings</h1>

      {error && (
        <div style={{ padding: 12, background: "#fef2f2", border: "1px solid #fca5a5", borderRadius: 6, marginBottom: 16, color: "#991b1b" }}>
          {error}
        </div>
      )}

      <div className="settings-grid" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
        {/* AI Providers */}
        <div style={{ background: "#fff", borderRadius: 8, padding: 20, border: "1px solid #e2e8f0" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
            <h2 style={{ fontSize: 16, fontWeight: 600, display: "flex", alignItems: "center", gap: 8 }}>
              <Cpu size={16} /> AI Providers
            </h2>
            <div style={{ display: "flex", gap: 6 }}>
              <button
                onClick={handleDetect}
                disabled={detecting}
                style={{
                  display: "flex", alignItems: "center", gap: 4,
                  padding: "6px 12px", background: "#f0f9ff", color: "#0369a1",
                  border: "1px solid #bae6fd", borderRadius: 6, cursor: "pointer", fontSize: 13,
                }}
              >
                <RefreshCw size={14} />
                {detecting ? "Detecting..." : "Auto-detect"}
              </button>
              <button
                onClick={() => setShowProviderForm(!showProviderForm)}
                style={{
                  display: "flex", alignItems: "center", gap: 4,
                  padding: "6px 12px", background: "#3b82f6", color: "#fff",
                  border: "none", borderRadius: 6, cursor: "pointer", fontSize: 13, fontWeight: 600,
                }}
              >
                <Plus size={14} /> Add
              </button>
            </div>
          </div>

          {showProviderForm && (
            <div style={{ padding: 16, background: "#f8fafc", borderRadius: 6, marginBottom: 12, border: "1px solid #e2e8f0" }}>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 8 }}>
                <div>
                  <label htmlFor="af-provider-name" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>Name</label>
                  <input
                    id="af-provider-name"
                    value={form.name}
                    onChange={(e) => setForm({ ...form, name: e.target.value })}
                    placeholder="e.g., Ollama Local"
                    style={{ width: "100%", padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 14 }}
                  />
                </div>
                <div>
                  <label htmlFor="af-provider-type" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>Type</label>
                  <select
                    id="af-provider-type"
                    value={form.adapterType}
                    onChange={(e) => setForm({
                      ...form,
                      adapterType: e.target.value as AiProvider["adapterType"],
                    })}
                    style={{ width: "100%", padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 14 }}
                  >
                    <option value="ollama">Ollama</option>
                    <option value="openai_compatible">OpenAI Compatible</option>
                  </select>
                </div>
              </div>
              <div style={{ marginBottom: 8 }}>
                <label htmlFor="af-provider-url" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>Base URL</label>
                <input
                  id="af-provider-url"
                  value={form.baseUrl}
                  onChange={(e) => setForm({ ...form, baseUrl: e.target.value })}
                  placeholder="http://localhost:11434"
                  style={{ width: "100%", padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 14 }}
                />
              </div>
              <div style={{ marginBottom: 8 }}>
                <label htmlFor="af-provider-model" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>Model</label>
                <input
                  id="af-provider-model"
                  value={form.defaultModel}
                  onChange={(e) => setForm({ ...form, defaultModel: e.target.value })}
                  placeholder="e.g., llama3, gpt-4"
                  style={{ width: "100%", padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 14 }}
                />
              </div>
              <div style={{ marginBottom: 8 }}>
                <label htmlFor="af-provider-keyref" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>API Key Env Var (optional)</label>
                <input
                  id="af-provider-keyref"
                  value={form.apiKeyRef ?? ""}
                  onChange={(e) => setForm({ ...form, apiKeyRef: e.target.value || null })}
                  placeholder="OPENAI_API_KEY"
                  style={{ width: "100%", padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 14 }}
                />
              </div>
              <div style={{ display: "flex", gap: 8 }}>
                <label htmlFor="af-provider-enabled" style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 13 }}>
                  <input
                    type="checkbox"
                    id="af-provider-enabled"
                    checked={form.enabled}
                    onChange={(e) => setForm({ ...form, enabled: e.target.checked })}
                  />
                  Enabled
                </label>
              </div>
              <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
                <button onClick={handleAddProvider} style={{ padding: "8px 16px", background: "#10b981", color: "#fff", border: "none", borderRadius: 6, cursor: "pointer", fontSize: 13, fontWeight: 600 }}>
                  Save Provider
                </button>
                <button onClick={() => { setShowProviderForm(false); setForm(EMPTY_PROVIDER); }} style={{ padding: "8px 16px", background: "#f1f5f9", color: "#475569", border: "none", borderRadius: 6, cursor: "pointer", fontSize: 13 }}>
                  Cancel
                </button>
              </div>
            </div>
          )}

          {providers.length === 0 ? (
            <div style={{ textAlign: "center", padding: 24, color: "#94a3b8" }}>
              <Cpu size={32} style={{ marginBottom: 8, opacity: 0.5 }} />
              <p style={{ fontSize: 13 }}>No AI providers configured. Click "Auto-detect" to find local providers.</p>
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {providers.map((p) => (
                <div key={p.id} style={{ padding: 12, background: "#f8fafc", borderRadius: 6, border: "1px solid #e2e8f0", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div>
                    <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 2, display: "flex", alignItems: "center", gap: 6 }}>
                      {p.name}
                      {p.enabled ? <Wifi size={12} color="#10b981" /> : <WifiOff size={12} color="#94a3b8" />}
                    </div>
                    <div style={{ fontSize: 12, color: "#64748b" }}>
                      {p.adapterType} · {p.defaultModel} · {p.baseUrl}
                    </div>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    {providerProbes[p.id] && (
                      <span style={{ fontSize: 10, color: providerProbes[p.id].reachable ? "#166534" : "#991b1b" }}>
                        {providerProbes[p.id].reachable
                          ? `${providerProbes[p.id].latencyMs}ms · ${providerProbes[p.id].models.length} models`
                          : "unreachable"}
                      </span>
                    )}
                    <button
                      onClick={() => handleProbeProvider(p.id)}
                      disabled={probingProviderId === p.id}
                      title="Test provider connection"
                      style={{ background: "none", border: "none", cursor: "pointer", color: "#0f766e" }}
                    >
                      <Activity size={16} />
                    </button>
                    <button
                      onClick={() => handleDeleteProvider(p.id)}
                      title="Delete provider"
                      style={{ background: "none", border: "none", cursor: "pointer", color: "#ef4444" }}
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* GitHub Integration */}
        <div style={{ background: "#fff", borderRadius: 8, padding: 20, border: "1px solid #e2e8f0" }}>
          <h2 style={{ fontSize: 16, fontWeight: 600, marginBottom: 12, display: "flex", alignItems: "center", gap: 8 }}>
            <Github size={16} /> GitHub Integration
          </h2>

          {ghAuth === null ? (
            <p style={{ color: "#94a3b8", fontSize: 13 }}>Checking GitHub CLI status...</p>
          ) : ghAuth.authenticated ? (
            <div style={{ padding: 16, background: "#f0fdf4", borderRadius: 6, border: "1px solid #bbf7d0" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                <Wifi size={16} color="#166534" />
                <span style={{ fontWeight: 600, color: "#166534" }}>Authenticated</span>
              </div>
              <div style={{ fontSize: 13, color: "#475569" }}>
                Logged in as <strong>{ghAuth.username}</strong>
              </div>
              <div style={{ fontSize: 12, color: "#64748b", marginTop: 8 }}>
                Repository metadata can be synced. GitHub write operations remain disabled.
              </div>
            </div>
          ) : (
            <div style={{ padding: 16, background: "#fef3c7", borderRadius: 6, border: "1px solid #fde68a" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                <WifiOff size={16} color="#92400e" />
                <span style={{ fontWeight: 600, color: "#92400e" }}>Not Authenticated</span>
              </div>
              <div style={{ fontSize: 13, color: "#475569", marginBottom: 8 }}>
                Install the GitHub CLI and run <code style={{ background: "#f1f5f9", padding: "2px 6px", borderRadius: 3, fontSize: 12 }}>gh auth login</code> to enable GitHub integration.
              </div>
              {ghAuth.message && !ghAuth.authenticated && (
                <div style={{ fontSize: 12, color: "#92400e" }}>{ghAuth.message}</div>
              )}
            </div>
          )}

          {/* Permission Summary */}
          <div style={{ marginTop: 16 }}>
            <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>Permission Policy</h3>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
              <thead>
                <tr style={{ borderBottom: "1px solid #e2e8f0" }}>
                  <th style={{ textAlign: "left", padding: "6px 0", color: "#64748b" }}>Capability</th>
                  <th style={{ textAlign: "left", padding: "6px 0", color: "#64748b" }}>Risk Level</th>
                  <th style={{ textAlign: "left", padding: "6px 0", color: "#64748b" }}>Approval</th>
                </tr>
              </thead>
              <tbody>
                <tr style={{ borderBottom: "1px solid #f1f5f9" }}>
                  <td style={{ padding: "6px 0" }}>fs.read</td>
                  <td style={{ padding: "6px 0" }}><span style={{ padding: "1px 6px", borderRadius: 3, background: "#dcfce7", color: "#166534", fontSize: 11 }}>low</span></td>
                  <td style={{ padding: "6px 0", color: "#64748b" }}>Auto-approved</td>
                </tr>
                <tr style={{ borderBottom: "1px solid #f1f5f9" }}>
                  <td style={{ padding: "6px 0" }}>github.read</td>
                  <td style={{ padding: "6px 0" }}><span style={{ padding: "1px 6px", borderRadius: 3, background: "#dcfce7", color: "#166534", fontSize: 11 }}>none</span></td>
                  <td style={{ padding: "6px 0", color: "#64748b" }}>Auto-approved</td>
                </tr>
                <tr style={{ borderBottom: "1px solid #f1f5f9" }}>
                  <td style={{ padding: "6px 0" }}>github.create_pr</td>
                  <td style={{ padding: "6px 0" }}><span style={{ padding: "1px 6px", borderRadius: 3, background: "#fef3c7", color: "#92400e", fontSize: 11 }}>high</span></td>
                  <td style={{ padding: "6px 0", color: "#64748b" }}>Disabled</td>
                </tr>
                <tr style={{ borderBottom: "1px solid #f1f5f9" }}>
                  <td style={{ padding: "6px 0" }}>github.create_release</td>
                  <td style={{ padding: "6px 0" }}><span style={{ padding: "1px 6px", borderRadius: 3, background: "#fef2f2", color: "#991b1b", fontSize: 11 }}>critical</span></td>
                  <td style={{ padding: "6px 0", color: "#64748b" }}>Disabled</td>
                </tr>
                <tr style={{ borderBottom: "1px solid #f1f5f9" }}>
                  <td style={{ padding: "6px 0" }}>shell.verify</td>
                  <td style={{ padding: "6px 0" }}><span style={{ padding: "1px 6px", borderRadius: 3, background: "#fef3c7", color: "#92400e", fontSize: 11 }}>medium</span></td>
                  <td style={{ padding: "6px 0", color: "#64748b" }}>Single-use approval</td>
                </tr>
                <tr style={{ borderBottom: "1px solid #f1f5f9" }}>
                  <td style={{ padding: "6px 0" }}>fs.write_patch</td>
                  <td style={{ padding: "6px 0" }}><span style={{ padding: "1px 6px", borderRadius: 3, background: "#fef2f2", color: "#991b1b", fontSize: 11 }}>high</span></td>
                  <td style={{ padding: "6px 0", color: "#64748b" }}>Isolated approval</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
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
