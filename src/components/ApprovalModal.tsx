import { AlertTriangle, Check, ShieldCheck, X } from "lucide-react";
import type { PermissionRequest } from "../types";

interface ApprovalModalProps {
  requests: PermissionRequest[];
  busy: boolean;
  onApprove: () => void;
  onDeny: () => void;
}

function detailString(value: unknown): string | null {
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return null;
}

export function ApprovalModal({
  requests,
  busy,
  onApprove,
  onDeny,
}: ApprovalModalProps) {
  const highestRisk = requests.some((request) => request.riskLevel === "high")
    ? "high"
    : "medium";

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="approval-title"
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 1100,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: 24,
        background: "rgba(15, 23, 42, 0.48)",
      }}
      onClick={busy ? undefined : onDeny}
    >
      <div
        style={{
          width: "min(680px, 94vw)",
          maxHeight: "84vh",
          overflow: "auto",
          borderRadius: 8,
          border: "1px solid #cbd5e1",
          background: "#fff",
          boxShadow: "0 18px 50px rgba(15, 23, 42, 0.22)",
        }}
        onClick={(event) => event.stopPropagation()}
      >
        <div style={{ padding: "18px 20px", borderBottom: "1px solid #e2e8f0" }}>
          <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
            <ShieldCheck size={20} color={highestRisk === "high" ? "#b91c1c" : "#b45309"} />
            <div style={{ flex: 1 }}>
              <h2 id="approval-title" style={{ margin: 0, fontSize: 16, color: "#0f172a" }}>
                Review execution approval
              </h2>
              <p style={{ margin: "4px 0 0", fontSize: 12, color: "#64748b" }}>
                Approval is single-use and becomes invalid if the repository context changes.
              </p>
            </div>
            <button
              type="button"
              aria-label="Close approval dialog"
              title="Close"
              disabled={busy}
              onClick={onDeny}
              style={{ border: 0, background: "transparent", color: "#64748b", cursor: "pointer" }}
            >
              <X size={18} />
            </button>
          </div>
        </div>

        <div style={{ padding: 20, display: "flex", flexDirection: "column", gap: 12 }}>
          {requests.map((request) => {
            const command = detailString(request.details.command) ?? request.command;
            const expanded = detailString(request.details.expandedCommand);
            const reason = detailString(request.details.reason);
            const filePath = detailString(request.details.filePath);
            const clean = request.details.workingTreeClean;
            const checks = Array.isArray(request.details.isolatedVerificationCommands)
              ? request.details.isolatedVerificationCommands
              : [];
            return (
              <section
                key={request.id}
                style={{
                  padding: 12,
                  border: "1px solid #e2e8f0",
                  borderRadius: 6,
                  background: "#f8fafc",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                  <AlertTriangle size={14} color={request.riskLevel === "high" ? "#b91c1c" : "#b45309"} />
                  <strong style={{ fontSize: 12, color: "#334155" }}>{request.capability}</strong>
                  <span
                    style={{
                      padding: "1px 6px",
                      borderRadius: 3,
                      fontSize: 10,
                      fontWeight: 700,
                      color: request.riskLevel === "high" ? "#991b1b" : "#92400e",
                      background: request.riskLevel === "high" ? "#fee2e2" : "#fef3c7",
                    }}
                  >
                    {request.riskLevel}
                  </span>
                </div>
                <div style={{ fontSize: 11, color: "#64748b", marginBottom: 6 }}>
                  Scope: <code>{request.scope}</code>
                </div>
                {filePath && (
                  <div style={{ fontSize: 12, color: "#334155", marginBottom: 6 }}>
                    Target: <code>{filePath}</code>
                  </div>
                )}
                {command && (
                  <pre style={{ margin: "0 0 6px", padding: 8, borderRadius: 4, background: "#0f172a", color: "#e2e8f0", fontSize: 11, whiteSpace: "pre-wrap" }}>
                    {command}
                  </pre>
                )}
                {expanded && expanded !== command && (
                  <details style={{ marginBottom: 6 }}>
                    <summary style={{ cursor: "pointer", fontSize: 11, color: "#475569" }}>
                      Expanded lifecycle scripts
                    </summary>
                    <pre style={{ margin: "6px 0 0", padding: 8, borderRadius: 4, background: "#fff", border: "1px solid #e2e8f0", color: "#334155", fontSize: 11, whiteSpace: "pre-wrap" }}>
                      {expanded}
                    </pre>
                  </details>
                )}
                {reason && <p style={{ margin: 0, fontSize: 11, color: "#64748b" }}>{reason}</p>}
                {clean === false && (
                  <p style={{ margin: "6px 0 0", fontSize: 11, color: "#991b1b" }}>
                    The working tree is not clean. This operation cannot proceed.
                  </p>
                )}
                {checks.length > 0 && (
                  <p style={{ margin: "6px 0 0", fontSize: 11, color: "#64748b" }}>
                    The patch will be tested in an isolated worktree using {checks.length} detected verification command(s).
                  </p>
                )}
              </section>
            );
          })}
        </div>

        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, padding: "14px 20px", borderTop: "1px solid #e2e8f0" }}>
          <button
            type="button"
            disabled={busy}
            onClick={onDeny}
            style={{ padding: "7px 12px", borderRadius: 5, border: "1px solid #cbd5e1", background: "#fff", color: "#475569", cursor: busy ? "wait" : "pointer" }}
          >
            Deny
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={onApprove}
            style={{ display: "flex", alignItems: "center", gap: 5, padding: "7px 12px", borderRadius: 5, border: 0, background: "#0f766e", color: "#fff", cursor: busy ? "wait" : "pointer", fontWeight: 600 }}
          >
            <Check size={14} />
            {busy ? "Running..." : "Approve once"}
          </button>
        </div>
      </div>
    </div>
  );
}
