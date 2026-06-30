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
import { Cpu, Github, Plus, Trash2, Wifi, WifiOff, RefreshCw, Activity, Edit } from "lucide-react";
import { ConfirmModal } from "../components/ConfirmModal";
import { ToastContainer, type ToastMessage } from "../components/Toast";

const EMPTY_PROVIDER: AiProvider = {
  id: "",
  name: "",
  adapterType: "openai_compatible",
  baseUrl: "https://api.openai.com/v1",
  defaultModel: "gpt-4o",
  apiKeyRef: "OPENAI_API_KEY",
  enabled: true,
  availableModels: [],
  isLocal: false,
  isDefault: false,
  config: {},
};

const PLACEHOLDERS: Record<string, { baseUrl: string; model: string; apiKeyRef: string }> = {
  ollama: {
    baseUrl: "http://localhost:11434",
    model: "e.g., llama3, qwen2.5",
    apiKeyRef: "Not required for local Ollama",
  },
  deepseek: {
    baseUrl: "https://api.deepseek.com",
    model: "e.g., deepseek-chat, deepseek-coder",
    apiKeyRef: "DEEPSEEK_API_KEY",
  },
  openai_compatible: {
    baseUrl: "https://api.openai.com/v1",
    model: "e.g., gpt-4o, gpt-4o-mini",
    apiKeyRef: "OPENAI_API_KEY",
  },
  anthropic: {
    baseUrl: "https://api.anthropic.com",
    model: "e.g., claude-3-5-sonnet-20241022",
    apiKeyRef: "ANTHROPIC_API_KEY",
  },
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
  const [configText, setConfigText] = useState("{}");
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

  function handleTypeChange(type: AiProvider["adapterType"]) {
    const defaults = {
      adapterType: type,
      baseUrl: "",
      defaultModel: "",
      apiKeyRef: null as string | null,
    };
    if (type === "ollama") {
      defaults.baseUrl = "http://localhost:11434";
      defaults.defaultModel = "llama3";
      defaults.apiKeyRef = null;
    } else if (type === "deepseek") {
      defaults.baseUrl = "https://api.deepseek.com";
      defaults.defaultModel = "deepseek-chat";
      defaults.apiKeyRef = "DEEPSEEK_API_KEY";
    } else if (type === "openai_compatible") {
      defaults.baseUrl = "https://api.openai.com/v1";
      defaults.defaultModel = "gpt-4o";
      defaults.apiKeyRef = "OPENAI_API_KEY";
    } else if (type === "anthropic") {
      defaults.baseUrl = "https://api.anthropic.com";
      defaults.defaultModel = "claude-3-5-sonnet-20241022";
      defaults.apiKeyRef = "ANTHROPIC_API_KEY";
    }
    setForm({
      ...form,
      ...defaults,
    });
  }

  async function handleAddProvider() {
    try {
      setError(null);
      let parsedConfig = {};
      if (configText.trim()) {
        try {
          parsedConfig = JSON.parse(configText);
        } catch (e) {
          showToast("Invalid Custom Options JSON format", "error");
          return;
        }
      }
      const provider = {
        ...form,
        id: form.id || crypto.randomUUID(),
        apiKeyRef: form.apiKeyRef?.trim() || null,
        config: parsedConfig,
      };
      await upsertAiProvider(provider);
      setShowProviderForm(false);
      setForm(EMPTY_PROVIDER);
      setConfigText("{}");
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
  const currentPlaceholders = PLACEHOLDERS[form.adapterType] || PLACEHOLDERS.openai_compatible;

  return (
    <div>
      <h1 style={{ fontSize: 26, fontWeight: 800, marginBottom: 24, letterSpacing: "-0.025em" }}>Settings</h1>

      {error && (
        <div className="badge badge-danger" style={{ display: "block", width: "100%", padding: 12, borderRadius: "var(--radius-sm)", marginBottom: 20, fontSize: 13 }}>
          {error}
        </div>
      )}

      <div className="settings-grid" style={{ display: "grid", gridTemplateColumns: "minmax(0, 1fr) minmax(0, 1fr)", gap: 20 }}>
        {/* AI Providers */}
        <div className="card" style={{ display: "flex", flexDirection: "column" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
            <h2 style={{ fontSize: 16, fontWeight: 700, display: "flex", alignItems: "center", gap: 8 }}>
              <Cpu size={18} color="var(--color-primary)" /> AI Providers
            </h2>
            <div style={{ display: "flex", gap: 8 }}>
              <button
                onClick={handleDetect}
                disabled={detecting}
                className="btn btn-secondary"
                style={{
                  display: "flex", alignItems: "center", gap: 6,
                  padding: "6px 12px",
                }}
              >
                <RefreshCw size={14} className={detecting ? "spin-slow" : ""} />
                {detecting ? "Detecting..." : "Auto-detect"}
              </button>
              <button
                onClick={() => setShowProviderForm(!showProviderForm)}
                className="btn btn-primary"
                style={{
                  display: "flex", alignItems: "center", gap: 6,
                  padding: "6px 12px",
                }}
              >
                <Plus size={14} /> Add
              </button>
            </div>
          </div>

          {showProviderForm && (
            <div className="card" style={{ background: "rgba(255, 255, 255, 0.01)", border: "1px solid var(--border-color)", padding: 16, marginBottom: 16 }}>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 12 }}>
                <div>
                  <label htmlFor="af-provider-name" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>Name</label>
                  <input
                    id="af-provider-name"
                    value={form.name}
                    onChange={(e) => setForm({ ...form, name: e.target.value })}
                    placeholder="e.g., Ollama Local"
                    className="input-field"
                  />
                </div>
                <div>
                  <label htmlFor="af-provider-type" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>Type</label>
                  <select
                    id="af-provider-type"
                    value={form.adapterType}
                    onChange={(e) => handleTypeChange(e.target.value as AiProvider["adapterType"])}
                    className="select-field"
                    style={{ width: "100%" }}
                  >
                    <option value="ollama">Local (Ollama)</option>
                    <option value="deepseek">DeepSeek</option>
                    <option value="openai_compatible">OpenAI</option>
                    <option value="anthropic">Anthropic</option>
                  </select>
                </div>
              </div>
              <div style={{ marginBottom: 12 }}>
                <label htmlFor="af-provider-url" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>Base URL</label>
                <input
                  id="af-provider-url"
                  value={form.baseUrl}
                  onChange={(e) => setForm({ ...form, baseUrl: e.target.value })}
                  placeholder={currentPlaceholders.baseUrl}
                  className="input-field"
                />
              </div>
              <div style={{ marginBottom: 12 }}>
                <label htmlFor="af-provider-model" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>Model</label>
                <input
                  id="af-provider-model"
                  value={form.defaultModel}
                  onChange={(e) => setForm({ ...form, defaultModel: e.target.value })}
                  placeholder={currentPlaceholders.model}
                  className="input-field"
                />
              </div>
              <div style={{ marginBottom: 12 }}>
                <label htmlFor="af-provider-keyref" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>API Key Env Var (optional)</label>
                <input
                  id="af-provider-keyref"
                  value={form.apiKeyRef ?? ""}
                  onChange={(e) => setForm({ ...form, apiKeyRef: e.target.value || null })}
                  placeholder={currentPlaceholders.apiKeyRef}
                  className="input-field"
                />
              </div>
              <div style={{ marginBottom: 16 }}>
                <label htmlFor="af-provider-config" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>Custom Options (JSON, optional)</label>
                <textarea
                  id="af-provider-config"
                  value={configText}
                  onChange={(e) => setConfigText(e.target.value)}
                  placeholder='{ "temperature": 0.2 }'
                  className="input-field"
                  style={{ fontFamily: "var(--font-mono)", fontSize: 12, height: 70, resize: "vertical", width: "100%", background: "rgba(255, 255, 255, 0.03)", border: "1px solid var(--border-color)", color: "var(--text-primary)", padding: 8, borderRadius: "var(--radius-sm)" }}
                />
              </div>
              <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
                <label htmlFor="af-provider-enabled" style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, cursor: "pointer", color: "var(--text-primary)" }}>
                  <input
                    type="checkbox"
                    id="af-provider-enabled"
                    checked={form.enabled}
                    onChange={(e) => setForm({ ...form, enabled: e.target.checked })}
                    style={{ width: 16, height: 16, accentColor: "var(--color-primary)", cursor: "pointer" }}
                  />
                  Enabled
                </label>
              </div>
              <div style={{ display: "flex", gap: 10, justifyContent: "flex-end" }}>
                <button onClick={() => { setShowProviderForm(false); setForm(EMPTY_PROVIDER); setConfigText("{}"); }} className="btn btn-secondary">
                  Cancel
                </button>
                <button onClick={handleAddProvider} className="btn btn-success">
                  Save Provider
                </button>
              </div>
            </div>
          )}

          {providers.length === 0 ? (
            <div style={{ textAlign: "center", padding: 32, color: "var(--text-secondary)" }}>
              <Cpu size={36} style={{ marginBottom: 10, opacity: 0.5, color: "var(--color-primary)" }} />
              <p style={{ fontSize: 13, margin: 0 }}>No AI providers configured. Click "Auto-detect" to find local providers.</p>
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {providers.map((p) => (
                <div key={p.id} style={{ padding: 14, background: "rgba(255, 255, 255, 0.01)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div style={{ minWidth: 0, flex: 1, paddingRight: 10 }}>
                    <div style={{ fontWeight: 700, fontSize: 14, marginBottom: 4, display: "flex", alignItems: "center", gap: 8 }}>
                      <span style={{ color: "var(--text-primary)" }}>{p.name}</span>
                      <span className={p.enabled ? "badge badge-success" : "badge badge-neutral"}>
                        {p.enabled ? "Active" : "Inactive"}
                      </span>
                    </div>
                    <div style={{ fontSize: 12, color: "var(--text-secondary)", fontFamily: "var(--font-mono)", textOverflow: "ellipsis", overflow: "hidden", whiteSpace: "nowrap" }}>
                      {p.adapterType} · {p.defaultModel} · {p.baseUrl}
                    </div>
                    {p.config && Object.keys(p.config).length > 0 && (
                      <div style={{ fontSize: 11, color: "var(--text-secondary)", fontFamily: "var(--font-mono)", marginTop: 4, textOverflow: "ellipsis", overflow: "hidden", whiteSpace: "nowrap" }}>
                        Options: {JSON.stringify(p.config)}
                      </div>
                    )}
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 12, flexShrink: 0 }}>
                    {providerProbes[p.id] && (
                      <span style={{ fontSize: 11, color: providerProbes[p.id].reachable ? "var(--color-success-text)" : "var(--color-danger-text)", fontWeight: 600, fontFamily: "var(--font-mono)" }}>
                        {providerProbes[p.id].reachable
                          ? `${providerProbes[p.id].latencyMs}ms · ${providerProbes[p.id].models.length} models`
                          : "unreachable"}
                      </span>
                    )}
                    <button
                      onClick={() => handleProbeProvider(p.id)}
                      disabled={probingProviderId === p.id}
                      title="Test provider connection"
                      style={{ background: "none", border: "none", cursor: "pointer", padding: 4, color: "var(--color-primary-text)", outline: "none" }}
                    >
                      <Activity size={16} className={probingProviderId === p.id ? "spin-slow" : ""} />
                    </button>
                    <button
                      onClick={() => {
                        setForm(p);
                        setConfigText(JSON.stringify(p.config || {}, null, 2));
                        setShowProviderForm(true);
                      }}
                      title="Edit provider"
                      style={{ background: "none", border: "none", cursor: "pointer", padding: 4, color: "var(--color-primary-text)", opacity: 0.7, transition: "opacity var(--transition-fast)", outline: "none" }}
                      onMouseEnter={(e) => (e.currentTarget.style.opacity = "1")}
                      onMouseLeave={(e) => (e.currentTarget.style.opacity = "0.7")}
                    >
                      <Edit size={16} />
                    </button>
                    <button
                      onClick={() => handleDeleteProvider(p.id)}
                      title="Delete provider"
                      style={{ background: "none", border: "none", cursor: "pointer", padding: 4, color: "var(--color-danger-text)", opacity: 0.7, transition: "opacity var(--transition-fast)", outline: "none" }}
                      onMouseEnter={(e) => (e.currentTarget.style.opacity = "1")}
                      onMouseLeave={(e) => (e.currentTarget.style.opacity = "0.7")}
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
        <div className="card" style={{ display: "flex", flexDirection: "column" }}>
          <h2 style={{ fontSize: 16, fontWeight: 700, marginBottom: 16, display: "flex", alignItems: "center", gap: 8 }}>
            <Github size={18} color="var(--color-accent)" /> GitHub Integration
          </h2>

          {ghAuth === null ? (
            <p style={{ color: "var(--text-secondary)", fontSize: 13 }}>Checking GitHub CLI status...</p>
          ) : ghAuth.authenticated ? (
            <div style={{ padding: 16, background: "var(--color-success-bg)", borderRadius: "var(--radius-md)", border: "1px solid var(--color-success-border)", marginBottom: 20 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                <Wifi size={16} color="var(--color-success-text)" />
                <span style={{ fontWeight: 700, color: "var(--color-success-text)" }}>Authenticated</span>
              </div>
              <div style={{ fontSize: 13, color: "var(--text-primary)" }}>
                Logged in as <strong>{ghAuth.username}</strong>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 8, lineHeight: 1.4 }}>
                Repository metadata can be synced. GitHub write operations remain disabled.
              </div>
            </div>
          ) : (
            <div style={{ padding: 16, background: "var(--color-warning-bg)", borderRadius: "var(--radius-md)", border: "1px solid var(--color-warning-border)", marginBottom: 20 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                <WifiOff size={16} color="var(--color-warning-text)" />
                <span style={{ fontWeight: 700, color: "var(--color-warning-text)" }}>Not Authenticated</span>
              </div>
              <div style={{ fontSize: 13, color: "var(--text-primary)", marginBottom: 8, lineHeight: 1.4 }}>
                Install the GitHub CLI and run <code style={{ background: "rgba(255,255,255,0.03)", padding: "2px 6px", borderRadius: 3, fontSize: 12, fontFamily: "var(--font-mono)" }}>gh auth login</code> to enable GitHub integration.
              </div>
              {ghAuth.message && !ghAuth.authenticated && (
                <div style={{ fontSize: 12, color: "var(--color-warning-text)", fontFamily: "var(--font-mono)" }}>{ghAuth.message}</div>
              )}
            </div>
          )}

          {/* Permission Summary */}
          <div>
            <h3 style={{ fontSize: 14, fontWeight: 700, marginBottom: 12, color: "var(--text-primary)" }}>Permission Policy</h3>
            <div style={{ overflowX: "auto" }}>
              <table className="custom-table" style={{ fontSize: 12 }}>
                <thead>
                  <tr>
                    <th>Capability</th>
                    <th>Risk Level</th>
                    <th>Approval</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="table-row-interactive">
                    <td style={{ padding: "10px 16px" }}>fs.read</td>
                    <td style={{ padding: "10px 16px" }}><span className="badge badge-success" style={{ fontSize: 10 }}>low</span></td>
                    <td style={{ padding: "10px 16px", color: "var(--text-secondary)" }}>Auto-approved</td>
                  </tr>
                  <tr className="table-row-interactive">
                    <td style={{ padding: "10px 16px" }}>github.read</td>
                    <td style={{ padding: "10px 16px" }}><span className="badge badge-success" style={{ fontSize: 10 }}>none</span></td>
                    <td style={{ padding: "10px 16px", color: "var(--text-secondary)" }}>Auto-approved</td>
                  </tr>
                  <tr className="table-row-interactive">
                    <td style={{ padding: "10px 16px" }}>github.create_pr</td>
                    <td style={{ padding: "10px 16px" }}><span className="badge badge-danger" style={{ fontSize: 10 }}>high</span></td>
                    <td style={{ padding: "10px 16px", color: "var(--text-secondary)" }}>Disabled</td>
                  </tr>
                  <tr className="table-row-interactive">
                    <td style={{ padding: "10px 16px" }}>github.create_release</td>
                    <td style={{ padding: "10px 16px" }}><span className="badge badge-danger" style={{ fontSize: 10 }}>critical</span></td>
                    <td style={{ padding: "10px 16px", color: "var(--text-secondary)" }}>Disabled</td>
                  </tr>
                  <tr className="table-row-interactive">
                    <td style={{ padding: "10px 16px" }}>shell.verify</td>
                    <td style={{ padding: "10px 16px" }}><span className="badge badge-warning" style={{ fontSize: 10 }}>medium</span></td>
                    <td style={{ padding: "10px 16px", color: "var(--text-secondary)" }}>Single-use approval</td>
                  </tr>
                  <tr className="table-row-interactive">
                    <td style={{ padding: "10px 16px" }}>fs.write_patch</td>
                    <td style={{ padding: "10px 16px" }}><span className="badge badge-danger" style={{ fontSize: 10 }}>high</span></td>
                    <td style={{ padding: "10px 16px", color: "var(--text-secondary)" }}>Isolated approval</td>
                  </tr>
                </tbody>
              </table>
            </div>
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
