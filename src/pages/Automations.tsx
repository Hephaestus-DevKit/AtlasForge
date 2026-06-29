import { useEffect, useState } from "react";
import {
  listAutomationRules,
  createAutomationRule,
  updateAutomationRule,
  deleteAutomationRule,
  listNotifications,
  markNotificationRead,
  markAllNotificationsRead,
} from "../api/ipc";
import type { AutomationRule, Notification } from "../types";
import { Zap, Plus, Trash2, Bell, Check, ToggleLeft, ToggleRight } from "lucide-react";
import { ConfirmModal } from "../components/ConfirmModal";
import { EmptyState } from "../components/EmptyState";

const EMPTY_RULE: AutomationRule = {
  id: "",
  name: "",
  description: "",
  triggerType: "schedule",
  triggerConfig: { intervalMinutes: 60 },
  actionType: "notify",
  actionConfig: {},
  targetRepoIds: [],
  targetRootIds: [],
  maxRiskLevel: "low",
  autoApply: false,
  enabled: true,
  lastTriggeredAt: null,
  lastRunJobId: null,
  runCount: 0,
};

export function Automations() {
  const [rules, setRules] = useState<AutomationRule[]>([]);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState<AutomationRule>(EMPTY_RULE);
  const [confirmAction, setConfirmAction] = useState<{ message: string; onConfirm: () => void } | null>(null);

  useEffect(() => {
    loadData();
  }, []);

  async function loadData() {
    try {
      setError(null);
      const [r, n] = await Promise.all([
        listAutomationRules(),
        listNotifications(false, 50),
      ]);
      setRules(r);
      setNotifications(n);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load data");
    }
  }

  async function handleCreate() {
    try {
      setError(null);
      const rule = { ...form, id: crypto.randomUUID() };
      await createAutomationRule(rule);
      setShowForm(false);
      setForm(EMPTY_RULE);
      await loadData();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to create rule");
    }
  }

  async function handleToggle(rule: AutomationRule) {
    try {
      setError(null);
      await updateAutomationRule({ ...rule, enabled: !rule.enabled });
      await loadData();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to toggle rule");
    }
  }

  async function handleDelete(id: string) {
    setConfirmAction({
      message: "Delete this automation rule?",
      onConfirm: async () => {
        setConfirmAction(null);
        try {
          setError(null);
          await deleteAutomationRule(id);
          await loadData();
        } catch (e: any) {
          setError(e?.toString() ?? "Failed to delete rule");
        }
      },
    });
  }

  async function handleMarkRead(id: string) {
    try {
      await markNotificationRead(id);
      await loadData();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to mark as read");
    }
  }

  async function handleMarkAllRead() {
    try {
      await markAllNotificationsRead();
      await loadData();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to mark all as read");
    }
  }

  const unreadCount = notifications.filter((n) => !n.read).length;

  return (
    <div>
      <h1 style={{ fontSize: 26, fontWeight: 800, marginBottom: 24, letterSpacing: "-0.025em" }}>Automations</h1>

      {error && (
        <div className="badge badge-danger" style={{ display: "block", width: "100%", padding: 12, borderRadius: "var(--radius-sm)", marginBottom: 20, fontSize: 13 }}>
          {error}
        </div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 20 }}>
        {/* Rules */}
        <div className="card" style={{ display: "flex", flexDirection: "column" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
            <h2 style={{ fontSize: 16, fontWeight: 700, display: "flex", alignItems: "center", gap: 8 }}>
              <Zap size={18} color="var(--color-primary)" /> Rules
            </h2>
            <button
              onClick={() => setShowForm(!showForm)}
              className="btn btn-primary"
            >
              <Plus size={16} /> New Rule
            </button>
          </div>

          {showForm && (
            <div className="card" style={{ background: "rgba(255, 255, 255, 0.01)", border: "1px solid var(--border-color)", padding: 16, marginBottom: 16 }}>
              <div style={{ marginBottom: 12 }}>
                <label htmlFor="af-rule-name" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>Rule Name</label>
                <input
                  id="af-rule-name"
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                  placeholder="e.g., Weekly repository review"
                  className="input-field"
                />
              </div>
              <div style={{ marginBottom: 12 }}>
                <label htmlFor="af-rule-desc" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>Description</label>
                <input
                  id="af-rule-desc"
                  value={form.description}
                  onChange={(e) => setForm({ ...form, description: e.target.value })}
                  placeholder="Describe what this rule does"
                  className="input-field"
                />
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 12 }}>
                <div>
                  <label htmlFor="af-rule-trigger" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>Trigger</label>
                  <input
                    id="af-rule-trigger"
                    value="Schedule"
                    readOnly
                    className="input-field"
                    style={{ opacity: 0.7, background: "rgba(255,255,255,0.02)" }}
                  />
                </div>
                <div>
                  <label htmlFor="af-rule-action" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>Action</label>
                  <input
                    id="af-rule-action"
                    value="Notification"
                    readOnly
                    className="input-field"
                    style={{ opacity: 0.7, background: "rgba(255,255,255,0.02)" }}
                  />
                </div>
              </div>
              <div style={{ marginBottom: 16 }}>
                <label htmlFor="af-rule-interval" style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>Interval (minutes)</label>
                <input
                  id="af-rule-interval"
                  type="number"
                  min={1}
                  max={10080}
                  value={form.triggerConfig.intervalMinutes ?? 60}
                  onChange={(e) => setForm({ ...form, triggerConfig: { intervalMinutes: Number(e.target.value) } })}
                  className="input-field"
                />
              </div>
              <div style={{ display: "flex", gap: 10, justifyContent: "flex-end" }}>
                <button onClick={() => setShowForm(false)} className="btn btn-secondary">
                  Cancel
                </button>
                <button onClick={handleCreate} className="btn btn-success">
                  Create Rule
                </button>
              </div>
            </div>
          )}

          {rules.length === 0 ? (
            <EmptyState
              icon={Zap}
              title="No automation rules yet"
              description="Create a scheduled notification rule."
            />
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {rules.map((rule) => (
                <div key={rule.id} style={{ padding: 14, background: "rgba(255, 255, 255, 0.01)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div style={{ minWidth: 0, flex: 1, paddingRight: 10 }}>
                    <div style={{ fontWeight: 700, fontSize: 14, marginBottom: 4, display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                      <span style={{ color: "var(--text-primary)" }}>{rule.name}</span>
                      <span className="badge badge-info" style={{ fontSize: 10 }}>
                        every {rule.triggerConfig.intervalMinutes ?? 60} min
                      </span>
                    </div>
                    {rule.description && (
                      <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4, textOverflow: "ellipsis", overflow: "hidden", whiteSpace: "nowrap" }}>{rule.description}</div>
                    )}
                    <div style={{ fontSize: 12, color: "var(--text-muted)", display: "flex", gap: 12 }}>
                      <span>Scheduled notification</span>
                      {rule.runCount > 0 && <span>Runs: {rule.runCount}</span>}
                    </div>
                    {rule.lastTriggeredAt && (
                      <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 4 }}>
                        Last triggered: {new Date(rule.lastTriggeredAt).toLocaleString()}
                      </div>
                    )}
                  </div>
                  <div style={{ display: "flex", gap: 10, alignItems: "center", flexShrink: 0 }}>
                    <button
                      onClick={() => handleToggle(rule)}
                      style={{ background: "none", border: "none", cursor: "pointer", padding: 4, color: rule.enabled ? "var(--color-success)" : "var(--text-muted)", outline: "none" }}
                      title={rule.enabled ? "Disable" : "Enable"}
                    >
                      {rule.enabled ? <ToggleRight size={24} /> : <ToggleLeft size={24} />}
                    </button>
                    <button
                      onClick={() => handleDelete(rule.id)}
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

        {/* Notifications */}
        <div className="card" style={{ display: "flex", flexDirection: "column" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
            <h2 style={{ fontSize: 16, fontWeight: 700, display: "flex", alignItems: "center", gap: 8 }}>
              <Bell size={18} color="var(--color-accent)" /> Notifications
              {unreadCount > 0 && (
                <span className="badge badge-danger" style={{ borderRadius: 12, padding: "2px 8px", fontSize: 10, filter: "drop-shadow(0 0 5px rgba(239, 68, 68, 0.4))" }}>
                  {unreadCount} new
                </span>
              )}
            </h2>
            {unreadCount > 0 && (
              <button
                onClick={handleMarkAllRead}
                style={{ fontSize: 12, color: "var(--color-primary-text)", background: "none", border: "none", cursor: "pointer", fontWeight: 600, outline: "none" }}
              >
                Mark all read
              </button>
            )}
          </div>

          {notifications.length === 0 ? (
            <EmptyState
              icon={Bell}
              title="No notifications"
              description="Notifications will appear here when automation rules trigger."
            />
          ) : (
            <div className="scrollbar-custom" style={{ display: "flex", flexDirection: "column", gap: 10, maxHeight: 500, overflowY: "auto" }}>
              {notifications.map((n) => (
                <div
                  key={n.id}
                  style={{
                    padding: 12,
                    borderRadius: "var(--radius-sm)",
                    border: "1px solid var(--border-color)",
                    background: n.read ? "rgba(255, 255, 255, 0.01)" : "rgba(99, 102, 241, 0.04)",
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "flex-start",
                    gap: 10,
                  }}
                >
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ fontWeight: n.read ? 600 : 700, fontSize: 13, marginBottom: 2, color: "var(--text-primary)" }}>{n.title}</div>
                    <div style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.4 }}>{n.message}</div>
                    <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 6, fontFamily: "var(--font-mono)" }}>{new Date(n.createdAt).toLocaleString()}</div>
                  </div>
                  {!n.read && (
                    <button
                      onClick={() => handleMarkRead(n.id)}
                      style={{ background: "none", border: "none", cursor: "pointer", padding: 4, color: "var(--color-primary)", outline: "none" }}
                      title="Mark as read"
                    >
                      <Check size={16} />
                    </button>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
      {confirmAction && (
        <ConfirmModal
          message={confirmAction.message}
          onConfirm={confirmAction.onConfirm}
          onCancel={() => setConfirmAction(null)}
        />
      )}
    </div>
  );
}
