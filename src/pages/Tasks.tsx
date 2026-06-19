import { useEffect, useState } from "react";
import { listJobs, getJobEvents, cancelJob, retryJob, listArtifacts } from "../api/ipc";
import type { Job, JobEvent, Artifact } from "../types";
import { ListTodo, ChevronDown, ChevronRight, XCircle, RotateCcw, Package, Clock, AlertTriangle, CheckCircle2, Loader2, Ban } from "lucide-react";
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
    return <AlertTriangle size={12} style={{ color: "#dc2626" }} />;
  }
  if (type.endsWith("_completed") || type === "job_completed") {
    return <CheckCircle2 size={12} style={{ color: "#16a34a" }} />;
  }
  if (type === "job_cancelled" || type === "root_skipped") {
    return <Ban size={12} style={{ color: "#94a3b8" }} />;
  }
  if (type.includes("started")) {
    return <Loader2 size={12} style={{ color: "#2563eb" }} />;
  }
  return <Clock size={12} style={{ color: "#94a3b8" }} />;
}

/** Render a concise summary from a parsed event payload. */
function renderPayloadSummary(type: string, payload: Record<string, unknown>): string | null {
  // Error events
  if ("error" in payload && typeof payload.error === "string") {
    return payload.error.length > 120 ? payload.error.slice(0, 120) + "…" : payload.error;
  }
  // Scan summary
  if (type === "scan_summary") {
    const parts: string[] = [];
    if ("reposDiscovered" in payload) parts.push(`${payload.reposDiscovered} repos`);
    if ("rootsScanned" in payload) parts.push(`${payload.rootsScanned} roots scanned`);
    if ("rootsSkipped" in payload) parts.push(`${payload.rootsSkipped} skipped`);
    return parts.join(", ") || null;
  }
  // Root skipped
  if (type === "root_skipped") {
    const label = typeof payload.label === "string" ? payload.label : "";
    const reason = typeof payload.reason === "string" ? payload.reason : "";
    return [label, reason].filter(Boolean).join(" — ") || null;
  }
  // Audit completed
  if (type === "audit_completed") {
    const score = payload.score;
    const repoId = typeof payload.repoId === "string" ? payload.repoId : "";
    return `Score: ${score}${repoId ? ` (${repoId.slice(0, 8)}…)` : ""}`;
  }
  // Reindex completed
  if (type === "reindex_completed") {
    const docs = payload.documents;
    const chunks = payload.chunks;
    return `${docs} documents, ${chunks} chunks`;
  }
  // Verification
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
  // GitHub sync completed
  if (type === "github_sync_completed") {
    const parts: string[] = [];
    if (typeof payload.workflows === "number") parts.push(`${payload.workflows} workflows`);
    if (typeof payload.prs === "number") parts.push(`${payload.prs} PRs`);
    if (typeof payload.releases === "number") parts.push(`${payload.releases} releases`);
    return parts.join(", ") || null;
  }
  // AI call completed
  if (type === "ai_call_completed") {
    const parts: string[] = [];
    if (typeof payload.tokensIn === "number") parts.push(`${payload.tokensIn} in`);
    if (typeof payload.tokensOut === "number") parts.push(`${payload.tokensOut} out`);
    return parts.length > 0 ? `${parts.join(", ")} tokens` : null;
  }
  // Repo discovered / profiled
  if (type === "scan_repo_discovered") {
    const path = typeof payload.path === "string" ? payload.path : "";
    return path.length > 80 ? path.slice(0, 80) + "…" : path;
  }
  // Generic: show repoId, rootId, rootCount if present
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
  pending: <Clock size={14} style={{ color: "#92400e" }} />,
  running: <Loader2 size={14} style={{ color: "#1e40af" }} />,
  completed: <CheckCircle2 size={14} style={{ color: "#166534" }} />,
  failed: <AlertTriangle size={14} style={{ color: "#991b1b" }} />,
  cancelled: <Ban size={14} style={{ color: "#475569" }} />,
};

const statusColors: Record<string, { bg: string; fg: string }> = {
  pending: { bg: "#fef3c7", fg: "#92400e" },
  running: { bg: "#dbeafe", fg: "#1e40af" },
  completed: { bg: "#dcfce7", fg: "#166534" },
  failed: { bg: "#fef2f2", fg: "#991b1b" },
  cancelled: { bg: "#f1f5f9", fg: "#475569" },
};

export function Tasks() {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [expandedJob, setExpandedJob] = useState<string | null>(null);
  const [events, setEvents] = useState<JobEvent[]>([]);
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);

  useEffect(() => {
    loadJobs();
  }, []);

  async function loadJobs() {
    try {
      setError(null);
      setJobs(await listJobs(100));
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load jobs");
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
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 24 }}>
        <h1 style={{ fontSize: 24, fontWeight: 700 }}>Tasks</h1>
        <button
          onClick={loadJobs}
          style={{ padding: "6px 12px", border: "1px solid #e2e8f0", borderRadius: 6, background: "#fff", cursor: "pointer", fontSize: 13 }}
        >
          Refresh
        </button>
      </div>

      {error && (
        <div style={{ padding: 12, background: "#fef2f2", border: "1px solid #fca5a5", borderRadius: 6, marginBottom: 16, color: "#991b1b" }}>
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
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {jobs.map((job) => {
            const isExpanded = expandedJob === job.id;
            const c = statusColors[job.status] ?? { bg: "#f1f5f9", fg: "#475569" };
            return (
              <div key={job.id} style={{ background: "#fff", borderRadius: 8, border: "1px solid #e2e8f0", overflow: "hidden" }}>
                {/* Job Header */}
                <div
                  style={{ padding: "12px 16px", display: "flex", alignItems: "center", gap: 12, cursor: "pointer" }}
                  onClick={() => toggleExpand(job.id)}
                >
                  {isExpanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
                  <div style={{ flex: 1 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      {statusIcons[job.status] ?? null}
                      <span style={{ fontWeight: 600, fontSize: 14 }}>{job.type}</span>
                      <span style={{ padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600, background: c.bg, color: c.fg }}>
                        {job.status}
                      </span>
                      {job.progressTotal > 0 && (
                        <span style={{ fontSize: 11, color: "#64748b" }}>
                          {job.progress}/{job.progressTotal}
                        </span>
                      )}
                    </div>
                    <div style={{ fontSize: 12, color: "#64748b", marginTop: 2 }}>
                      {new Date(job.createdAt).toLocaleString()}
                      {job.completedAt && <> · Completed: {new Date(job.completedAt).toLocaleString()}</>}
                      {job.parentJobId && (
                        <span style={{ marginLeft: 8, color: "#94a3b8" }}>
                          (retry of {job.parentJobId.slice(0, 8)}…)
                        </span>
                      )}
                    </div>
                    {job.errorMessage && (
                      <div style={{ fontSize: 12, color: "#991b1b", marginTop: 4, background: "#fef2f2", padding: "4px 8px", borderRadius: 4 }}>
                        {job.errorMessage.length > 200 ? job.errorMessage.slice(0, 200) + "…" : job.errorMessage}
                      </div>
                    )}
                  </div>
                  <div style={{ display: "flex", gap: 6 }}>
                    {job.status === "running" && (
                      <button
                        onClick={(e) => { e.stopPropagation(); handleCancel(job.id); }}
                        style={{ display: "flex", alignItems: "center", gap: 4, padding: "4px 8px", background: "#fef2f2", color: "#991b1b", border: "1px solid #fca5a5", borderRadius: 4, cursor: "pointer", fontSize: 12 }}
                      >
                        <XCircle size={12} /> Cancel
                      </button>
                    )}
                    {(job.status === "failed" || job.status === "cancelled") && (
                      <button
                        onClick={(e) => { e.stopPropagation(); handleRetry(job.id); }}
                        style={{ display: "flex", alignItems: "center", gap: 4, padding: "4px 8px", background: "#f0f9ff", color: "#0369a1", border: "1px solid #bae6fd", borderRadius: 4, cursor: "pointer", fontSize: 12 }}
                      >
                        <RotateCcw size={12} /> Retry
                      </button>
                    )}
                  </div>
                </div>

                {/* Expanded Detail Panel */}
                {isExpanded && (
                  <div style={{ borderTop: "1px solid #e2e8f0", padding: "12px 16px", background: "#f8fafc" }}>
                    {/* Progress bar */}
                    {job.progressTotal > 0 && (
                      <div style={{ marginBottom: 12 }}>
                        <div style={{ height: 6, background: "#e2e8f0", borderRadius: 3, overflow: "hidden" }}>
                          <div
                            style={{
                              height: "100%",
                              width: `${Math.min(100, (job.progress / job.progressTotal) * 100)}%`,
                              background: job.status === "failed" ? "#ef4444" : job.status === "completed" ? "#22c55e" : "#3b82f6",
                              borderRadius: 3,
                              transition: "width 0.3s ease",
                            }}
                          />
                        </div>
                        <div style={{ fontSize: 11, color: "#94a3b8", marginTop: 2 }}>
                          {job.progress} / {job.progressTotal} steps
                        </div>
                      </div>
                    )}

                    {/* Events Timeline */}
                    <div style={{ marginBottom: 12 }}>
                      <h4 style={{ fontSize: 13, fontWeight: 600, marginBottom: 6 }}>Events</h4>
                      {events.length === 0 ? (
                        <p style={{ fontSize: 12, color: "#94a3b8" }}>No events recorded.</p>
                      ) : (
                        <div style={{ borderLeft: "2px solid #e2e8f0", marginLeft: 6, paddingLeft: 12 }}>
                          {events.map((evt) => {
                            const parsed = parsePayload(evt.payload);
                            const label = eventTypeLabels[evt.type] ?? evt.type;
                            const summary = parsed ? renderPayloadSummary(evt.type, parsed) : null;
                            return (
                              <div key={evt.id} style={{ display: "flex", gap: 8, padding: "4px 0", fontSize: 12, position: "relative" }}>
                                <div style={{ position: "absolute", left: -17, top: 4 }}>
                                  <EventIcon type={evt.type} />
                                </div>
                                <span style={{ color: "#94a3b8", fontFamily: "monospace", minWidth: 20, fontSize: 10 }}>#{evt.seq}</span>
                                <span style={{ fontWeight: 500, color: "#475569" }}>{label}</span>
                                {summary && (
                                  <span style={{ color: "#64748b" }}>
                                    {summary}
                                  </span>
                                )}
                                <span style={{ color: "#cbd5e1", marginLeft: "auto", fontSize: 11, whiteSpace: "nowrap" }}>
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
                        <h4 style={{ fontSize: 13, fontWeight: 600, marginBottom: 6, display: "flex", alignItems: "center", gap: 4 }}>
                          <Package size={12} /> Artifacts
                        </h4>
                        {artifacts.map((art) => (
                          <div key={art.id} style={{ padding: 8, background: "#fff", borderRadius: 4, marginBottom: 4, border: "1px solid #e2e8f0", fontSize: 12 }}>
                            <div style={{ fontWeight: 500, marginBottom: 2 }}>
                              {art.artifactType} {art.filePath && `· ${art.filePath}`}
                            </div>
                            {art.content && (
                              <pre style={{ margin: 0, padding: 6, background: "#1e293b", color: "#e2e8f0", borderRadius: 3, fontSize: 11, overflow: "auto", maxHeight: 80 }}>
                                {art.content.length > 200 ? art.content.slice(0, 200) + "…" : art.content}
                              </pre>
                            )}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}