import { useEffect, useRef } from "react";
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
  const dialogRef = useRef<HTMLDivElement>(null);
  const onDenyRef = useRef(onDeny);
  const busyRef = useRef(busy);
  useEffect(() => { onDenyRef.current = onDeny; }, [onDeny]);
  useEffect(() => { busyRef.current = busy; }, [busy]);
  useEffect(() => {
    const previousFocus = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    dialogRef.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !busyRef.current) {
        event.preventDefault();
        onDenyRef.current();
        return;
      }
      if (event.key !== "Tab" || !dialogRef.current) return;
      const focusable = Array.from(dialogRef.current.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ));
      if (focusable.length === 0) {
        event.preventDefault();
        dialogRef.current.focus();
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      previousFocus?.focus();
    };
  }, []);

  const highestRisk = requests.some((request) => request.riskLevel === "high")
    ? "high"
    : "medium";

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="approval-title"
      ref={dialogRef}
      tabIndex={-1}
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 10000,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: 24,
        background: "rgba(3, 7, 18, 0.6)",
        backdropFilter: "blur(8px)",
        WebkitBackdropFilter: "blur(8px)",
      }}
      onClick={busy ? undefined : onDeny}
    >
      <div
        className="card fade-in"
        style={{
          width: "min(680px, 94vw)",
          maxHeight: "84vh",
          overflow: "auto",
          padding: 0,
          boxShadow: "var(--shadow-lg), 0 0 50px rgba(0, 0, 0, 0.5)",
          display: "flex",
          flexDirection: "column",
        }}
        onClick={(event) => event.stopPropagation()}
      >
        <div style={{ padding: "18px 24px", borderBottom: "1px solid var(--border-color)" }}>
          <div style={{ display: "flex", alignItems: "flex-start", gap: 12 }}>
            <ShieldCheck size={24} color={highestRisk === "high" ? "var(--color-danger)" : "var(--color-warning)"} style={{ flexShrink: 0, marginTop: 2 }} />
            <div style={{ flex: 1 }}>
              <h2 id="approval-title" style={{ margin: 0, fontSize: 16, color: "var(--text-primary)", fontWeight: 700 }}>
                Review execution approval
              </h2>
              <p style={{ margin: "4px 0 0", fontSize: 12, color: "var(--text-secondary)" }}>
                Approval is single-use and becomes invalid if the repository context changes.
              </p>
            </div>
            <button
              type="button"
              aria-label="Close approval dialog"
              title="Close"
              disabled={busy}
              onClick={onDeny}
              style={{
                border: 0,
                background: "transparent",
                color: "var(--text-secondary)",
                cursor: "pointer",
                padding: 4,
                opacity: 0.6,
                transition: "opacity var(--transition-fast)",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.opacity = "1")}
              onMouseLeave={(e) => (e.currentTarget.style.opacity = "0.6")}
            >
              <X size={18} />
            </button>
          </div>
        </div>

        <div className="scrollbar-custom" style={{ padding: 24, display: "flex", flexDirection: "column", gap: 16, overflowY: "auto", flex: 1 }}>
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
                  padding: 16,
                  border: "1px solid var(--border-color)",
                  borderRadius: "var(--radius-md)",
                  background: "rgba(255, 255, 255, 0.02)",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
                  <AlertTriangle size={14} color={request.riskLevel === "high" ? "var(--color-danger-text)" : "var(--color-warning-text)"} />
                  <strong style={{ fontSize: 13, color: "var(--text-primary)", fontWeight: 600 }}>{request.capability}</strong>
                  <span
                    className={request.riskLevel === "high" ? "badge badge-danger" : "badge badge-warning"}
                  >
                    {request.riskLevel}
                  </span>
                </div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>
                  Scope: <code style={{ background: "rgba(255,255,255,0.05)", padding: "2px 6px", borderRadius: 4, fontFamily: "var(--font-mono)", fontSize: 11 }}>{request.scope}</code>
                </div>
                {filePath && (
                  <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>
                    Target: <code style={{ background: "rgba(255,255,255,0.05)", padding: "2px 6px", borderRadius: 4, fontFamily: "var(--font-mono)", fontSize: 11 }}>{filePath}</code>
                  </div>
                )}
                {command && (
                  <pre style={{ margin: "0 0 10px", padding: 12, borderRadius: "var(--radius-sm)", background: "var(--bg-input)", border: "1px solid var(--border-color)", color: "#93c5fd", fontSize: 12, fontFamily: "var(--font-mono)", whiteSpace: "pre-wrap", overflowX: "auto" }}>
                    {command}
                  </pre>
                )}
                {expanded && expanded !== command && (
                  <details style={{ marginBottom: 10 }}>
                    <summary style={{ cursor: "pointer", fontSize: 12, color: "var(--color-primary)", fontWeight: 500, outline: "none" }}>
                      Expanded lifecycle scripts
                    </summary>
                    <pre style={{ margin: "8px 0 0", padding: 12, borderRadius: "var(--radius-sm)", background: "rgba(255, 255, 255, 0.01)", border: "1px solid var(--border-color)", color: "var(--text-secondary)", fontSize: 11, fontFamily: "var(--font-mono)", whiteSpace: "pre-wrap" }}>
                      {expanded}
                    </pre>
                  </details>
                )}
                {reason && <p style={{ margin: 0, fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.4 }}>{reason}</p>}
                {clean === false && (
                  <p style={{ margin: "8px 0 0", fontSize: 12, color: "var(--color-danger-text)", fontWeight: 500, display: "flex", alignItems: "center", gap: 6 }}>
                    <span style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--color-danger)" }} />
                    The working tree is not clean. This operation cannot proceed.
                  </p>
                )}
                {checks.length > 0 && (
                  <div style={{ marginTop: 12 }}>
                    <p style={{ margin: "0 0 8px", fontSize: 12, color: "var(--color-info-text)", display: "flex", alignItems: "center", gap: 6 }}>
                      <span style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--color-info)" }} />
                      Approving this patch also runs these commands in the isolated worktree:
                    </p>
                    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                      {checks.map((check, index) => {
                        const details = check && typeof check === "object"
                          ? check as Record<string, unknown>
                          : {};
                        const verificationCommand = detailString(details.command) ?? "Unknown command";
                        const verificationExpanded = detailString(details.expandedCommand);
                        const verificationRisk = detailString(details.risk);
                        return (
                          <div
                            key={`${verificationCommand}-${index}`}
                            style={{
                              padding: 10,
                              borderRadius: "var(--radius-sm)",
                              background: "var(--bg-input)",
                              border: "1px solid var(--border-color)",
                            }}
                          >
                            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                              <code style={{ color: "#93c5fd", fontSize: 11, fontFamily: "var(--font-mono)" }}>
                                {verificationCommand}
                              </code>
                              {verificationRisk && <span className="badge badge-warning">{verificationRisk}</span>}
                            </div>
                            {verificationExpanded && verificationExpanded !== verificationCommand && (
                              <pre style={{ margin: "8px 0 0", color: "var(--text-secondary)", fontSize: 11, fontFamily: "var(--font-mono)", whiteSpace: "pre-wrap" }}>
                                {verificationExpanded}
                              </pre>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}
              </section>
            );
          })}
        </div>

        <div style={{ display: "flex", justifyContent: "flex-end", gap: 12, padding: "16px 24px", borderTop: "1px solid var(--border-color)" }}>
          <button
            type="button"
            disabled={busy}
            onClick={onDeny}
            className="btn btn-secondary"
          >
            Deny
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={onApprove}
            className="btn btn-success"
            style={{
              background: "var(--color-success)",
              color: "#fff",
              border: "none",
              boxShadow: "0 2px 8px rgba(16, 185, 129, 0.3)",
            }}
          >
            <Check size={14} />
            {busy ? "Running..." : "Approve once"}
          </button>
        </div>
      </div>
    </div>
  );
}
