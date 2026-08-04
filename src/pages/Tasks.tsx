import { useEffect, useRef, useState } from "react";
import { listJobs, getJobEvents, cancelJob, retryJob, listArtifacts, listAuditLog } from "../api/ipc";
import type { Job, JobEvent, Artifact, AuditEntry } from "../types";
import { ListTodo, ChevronDown, ChevronRight, RotateCcw, Package, Clock, AlertTriangle, CheckCircle2, Loader2, Ban, ShieldCheck } from "lucide-react";
import { EmptyState } from "../components/EmptyState";

/** Parse a JSON payload string safely. */
function parsePayload(raw: string): Record<string, unknown> | null {
  try {
    return JSON.parse(raw);
  } catch (_e) {
    return null;
  }
}

/** Human-readable labels for job event types. */
const eventTypeLabels: Record<string, string> = {
  job_created: "Created",
  job_started: "Started",
  job_cancelled: "Cancelled",
  job_failed: "Failed",
  job_completed: "Completed",
  scan_started: "Scan started",
  scan_root_started: "Root scan started",
  scan_repo_discovered: "Repo discovered",
  scan_repo_profiled: "Repo profiled",
  scan_repo_profile_failed: "Repo profile failed",
  scan_repo_completed: "Repo scan completed",
  scan_root_completed: "Root scan completed",
  scan_root_scanned: "Root scanned",
  scan_completed: "Scan completed",
  root_scanned: "Root scanned",
  root_scan_error: "Scan error",
  root_skipped: "Root skipped",
  scan_summary: "Scan summary",
  audit_started: "Audit started",
  audit_completed: "Audit completed",
  audit_failed: "Audit failed",
  reindex_started: "Reindex started",
  reindex_completed: "Reindex completed",
  reindex_failed: "Reindex failed",
  verification_started: "Verification started",
  verification_completed: "Verification passed",
  verification_failed: "Verification failed",
  github_sync_started: "GitHub sync started",
  github_sync_completed: "GitHub sync completed",
  github_sync_failed: "GitHub sync failed",
  ai_call_started: "AI call started",
  ai_call_completed: "AI call completed",
  ai_call_failed: "AI call failed",
  patch_apply_started: "Patch apply started",
  patch_apply_completed: "Patch applied",
  patch_apply_failed: "Patch apply failed",
};

/** Icon for event type. */
function EventIcon({ type }: { type: string }) {
  if (type.endsWith("_failed") || type === "root_scan_error") {
    return <AlertTriangle size={12} color="var(--color-danger)" />;
  }
  if (type.endsWith("_completed") || type === "job_completed") {
    return <CheckCircle2 size={12} color="var(--color-success)" />;
  }
  if (type === "job_cancelled" || type === "root_skipped") {
    return <Ban size={12} color="var(--text-muted)" />;
  }
  if (type.includes("started")) {
    return <Loader2 size={12} color="var(--color-primary)" className="spin-slow" />;
  }
  return <Clock size={12} color="var(--text-secondary)" />;
}

/** Render a concise summary from a parsed event payload. */
function renderPayloadSummary(type: string, payload: Record<string, unknown>): string | null {
  if ("error" in payload && typeof payload.error === "string") {
    return payload.error.length > 120 ? payload.error.slice(0, 120) + "…" : payload.error;
  }
  if (type === "scan_summary") {
    const parts: string[] = [];
    if ("reposDiscovered" in payload) parts.push(`${payload.reposDiscovered} repos`);
    if ("rootsScanned" in payload) parts.push(`${payload.rootsScanned} roots scanned`);
    if ("rootsSkipped" in payload) parts.push(`${payload.rootsSkipped} skipped`);
    return parts.join(", ") || null;
  }
  if (type === "root_skipped") {
    const label = typeof payload.label === "string" ? payload.label : "";
    const reason = typeof payload.reason === "string" ? payload.reason : "";
    return [label, reason].filter(Boolean).join(" — ") || null;
  }
  if (type === "audit_completed") {
    const score = payload.score;
    const repoId = typeof payload.repoId === "string" ? payload.repoId : "";
    return `Score: ${score}${repoId ? ` (${repoId.slice(0, 8)}…)` : ""}`;
  }
  if (type === "reindex_completed") {
    const docs = payload.documents;
    const chunks = payload.chunks;
    return `${docs} documents, ${chunks} chunks`;
  }
  if (type === "verification_completed" || type === "verification_failed") {
    const cmd = typeof payload.command === "string" ? payload.command : "";
    const dur = payload.durationMs;
    const exitCode = payload.exitCode;
    const parts: string[] = [];
    if (cmd) parts.push(cmd);
    if (typeof dur === "number") parts.push(`${dur}ms`);
    if (typeof exitCode === "number") parts.push(`exit ${exitCode}`);
    return parts.join(" · ") || null;
  }
  if (type === "github_sync_completed") {
    const parts: string[] = [];
    if (typeof payload.workflows === "number") parts.push(`${payload.workflows} workflows`);
    if (typeof payload.prs === "number") parts.push(`${payload.prs} PRs`);
    if (typeof payload.releases === "number") parts.push(`${payload.releases} releases`);
    return parts.join(", ") || null;
  }
  if (type === "ai_call_completed") {
    const parts: string[] = [];
    if (typeof payload.tokensIn === "number") parts.push(`${payload.tokensIn} in`);
    if (typeof payload.tokensOut === "number") parts.push(`${payload.tokensOut} out`);
    return parts.length > 0 ? `${parts.join(", ")} tokens` : null;
  }
  if (type === "scan_repo_discovered") {
    const path = typeof payload.path === "string" ? payload.path : "";
    return path.length > 80 ? path.slice(0, 80) + "…" : path;
  }
  const parts: string[] = [];
  if ("repoId" in payload && typeof payload.repoId === "string") {
    parts.push(`repo ${payload.repoId.slice(0, 8)}…`);
  }
  if ("rootId" in payload && typeof payload.rootId === "string") {
    parts.push(`root ${payload.rootId.slice(0, 8)}…`);
  }
  if ("rootCount" in payload) {
    parts.push(`${payload.rootCount} roots`);
  }
  return parts.length > 0 ? parts.join(", ") : null;
}

const statusIcons: Record<string, React.ReactNode> = {
  pending: <Clock size={14} color="var(--color-warning-text)" />,
  running: <Loader2 size={14} color="var(--color-info-text)" className="spin-slow" />,
  completed: <CheckCircle2 size={14} color="var(--color-success-text)" />,
  failed: <AlertTriangle size={14} color="var(--color-danger-text)" />,
  cancelled: <Ban size={14} color="var(--text-secondary)" />,
};

export function Tasks() {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [expandedJob, setExpandedJob] = useState<string | null>(null);
  const [events, setEvents] = useState<JobEvent[]>([]);
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [auditEntries, setAuditEntries] = useState<AuditEntry[]>([]);
  const loadInFlight = useRef(false);

  useEffect(() => {
    let stopped = false;
    let timer: number | undefined;
    const poll = async () => {
      await loadJobs();
      if (!stopped) {
        timer = window.setTimeout(poll, document.hidden ? 15_000 : 3_000);
      }
    };
    void poll();
    return () => {
      stopped = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, []);

  async function loadJobs() {
    if (loadInFlight.current) return;
    loadInFlight.current = true;
    try {
      setError(null);
      const [jobList, auditLog] = await Promise.all([
        listJobs(100),
        listAuditLog(30),
      ]);
      setJobs(jobList);
      setAuditEntries(auditLog);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load jobs");
    } finally {
      loadInFlight.current = false;
    }
  }

  async function toggleExpand(jobId: string) {
    if (expandedJob === jobId) {
      setExpandedJob(null);
      setEvents([]);
      setArtifacts([]);
      return;
    }
    try {
      setExpandedJob(jobId);
      const [ev, art] = await Promise.all([
        getJobEvents(jobId),
        listArtifacts(jobId),
      ]);
      setEvents(ev);
      setArtifacts(art);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load events");
    }
  }

  async function handleCancel(jobId: string) {
    try {
      setError(null);
      await cancelJob(jobId);
      await loadJobs();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to cancel job");
    }
  }

  async function handleRetry(jobId: string) {
    try {
      setError(null);
      await retryJob(jobId);
      await loadJobs();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to retry job");
    }
  }

  function formatTime(iso: string | null | undefined): string {
    if (!iso) return "";
    try {
      return new Date(iso).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit", second: "2-digit" });
    } catch (_err) {
      return iso;
    }
  }

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 28 }}>
        <h1 style={{ fontSize: 26, fontWeight: 800, letterSpacing: "-0.025em" }}>Tasks</h1>
        <button
          onClick={loadJobs}
          className="btn btn-secondary"
        >
          Refresh
        </button>
      </div>

      {error && (
        <div className="badge badge-danger" style={{ display: "block", width: "100%", padding: 12, borderRadius: "var(--radius-sm)", marginBottom: 20, fontSize: 13 }}>
          {error}
        </div>
      )}

      {jobs.length === 0 ? (
        <EmptyState
          icon={ListTodo}
          title="No jobs yet"
          description="Start a scan from the Dashboard to create one."
        />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 12, marginBottom: 32 }}>
          {jobs.map((job) => {
            const isExpanded = expandedJob === job.id;
            const badgeClasses: Record<string, string> = {
              pending: "badge badge-warning",
              running: "badge badge-info pulse-glow",
              completed: "badge badge-success",
              failed: "badge badge-danger",
              cancelled: "badge badge-neutral",
            };
            const cls = badgeClasses[job.status] ?? "badge badge-neutral";
            return (
              <div key={job.id} className="card" style={{ padding: 0, overflow: "hidden" }}>
                {/* Job Header */}
                <div
                  style={{ padding: "16px 20px", display: "flex", alignItems: "center", gap: 14, cursor: "pointer" }}
                  onClick={() => toggleExpand(job.id)}
                >
                  {isExpanded ? <ChevronDown size={18} color="var(--text-secondary)" /> : <ChevronRight size={18} color="var(--text-secondary)" />}
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ display: "flex", alignItems: "center", flexWrap: "wrap", gap: 10 }}>
                      {statusIcons[job.status] ?? null}
                      <span style={{ fontWeight: 700, fontSize: 15, color: "var(--text-primary)" }}>{job.type}</span>
                      <span className={cls}>
                        {job.status}
                      </span>
                      {job.progressTotal > 0 && (
                        <span style={{ fontSize: 12, color: "var(--text-secondary)", fontWeight: 600 }}>
                          {job.progress} / {job.progressTotal}
                        </span>
                      )}
                    </div>
                    <div style={{ fontSize: 12, color: "var(--text-muted)", marginTop: 4 }}>
                      {new Date(job.createdAt).toLocaleString()}
                      {job.completedAt && <> · Completed: {new Date(job.completedAt).toLocaleString()}</>}
                      {job.parentJobId && (
                        <span style={{ marginLeft: 8, color: "var(--text-muted)", fontFamily: "var(--font-mono)" }}>
                          (retry of {job.parentJobId.slice(0, 8)}…)
                        </span>
                      )}
                    </div>
                    {job.errorMessage && (
                      <div className="badge badge-danger" style={{ display: "block", marginTop: 8, padding: "8px 12px", borderRadius: "var(--radius-sm)", fontSize: 12, textAlign: "left" }}>
                        {job.errorMessage.length > 200 ? job.errorMessage.slice(0, 200) + "…" : job.errorMessage}
                      </div>
                    )}
                  </div>
                  <div style={{ display: "flex", gap: 8, flexShrink: 0 }} onClick={(e) => e.stopPropagation()}>
                    {(job.status === "running" || job.status === "pending") && (
                      <button
                        onClick={() => handleCancel(job.id)}
                        className="btn btn-danger"
                        style={{ padding: "6px 12px", fontSize: 12 }}
                      >
                        Cancel
                      </button>
                    )}
                    {(job.status === "failed" || job.status === "cancelled") && (
                      <button
                        onClick={() => handleRetry(job.id)}
                        className="btn btn-primary"
                        style={{ padding: "6px 12px", fontSize: 12 }}
                      >
                        <RotateCcw size={12} /> Retry
                      </button>
                    )}
                  </div>
                </div>

                {/* Expanded Detail Panel */}
                {isExpanded && (
                  <div style={{ borderTop: "1px solid var(--border-color)", padding: 20, background: "rgba(255, 255, 255, 0.01)" }}>
                    {/* Progress bar */}
                    {job.progressTotal > 0 && (
                      <div style={{ marginBottom: 18 }}>
                        <div style={{ height: 6, background: "rgba(255, 255, 255, 0.05)", borderRadius: 3, overflow: "hidden" }}>
                          <div
                            style={{
                              height: "100%",
                              width: `${Math.min(100, (job.progress / job.progressTotal) * 100)}%`,
                              background: job.status === "failed" ? "var(--color-danger)" : job.status === "completed" ? "var(--color-success)" : "var(--color-primary)",
                              borderRadius: 3,
                              transition: "width 0.3s ease",
                            }}
                          />
                        </div>
                        <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 6, fontWeight: 500 }}>
                          {job.progress} / {job.progressTotal} steps completed
                        </div>
                      </div>
                    )}

                    {/* Events Timeline */}
                    <div style={{ marginBottom: 18 }}>
                      <h4 style={{ fontSize: 13, fontWeight: 700, marginBottom: 10, color: "var(--text-primary)" }}>Events</h4>
                      {events.length === 0 ? (
                        <p style={{ fontSize: 12, color: "var(--text-muted)", fontStyle: "italic" }}>No events recorded.</p>
                      ) : (
                        <div style={{ borderLeft: "2px solid var(--border-color)", marginLeft: 6, paddingLeft: 16 }}>
                          {events.map((evt) => {
                            const parsed = parsePayload(evt.payload);
                            const label = eventTypeLabels[evt.type] ?? evt.type;
                            const summary = parsed ? renderPayloadSummary(evt.type, parsed) : null;
                            return (
                              <div key={evt.id} style={{ display: "flex", gap: 8, padding: "6px 0", fontSize: 12, position: "relative", alignItems: "center" }}>
                                <div style={{ position: "absolute", left: -22, top: "50%", transform: "translateY(-50%)", background: "var(--bg-card)", padding: 2 }}>
                                  <EventIcon type={evt.type} />
                                </div>
                                <span style={{ color: "var(--text-muted)", fontFamily: "var(--font-mono)", minWidth: 24, fontSize: 10 }}>#{evt.seq}</span>
                                <span style={{ fontWeight: 700, color: "var(--text-primary)" }}>{label}</span>
                                {summary && (
                                  <span style={{ color: "var(--text-secondary)" }}>
                                    {summary}
                                  </span>
                                )}
                                <span style={{ color: "var(--text-muted)", marginLeft: "auto", fontSize: 11, fontFamily: "var(--font-mono)" }}>
                                  {formatTime(evt.createdAt)}
                                </span>
                              </div>
                            );
                          })}
                        </div>
                      )}
                    </div>

                    {/* Artifacts */}
                    {artifacts.length > 0 && (
                      <div>
                        <h4 style={{ fontSize: 13, fontWeight: 700, marginBottom: 10, display: "flex", alignItems: "center", gap: 6, color: "var(--text-primary)" }}>
                          <Package size={14} color="var(--color-primary)" /> Artifacts
                        </h4>
                        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                          {artifacts.map((art) => (
                            <div key={art.id} style={{ padding: 12, background: "rgba(255,255,255,0.01)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", fontSize: 12 }}>
                              <div style={{ fontWeight: 700, marginBottom: 6, color: "var(--text-primary)" }}>
                                {art.artifactType} {art.filePath && `· ${art.filePath}`}
                              </div>
                              {art.content && (
                                <pre style={{ margin: 0, padding: 10, background: "var(--bg-input)", border: "1px solid var(--border-color)", color: "#93c5fd", borderRadius: 4, fontSize: 11, fontFamily: "var(--font-mono)", overflow: "auto", maxHeight: 100 }}>
                                  {art.content.length > 200 ? art.content.slice(0, 200) + "…" : art.content}
                                </pre>
                              )}
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* Security Activity Section */}
      <section style={{ marginTop: 32 }}>
        <h2 style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 16, fontSize: 16, fontWeight: 700, color: "var(--text-primary)" }}>
          <ShieldCheck size={18} color="var(--color-success)" />
          Security Activity
        </h2>
        {auditEntries.length === 0 ? (
          <p style={{ fontSize: 12, color: "var(--text-muted)", fontStyle: "italic" }}>No audit activity recorded.</p>
        ) : (
          <div className="card" style={{ padding: 0, overflow: "hidden" }}>
            <div style={{ overflowX: "auto" }}>
              <table className="custom-table" style={{ fontSize: 12 }}>
                <thead>
                  <tr>
                    <th style={{ padding: "10px 16px" }}>Action</th>
                    <th style={{ padding: "10px 16px" }}>Capability</th>
                    <th style={{ padding: "10px 16px" }}>Risk</th>
                    <th style={{ padding: "10px 16px" }}>Timestamp</th>
                  </tr>
                </thead>
                <tbody>
                  {auditEntries.map((entry) => {
                    const isHigh = entry.riskLevel === "high" || entry.riskLevel === "critical";
                    return (
                      <tr key={entry.id} className="table-row-interactive">
                        <td style={{ padding: "12px 16px" }}>
                          <div style={{ fontWeight: 700, color: "var(--text-primary)" }}>{entry.action}</div>
                          <div style={{ color: "var(--text-secondary)", fontSize: 11, marginTop: 2, fontFamily: "var(--font-mono)" }}>{entry.subject}</div>
                        </td>
                        <td style={{ padding: "12px 16px" }}>
                          <code style={{ fontFamily: "var(--font-mono)", background: "rgba(255,255,255,0.03)", padding: "2px 6px", borderRadius: 4 }}>{entry.capability}</code>
                        </td>
                        <td style={{ padding: "12px 16px" }}>
                          <span className={isHigh ? "badge badge-danger" : "badge badge-neutral"}>
                            {entry.riskLevel}
                          </span>
                        </td>
                        <td style={{ padding: "12px 16px", color: "var(--text-muted)", fontSize: 11 }}>
                          {new Date(entry.createdAt).toLocaleString()}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
