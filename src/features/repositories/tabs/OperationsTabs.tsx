import type { GitHubEvidence, GitHubIntegration, PatchProposal, VerificationCommand, VerificationResult, VerificationRun } from "../../../types";
import { Check, CheckCircle2, ExternalLink, Github, Play, RefreshCw, RotateCcw, X, XCircle } from "lucide-react";
import { PatchStatusBadge } from "./tabUi";
// --- GitHub Tab ---

export function GitHubTab({ integration, evidence, syncing, onSync }: {
  integration: GitHubIntegration | null;
  evidence: GitHubEvidence | null;
  syncing: boolean;
  onSync: () => void;
}) {
  if (!integration) {
    return (
      <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)" }}>
        <Github size={32} style={{ marginBottom: 8, opacity: 0.5 }} />
        <p style={{ fontSize: 13 }}>No GitHub integration detected.</p>
        <p style={{ fontSize: 11 }}>Ensure this repo has a GitHub remote and gh CLI is authenticated.</p>
      </div>
    );
  }

  return (
    <>
      <div style={{ padding: 16, background: "var(--bg-app)", borderRadius: 6, border: "1px solid var(--border-color)", marginBottom: 12 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
          <Github size={16} />
          <span style={{ fontWeight: 600, fontSize: 14 }}>{integration.githubOwner}/{integration.githubRepo}</span>
        </div>
        {integration.defaultBranch && (
          <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Default branch: {integration.defaultBranch}</div>
        )}
        {integration.lastSyncedAt && (
          <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 4 }}>
            Last synced: {new Date(integration.lastSyncedAt).toLocaleString()}
          </div>
        )}
      </div>

      <button
        onClick={onSync}
        disabled={syncing}
        style={{
          display: "flex", alignItems: "center", gap: 6, padding: "8px 16px",
          background: syncing ? "var(--text-muted)" : "var(--bg-input)", color: "var(--text-primary)",
          border: "1px solid var(--border-color)", borderRadius: 6, cursor: "pointer", fontSize: 13, fontWeight: 600,
        }}
      >
        <RefreshCw size={14} />
        {syncing ? "Syncing..." : "Sync GitHub Data"}
      </button>

      {evidence && (
        <div style={{ marginTop: 16, display: "grid", gap: 16 }}>
          {evidence.syncErrors.length > 0 && (
            <div style={{ padding: 10, borderLeft: "3px solid var(--color-danger)", background: "var(--color-danger-bg)", color: "var(--color-danger-text)", fontSize: 11 }}>
              {evidence.syncErrors.join("; ")}
            </div>
          )}
          <GitHubEvidenceList
            title="Workflow runs"
            items={evidence.workflowRuns.slice(0, 6).map((run) => ({
              id: run.id,
              label: run.workflowName,
              meta: [run.branch, run.conclusion ?? run.status].filter(Boolean).join(" · "),
              url: run.url,
            }))}
          />
          <GitHubEvidenceList
            title="Pull requests"
            items={evidence.pullRequests.slice(0, 6).map((pr) => ({
              id: pr.id,
              label: `#${pr.prNumber} ${pr.title}`,
              meta: [pr.state, pr.author].filter(Boolean).join(" · "),
              url: pr.url,
            }))}
          />
          <GitHubEvidenceList
            title="Releases"
            items={evidence.releases.slice(0, 6).map((release) => ({
              id: release.id,
              label: release.tagName,
              meta: [release.name, release.isDraft ? "draft" : null].filter(Boolean).join(" · "),
              url: release.url,
            }))}
          />
        </div>
      )}
    </>
  );
}

function GitHubEvidenceList({ title, items }: {
  title: string;
  items: Array<{ id: string; label: string; meta: string; url: string | null }>;
}) {
  return (
    <section>
      <h4 style={{ margin: "0 0 6px", fontSize: 12, color: "var(--text-primary)" }}>{title}</h4>
      {items.length === 0 ? (
        <p style={{ margin: 0, fontSize: 11, color: "var(--text-secondary)" }}>No synced records.</p>
      ) : (
        <div style={{ borderTop: "1px solid var(--border-color)" }}>
          {items.map((item) => (
            <div key={item.id} style={{ padding: "7px 0", borderBottom: "1px solid var(--border-color)" }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8 }}>
                <span style={{ minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 11, fontWeight: 600, color: "var(--text-primary)" }}>
                  {item.label}
                </span>
                {item.url && <ExternalLink size={12} color="var(--text-secondary)" aria-label={item.url} />}
              </div>
              {item.meta && <div style={{ marginTop: 2, fontSize: 10, color: "var(--text-secondary)" }}>{item.meta}</div>}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

// --- Verify Tab ---

function RiskBadge({ level }: { level: string }) {
  const colors: Record<string, { bg: string; fg: string }> = {
    high: { bg: "var(--color-danger-bg)", fg: "var(--color-danger-text)" },
    medium: { bg: "var(--color-warning-bg)", fg: "var(--color-warning-text)" },
    low: { bg: "var(--color-info-bg)", fg: "var(--color-info-text)" },
  };
  const c = colors[level] ?? { bg: "var(--bg-app)", fg: "var(--text-secondary)" };
  return (
    <span style={{ padding: "1px 6px", borderRadius: 3, fontSize: 10, fontWeight: 600, background: c.bg, color: c.fg, border: `1px solid ${c.fg}40` }}>
      {level}
    </span>
  );
}

export function VerifyTab({ commands, runningCmd, result, onRun, runs, batchRunning, onBatchRun }: {
  commands: VerificationCommand[]; runningCmd: string | null;
  result: VerificationResult | null; onRun: (cmd: VerificationCommand) => void;
  runs: VerificationRun[]; batchRunning: boolean; onBatchRun: () => void;
}) {
  const batchCommandCount = commands.filter((command) => command.category !== "install").length;

  return (
    <>
      {commands.length === 0 ? (
        <p style={{ color: "var(--text-secondary)", fontSize: 13 }}>No verification commands detected for this repository.</p>
      ) : (
        <>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{commands.length} command(s) detected</span>
            <button
              onClick={onBatchRun}
              disabled={batchRunning || batchCommandCount === 0}
              title="Review and run all detected non-install checks"
              style={{
                display: "flex", alignItems: "center", gap: 4, padding: "4px 10px",
                background: "var(--color-info-bg)", color: "var(--color-info-text)", border: "1px solid var(--color-info-border)",
                borderRadius: 4,
                cursor: batchRunning || batchCommandCount === 0 ? "not-allowed" : "pointer",
                fontSize: 12,
                opacity: batchRunning || batchCommandCount === 0 ? 0.6 : 1,
              }}
            >
              <Play size={12} />
              {batchRunning ? "Running checks..." : `Review & Run (${batchCommandCount})`}
            </button>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 16 }}>
            {commands.map((cmd) => {
              return (
              <div key={cmd.command} style={{ padding: 10, background: "var(--bg-app)", borderRadius: 6, border: "1px solid var(--border-color)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <div style={{ fontWeight: 600, fontSize: 12, fontFamily: "monospace" }}>{cmd.command}</div>
                  <div style={{ fontSize: 11, color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: 6 }}>
                    {cmd.name} - {cmd.category}
                    <RiskBadge level={cmd.riskLevel} />
                  </div>
                </div>
                <button
                  onClick={() => onRun(cmd)}
                  disabled={runningCmd === cmd.command}
                  title={`Review and run ${cmd.command}`}
                  style={{
                    display: "flex", alignItems: "center", gap: 4, padding: "4px 10px",
                    background: "var(--color-info-bg)", color: "var(--color-info-text)", border: "1px solid var(--color-info-border)",
                    borderRadius: 4,
                    cursor: runningCmd === cmd.command ? "not-allowed" : "pointer",
                    fontSize: 12,
                  }}
                >
                  <Play size={12} />
                  {runningCmd === cmd.command ? "Running..." : "Review & Run"}
                </button>
              </div>
              );
            })}
          </div>
        </>
      )}

      {result && (
        <div style={{ border: "1px solid var(--border-color)", borderRadius: 6, overflow: "hidden", marginBottom: 16 }}>
          <div style={{ padding: "8px 12px", background: result.success ? "var(--color-success-bg)" : "var(--color-danger-bg)", display: "flex", alignItems: "center", gap: 6 }}>
            {result.success ? <CheckCircle2 size={14} color="var(--color-success-text)" /> : <XCircle size={14} color="var(--color-danger-text)" />}
            <span style={{ fontWeight: 600, fontSize: 13, color: result.success ? "var(--color-success-text)" : "var(--color-danger-text)" }}>
              {result.success ? "Passed" : "Failed"} (exit {result.exitCode ?? "N/A"}, {result.durationMs}ms)
            </span>
          </div>
          {result.stdout && (
            <pre style={{ margin: 0, padding: 8, background: "var(--bg-app)", color: "var(--text-primary)", fontSize: 11, maxHeight: 100, overflow: "auto" }}>
              {result.stdout.length > 500 ? result.stdout.slice(0, 500) + "..." : result.stdout}
            </pre>
          )}
          {result.stderr && (
            <pre style={{ margin: 0, padding: 8, background: "var(--bg-app)", color: "var(--color-danger-text)", fontSize: 11, maxHeight: 80, overflow: "auto", borderTop: "1px solid var(--border-color)" }}>
              {result.stderr.length > 500 ? result.stderr.slice(0, 500) + "..." : result.stderr}
            </pre>
          )}
        </div>
      )}

      {runs.length > 0 && (
        <div>
          <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)", marginBottom: 6 }}>Previous Runs</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            {runs.map((run) => (
              <div key={run.id} style={{ padding: 8, background: "var(--bg-app)", borderRadius: 4, border: "1px solid var(--border-color)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  {run.success ? <CheckCircle2 size={12} color="var(--color-success-text)" /> : <XCircle size={12} color="var(--color-danger-text)" />}
                  <span style={{ fontSize: 11, fontFamily: "monospace", fontWeight: 600 }}>{run.command}</span>
                  <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>{run.category}</span>
                  <RiskBadge level={run.riskLevel} />
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 10, color: "var(--text-secondary)" }}>
                  {run.exitCode != null && <span>exit {run.exitCode}</span>}
                  <span>{run.durationMs}ms</span>
                  {run.timedOut && <span style={{ color: "var(--color-warning-text)" }}>timeout</span>}
                  <span>{new Date(run.createdAt).toLocaleString()}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </>
  );
}

// --- Patches Tab ---

export function PatchesTab({ patches, onApply, onReject, onRollback }: {
  patches: PatchProposal[];
  onApply: (id: string) => void;
  onReject: (id: string) => void;
  onRollback: (id: string) => void;
}) {
  if (patches.length === 0) {
    return <p style={{ color: "var(--text-secondary)", fontSize: 13 }}>No AI-generated patches for this repository.</p>;
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      {patches.map((p) => (
        <div key={p.id} style={{ padding: 12, background: "var(--bg-app)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
            <div>
              <span style={{ fontWeight: 600, fontSize: 13, color: "var(--text-primary)" }}>{p.description}</span>
              <PatchStatusBadge status={p.status} />
            </div>
          </div>
          <pre style={{ margin: 0, padding: 8, background: "var(--bg-input)", color: "var(--text-primary)", borderRadius: 4, fontSize: 11, maxHeight: 100, overflow: "auto" }}>
            {p.patchContent.length > 400 ? p.patchContent.slice(0, 400) + "..." : p.patchContent}
          </pre>
          <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
            {p.status === "proposed" && (
              <>
                <button onClick={() => onApply(p.id)} style={{ display: "flex", alignItems: "center", gap: 4, padding: "4px 10px", background: "var(--color-success-bg)", color: "var(--color-success-text)", border: "1px solid var(--color-success-border)", borderRadius: 4, cursor: "pointer", fontSize: 12 }}>
                  <Check size={12} /> Apply
                </button>
                <button onClick={() => onReject(p.id)} style={{ display: "flex", alignItems: "center", gap: 4, padding: "4px 10px", background: "var(--color-danger-bg)", color: "var(--color-danger-text)", border: "1px solid var(--color-danger-border)", borderRadius: 4, cursor: "pointer", fontSize: 12 }}>
                  <X size={12} /> Reject
                </button>
              </>
            )}
            {p.status === "applied" && (
              <button onClick={() => onRollback(p.id)} style={{ display: "flex", alignItems: "center", gap: 4, padding: "4px 10px", background: "var(--color-warning-bg)", color: "var(--color-warning-text)", border: "1px solid var(--color-warning-border)", borderRadius: 4, cursor: "pointer", fontSize: 12 }}>
                <RotateCcw size={12} /> Rollback
              </button>
            )}
          </div>
          {p.appliedAt && (
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 4 }}>
              Applied: {new Date(p.appliedAt).toLocaleString()}
            </div>
          )}
          {p.status === "applied" && p.verificationResult && (
            <div style={{ marginTop: 8, padding: 8, background: "var(--color-success-bg)", borderRadius: 4, border: "1px solid var(--color-success-border)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 4, marginBottom: 4 }}>
                <CheckCircle2 size={12} color="var(--color-success-text)" />
                <span style={{ fontSize: 11, fontWeight: 600, color: "var(--color-success-text)" }}>Post-apply Verification</span>
              </div>
              <pre style={{ margin: 0, fontSize: 10, color: "var(--text-primary)", whiteSpace: "pre-wrap" }}>
                {p.verificationResult}
              </pre>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
