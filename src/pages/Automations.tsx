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
      <h1 style={{ fontSize: 24, fontWeight: 700, marginBottom: 24 }}>Automations</h1>

      {error && (
        <div style={{ padding: 12, background: "#fef2f2", border: "1px solid #fca5a5", borderRadius: 6, marginBottom: 16, color: "#991b1b" }}>
          {error}
        </div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
        {/* Rules */}
        <div style={{ background: "#fff", borderRadius: 8, padding: 20, border: "1px solid #e2e8f0" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
            <h2 style={{ fontSize: 16, fontWeight: 600, display: "flex", alignItems: "center", gap: 8 }}>
              <Zap size={16} /> Rules
            </h2>
            <button
              onClick={() => setShowForm(!showForm)}
              style={{
                display: "flex", alignItems: "center", gap: 4,
                padding: "6px 12px", background: "#3b82f6", color: "#fff",
                border: "none", borderRadius: 6, cursor: "pointer", fontSize: 13, fontWeight: 600,
              }}
            >
              <Plus size={14} /> New Rule
            </button>
          </div>

          {showForm && (
            <div style={{ padding: 16, background: "#f8fafc", borderRadius: 6, marginBottom: 12, border: "1px solid #e2e8f0" }}>
              <div style={{ marginBottom: 8 }}>
                <label htmlFor="af-rule-name" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>Rule Name</label>
                <input
                  id="af-rule-name"
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                  placeholder="e.g., Weekly repository review"
                  style={{ width: "100%", padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 14 }}
                />
              </div>
              <div style={{ marginBottom: 8 }}>
                <label htmlFor="af-rule-desc" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>Description</label>
                <input
                  id="af-rule-desc"
                  value={form.description}
                  onChange={(e) => setForm({ ...form, description: e.target.value })}
                  placeholder="Describe what this rule does"
                  style={{ width: "100%", padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 14 }}
                />
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 8 }}>
                <div>
                  <label htmlFor="af-rule-trigger" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>Trigger</label>
                  <input
                    id="af-rule-trigger"
                    value="Schedule"
                    readOnly
                    style={{ width: "100%", padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 14, background: "#f1f5f9" }}
                  />
                </div>
                <div>
                  <label htmlFor="af-rule-action" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>Action</label>
                  <input
                    id="af-rule-action"
                    value="Notification"
                    readOnly
                    style={{ width: "100%", padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 14, background: "#f1f5f9" }}
                  />
                </div>
              </div>
              <div style={{ marginBottom: 8 }}>
                <label htmlFor="af-rule-interval" style={{ display: "block", fontSize: 12, color: "#64748b", marginBottom: 4 }}>Interval (minutes)</label>
                <input
                  id="af-rule-interval"
                  type="number"
                  min={1}
                  max={10080}
                  value={form.triggerConfig.intervalMinutes ?? 60}
                  onChange={(e) => setForm({ ...form, triggerConfig: { intervalMinutes: Number(e.target.value) } })}
                  style={{ width: "100%", padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 4, fontSize: 14 }}
                />
              </div>
              <div style={{ display: "flex", gap: 8 }}>
                <button onClick={handleCreate} style={{ padding: "8px 16px", background: "#10b981", color: "#fff", border: "none", borderRadius: 6, cursor: "pointer", fontSize: 13, fontWeight: 600 }}>
                  Create Rule
                </button>
                <button onClick={() => setShowForm(false)} style={{ padding: "8px 16px", background: "#f1f5f9", color: "#475569", border: "none", borderRadius: 6, cursor: "pointer", fontSize: 13 }}>
                  Cancel
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
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {rules.map((rule) => (
                <div key={rule.id} style={{ padding: 12, background: "#f8fafc", borderRadius: 6, border: "1px solid #e2e8f0", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div>
                    <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 2, display: "flex", alignItems: "center", gap: 6 }}>
                      {rule.name}
                      <span style={{ fontSize: 11, padding: "1px 6px", borderRadius: 4, fontWeight: 600, background: "#eff6ff", color: "#1d4ed8" }}>
                        every {rule.triggerConfig.intervalMinutes ?? 60} min
                      </span>
                    </div>
                    {rule.description && (
                      <div style={{ fontSize: 12, color: "#64748b", marginBottom: 2 }}>{rule.description}</div>
                    )}
                    <div style={{ fontSize: 12, color: "#64748b" }}>
                      Scheduled notification
                      {rule.runCount > 0 && <span style={{ marginLeft: 8 }}>Runs: {rule.runCount}</span>}
                    </div>
                    {rule.lastTriggeredAt && (
                      <div style={{ fontSize: 11, color: "#94a3b8", marginTop: 2 }}>
                        Last triggered: {new Date(rule.lastTriggeredAt).toLocaleString()}
                      </div>
                    )}
                  </div>
                  <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                    <button
                      onClick={() => handleToggle(rule)}
                      style={{ background: "none", border: "none", cursor: "pointer", color: rule.enabled ? "#10b981" : "#94a3b8" }}
                      title={rule.enabled ? "Disable" : "Enable"}
                    >
                      {rule.enabled ? <ToggleRight size={20} /> : <ToggleLeft size={20} />}
                    </button>
                    <button
                      onClick={() => handleDelete(rule.id)}
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

        {/* Notifications */}
        <div style={{ background: "#fff", borderRadius: 8, padding: 20, border: "1px solid #e2e8f0" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
            <h2 style={{ fontSize: 16, fontWeight: 600, display: "flex", alignItems: "center", gap: 8 }}>
              <Bell size={16} /> Notifications
              {unreadCount > 0 && (
                <span style={{ background: "#ef4444", color: "#fff", borderRadius: 10, padding: "1px 6px", fontSize: 11, fontWeight: 600 }}>
                  {unreadCount}
                </span>
              )}
            </h2>
            {unreadCount > 0 && (
              <button
                onClick={handleMarkAllRead}
                style={{ fontSize: 12, color: "#3b82f6", background: "none", border: "none", cursor: "pointer" }}
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
            <div style={{ display: "flex", flexDirection: "column", gap: 6, maxHeight: 500, overflow: "auto" }}>
              {notifications.map((n) => (
                <div
                  key={n.id}
                  style={{
                    padding: 10, borderRadius: 6, border: "1px solid #e2e8f0",
                    background: n.read ? "#fff" : "#f0f9ff",
                    display: "flex", justifyContent: "space-between", alignItems: "flex-start",
                  }}
                >
                  <div style={{ flex: 1 }}>
                    <div style={{ fontWeight: n.read ? 400 : 600, fontSize: 13, marginBottom: 2 }}>{n.title}</div>
                    <div style={{ fontSize: 12, color: "#64748b" }}>{n.message}</div>
                    <div style={{ fontSize: 11, color: "#94a3b8", marginTop: 2 }}>{new Date(n.createdAt).toLocaleString()}</div>
                  </div>
                  {!n.read && (
                    <button
                      onClick={() => handleMarkRead(n.id)}
                      style={{ background: "none", border: "none", cursor: "pointer", color: "#3b82f6" }}
                      title="Mark as read"
                    >
                      <Check size={14} />
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
