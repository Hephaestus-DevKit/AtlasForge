import { useMemo } from "react";
import type { CategoryScore, Finding, HealthSnapshot, RecommendedTask, RepoProfile, VerificationResult } from "../../../types";
import { Activity, CheckCircle2, Code2, FileText, Github, Layers, Package, RefreshCw, Shield, Terminal, XCircle } from "lucide-react";
import { EmptyText, MiniBadge, ProfileSection, SeverityBadge, Tag } from "./tabUi";
// --- Overview Tab ---

export function OverviewTab({ profile, snapshot, findings, verifyResult, reindexing, onAudit, onReindex, onSync }: {
  profile: RepoProfile | null;
  snapshot: HealthSnapshot | null;
  findings: Finding[];
  verifyResult: VerificationResult | null;
  reindexing: boolean;
  onAudit: () => void;
  onReindex: () => void;
  onSync: () => void;
}) {
  // Parse categoryScores JSON
  let categoryScores: Record<string, CategoryScore> = {};
  let recommendedTasks: RecommendedTask[] = [];
  if (snapshot) {
    try { categoryScores = JSON.parse(snapshot.categoryScores); } catch { /* ignore parse errors */ }
    try { recommendedTasks = JSON.parse(snapshot.recommendedTasks); } catch { /* ignore parse errors */ }
  }

  return (
    <div>
      {/* Quick Actions */}
      <div style={{ marginBottom: 16 }}>
        <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 8, textTransform: "uppercase", letterSpacing: 0.5 }}>Quick Actions</h4>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
          <button onClick={onAudit} style={{ display: "flex", alignItems: "center", gap: 4, padding: "6px 12px", background: "var(--color-success)", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer", fontSize: 12, fontWeight: 600 }}>
            <Shield size={12} /> Run Audit
          </button>
          <button onClick={onReindex} disabled={reindexing} style={{ display: "flex", alignItems: "center", gap: 4, padding: "6px 12px", background: reindexing ? "var(--text-muted)" : "var(--color-primary)", color: "#fff", border: "none", borderRadius: 4, cursor: reindexing ? "not-allowed" : "pointer", fontSize: 12, fontWeight: 600 }}>
            <RefreshCw size={12} /> {reindexing ? "Indexing..." : "Reindex"}
          </button>
          <button onClick={onSync} style={{ display: "flex", alignItems: "center", gap: 4, padding: "6px 12px", background: "var(--text-secondary)", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer", fontSize: 12, fontWeight: 600 }}>
            <Github size={12} /> Sync GitHub
          </button>
        </div>
      </div>

      {/* Profile Summary */}
      {profile && (
        <div style={{ marginBottom: 16 }}>
          <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 8, textTransform: "uppercase", letterSpacing: 0.5 }}>Tech Stack</h4>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
            {profile.languages.map((l) => <Tag key={l} label={l} color="var(--color-primary)" />)}
            {profile.frameworks.map((f) => <Tag key={f} label={f} color="var(--color-accent)" />)}
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 6 }}>
            <MiniBadge label="README" active={profile.hasReadme} />
            <MiniBadge label="LICENSE" active={profile.hasLicense} />
            <MiniBadge label="CI" active={profile.ciSystems.length > 0} />
          </div>
        </div>
      )}

      {/* Health Score Summary */}
      {snapshot && (
        <div style={{ marginBottom: 16 }}>
          <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 8, textTransform: "uppercase", letterSpacing: 0.5 }}>Health</h4>
          <div style={{ display: "flex", alignItems: "center", gap: 12, padding: 12, background: "var(--bg-app)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
            <div style={{
              width: 44, height: 44, borderRadius: "50%", display: "flex", alignItems: "center", justifyContent: "center",
              background: snapshot.score >= 80 ? "var(--color-success-bg)" : snapshot.score >= 50 ? "var(--color-warning-bg)" : "var(--color-danger-bg)",
              color: snapshot.score >= 80 ? "var(--color-success-text)" : snapshot.score >= 50 ? "var(--color-warning-text)" : "var(--color-danger-text)",
              border: snapshot.score >= 80 ? "1px solid var(--color-success-border)" : snapshot.score >= 50 ? "1px solid var(--color-warning-border)" : "1px solid var(--color-danger-border)",
              fontSize: 16, fontWeight: 700,
            }}>
              {snapshot.score}
            </div>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>Score</div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>{findings.length} findings</div>
            </div>
          </div>
          {Object.keys(categoryScores).length > 0 && (
            <div style={{ marginTop: 8, display: "flex", flexDirection: "column", gap: 4 }}>
              {Object.entries(categoryScores).map(([cat, cs]) => (
                <div key={cat} style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 11 }}>
                  <span style={{ minWidth: 90, color: "var(--text-secondary)", fontWeight: 500 }}>{cat}</span>
                  <div style={{ flex: 1, height: 6, background: "var(--border-color)", borderRadius: 3, overflow: "hidden" }}>
                    <div style={{ width: `${cs.maxScore > 0 ? (cs.score / cs.maxScore) * 100 : 0}%`, height: "100%", background: cs.score / (cs.maxScore || 1) >= 0.8 ? "var(--color-success)" : cs.score / (cs.maxScore || 1) >= 0.5 ? "var(--color-warning)" : "var(--color-danger)", borderRadius: 3 }} />
                  </div>
                  <span style={{ color: "var(--text-secondary)", minWidth: 32, textAlign: "right" }}>{cs.score}/{cs.maxScore}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Verification Summary */}
      {verifyResult && (
        <div style={{ marginBottom: 16 }}>
          <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 8, textTransform: "uppercase", letterSpacing: 0.5 }}>Last Verification</h4>
          <div style={{ display: "flex", alignItems: "center", gap: 6, padding: 8, background: verifyResult.success ? "var(--color-success-bg)" : "var(--color-danger-bg)", borderRadius: 4, border: `1px solid ${verifyResult.success ? "var(--color-success-border)" : "var(--color-danger-border)"}` }}>
            {verifyResult.success ? <CheckCircle2 size={14} color="var(--color-success-text)" /> : <XCircle size={14} color="var(--color-danger-text)" />}
            <span style={{ fontSize: 12, fontWeight: 600, color: verifyResult.success ? "var(--color-success-text)" : "var(--color-danger-text)" }}>
              {verifyResult.success ? "Passed" : "Failed"}
            </span>
            {verifyResult.exitCode !== undefined && (
              <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>exit: {verifyResult.exitCode}</span>
            )}
          </div>
        </div>
      )}

      {/* Recommended Tasks */}
      {recommendedTasks.length > 0 && (
        <div>
          <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 8, textTransform: "uppercase", letterSpacing: 0.5 }}>Recommended Tasks</h4>
          {recommendedTasks.map((task, i) => (
            <div key={i} style={{ padding: 8, background: "rgba(255,255,255,0.02)", borderRadius: 4, marginBottom: 4, border: "1px solid var(--border-color)", fontSize: 12 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                <SeverityBadge severity={task.priority} />
                <span style={{ fontWeight: 600, color: "var(--text-primary)" }}>{task.title}</span>
              </div>
              <div style={{ color: "var(--text-secondary)", fontSize: 11, marginTop: 2 }}>{task.description}</div>
            </div>
          ))}
        </div>
      )}

      {!profile && !snapshot && !verifyResult && (
        <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)" }}>
          <Activity size={32} style={{ marginBottom: 8, opacity: 0.5 }} />
          <p style={{ fontSize: 13 }}>No data yet for this repository.</p>
          <p style={{ fontSize: 11 }}>Use the actions above to audit, index, or sync this repo.</p>
        </div>
      )}
    </div>
  );
}

// --- Profile Tab ---

export function ProfileTab({ profile }: { profile: RepoProfile | null }) {
  if (!profile) {
    return (
      <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)" }}>
        <Shield size={32} style={{ marginBottom: 8, opacity: 0.5 }} />
        <p style={{ fontSize: 13 }}>No profile available</p>
        <p style={{ fontSize: 11 }}>Run a scan or click "Refresh Profiles" to detect tech stack.</p>
      </div>
    );
  }

  return (
    <>
      <ProfileSection icon={Code2} title="Languages" color="var(--color-primary)">
        {profile.languages.length > 0 ? (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
            {profile.languages.map((lang) => <Tag key={lang} label={lang} color="var(--color-primary)" />)}
          </div>
        ) : <EmptyText>No languages detected</EmptyText>}
      </ProfileSection>

      <ProfileSection icon={Package} title="Frameworks" color="var(--color-accent)">
        {profile.frameworks.length > 0 ? (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
            {profile.frameworks.map((fw) => <Tag key={fw} label={fw} color="var(--color-accent)" />)}
          </div>
        ) : <EmptyText>No frameworks detected</EmptyText>}
      </ProfileSection>

      <ProfileSection icon={Layers} title="Package Managers" color="var(--color-success)">
        {profile.packageManagers.length > 0 ? (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
            {profile.packageManagers.map((pm) => <Tag key={pm} label={pm} color="var(--color-success)" />)}
          </div>
        ) : <EmptyText>No package managers detected</EmptyText>}
      </ProfileSection>

      <ProfileSection icon={Terminal} title="Scripts" color="var(--color-warning)">
        {Object.keys(profile.scripts).length > 0 ? (
          <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
            {Object.entries(profile.scripts).map(([key, value]) => (
              <div key={key} style={{ fontSize: 11, display: "flex", gap: 6 }}>
                <span style={{ color: "var(--color-warning-text)", fontWeight: 600, minWidth: 80 }}>{key.replace(/^(npm|cargo|python):/, "")}</span>
                <span style={{ color: "var(--text-secondary)", fontFamily: "monospace", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{value}</span>
              </div>
            ))}
          </div>
        ) : <EmptyText>No scripts detected</EmptyText>}
      </ProfileSection>

      <ProfileSection icon={RefreshCw} title="CI Systems" color="var(--color-info)">
        {profile.ciSystems.length > 0 ? (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
            {profile.ciSystems.map((ci) => <Tag key={ci} label={ci} color="var(--color-info)" />)}
          </div>
        ) : <EmptyText>No CI systems detected</EmptyText>}
      </ProfileSection>

      <ProfileSection icon={FileText} title="Documentation" color="var(--text-secondary)">
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <MiniBadge label="README" active={profile.hasReadme} />
          <MiniBadge label="LICENSE" active={profile.hasLicense} />
          {profile.licenseType && (
            <span style={{ padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600, background: "var(--color-info-bg)", color: "var(--color-info-text)", border: "1px solid var(--color-info-border)" }}>
              {profile.licenseType}
            </span>
          )}
        </div>
      </ProfileSection>

      <div style={{ marginTop: 16, fontSize: 10, color: "var(--text-secondary)" }}>
        Profiled: {new Date(profile.detectedAt).toLocaleString()}
      </div>
    </>
  );
}

// --- Audit Tab ---

export function AuditTab({ snapshot, findings, auditing, onAudit }: {
  snapshot: HealthSnapshot | null; findings: Finding[]; auditing: boolean; onAudit: () => void;
}) {
  const categoryScores: Record<string, CategoryScore> | null = useMemo(() => {
    if (!snapshot?.categoryScores) return null;
    try { return JSON.parse(snapshot.categoryScores); } catch { return null; }
  }, [snapshot?.categoryScores]);

  const recommendedTasks: RecommendedTask[] = useMemo(() => {
    if (!snapshot?.recommendedTasks) return [];
    try { return JSON.parse(snapshot.recommendedTasks); } catch { return []; }
  }, [snapshot?.recommendedTasks]);

  return (
    <>
      <button
        onClick={onAudit}
        disabled={auditing}
        style={{
          display: "flex", alignItems: "center", gap: 6, padding: "8px 16px",
          background: auditing ? "var(--text-muted)" : "var(--color-success)", color: "#fff",
          border: "none", borderRadius: 6, cursor: "pointer", fontSize: 13, fontWeight: 600, marginBottom: 16,
        }}
      >
        <Shield size={14} />
        {auditing ? "Auditing..." : "Run Audit"}
      </button>

      {snapshot && (
        <>
          {/* Overall Score */}
          <div style={{ padding: 16, background: "var(--bg-app)", borderRadius: 6, border: "1px solid var(--border-color)", marginBottom: 12 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 8 }}>
              <div style={{
                width: 56, height: 56, borderRadius: "50%", display: "flex", alignItems: "center", justifyContent: "center",
                background: snapshot.score >= 80 ? "var(--color-success-bg)" : snapshot.score >= 50 ? "var(--color-warning-bg)" : "var(--color-danger-bg)",
                color: snapshot.score >= 80 ? "var(--color-success-text)" : snapshot.score >= 50 ? "var(--color-warning-text)" : "var(--color-danger-text)",
                border: snapshot.score >= 80 ? "1px solid var(--color-success-border)" : snapshot.score >= 50 ? "1px solid var(--color-warning-border)" : "1px solid var(--color-danger-border)",
                fontSize: 20, fontWeight: 700,
              }}>
                {snapshot.score}
              </div>
              <div>
                <div style={{ fontWeight: 600, fontSize: 14, color: "var(--text-primary)" }}>Health Score</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
                  {snapshot.score >= 80 ? "Good — repo meets most quality standards" :
                   snapshot.score >= 50 ? "Fair — some areas need attention" :
                   "Needs work — significant issues found"}
                </div>
              </div>
            </div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
              Checked: {new Date(snapshot.createdAt).toLocaleString()} · Weighted average across 10 categories
            </div>
          </div>

          {/* Category Breakdown */}
          {categoryScores && (
            <div style={{ marginBottom: 12 }}>
              <h4 style={{ fontSize: 13, fontWeight: 600, marginBottom: 8, color: "var(--text-primary)" }}>Category Breakdown</h4>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))", gap: 8 }}>
                {Object.entries(categoryScores).sort((a, b) => b[1].weight - a[1].weight).map(([cat, cs]) => (
                  <div key={cat} style={{ padding: 8, background: "var(--bg-input)", borderRadius: 4, border: "1px solid var(--border-color)", fontSize: 12 }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
                      <span style={{ fontWeight: 600, textTransform: "capitalize", color: "var(--text-primary)" }}>{cat.replace(/_/g, " ")}</span>
                      <span style={{ fontWeight: 700, color: cs.score >= 80 ? "var(--color-success-text)" : cs.score >= 50 ? "var(--color-warning-text)" : "var(--color-danger-text)" }}>
                        {cs.score}/{cs.maxScore}
                      </span>
                    </div>
                    <div style={{ height: 4, background: "var(--border-color)", borderRadius: 2, overflow: "hidden" }}>
                      <div style={{
                        width: `${(cs.score / cs.maxScore) * 100}%`, height: "100%",
                        background: cs.score >= 80 ? "var(--color-success)" : cs.score >= 50 ? "var(--color-warning)" : "var(--color-danger)",
                        borderRadius: 2,
                      }} />
                    </div>
                    <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 2 }}>
                      weight: {cs.weight.toFixed(1)} · {cs.findings.length} finding{cs.findings.length !== 1 ? "s" : ""}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Recommended Tasks */}
          {recommendedTasks.length > 0 && (
            <div style={{ marginBottom: 12 }}>
              <h4 style={{ fontSize: 13, fontWeight: 600, marginBottom: 8, color: "var(--text-primary)" }}>Recommended Actions ({recommendedTasks.length})</h4>
              {recommendedTasks.map((task, i) => (
                <div key={i} style={{ padding: 8, background: "var(--color-warning-bg)", borderRadius: 4, marginBottom: 4, border: "1px solid var(--color-warning-border)", fontSize: 12 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 2 }}>
                    <SeverityBadge severity={task.priority === "high" ? "critical" : task.priority === "medium" ? "warning" : "info"} />
                    <span style={{ fontWeight: 600, color: "var(--text-primary)" }}>{task.title}</span>
                    <span style={{ color: "var(--text-secondary)" }}>— {task.category}</span>
                  </div>
                  <div style={{ color: "var(--text-secondary)" }}>{task.description}</div>
                  {task.autoFixable && <div style={{ fontSize: 11, color: "var(--color-primary)", marginTop: 2 }}>⚡ Auto-fixable</div>}
                </div>
              ))}
            </div>
          )}

          {/* Findings */}
          {findings.length > 0 && (
            <div>
              <h4 style={{ fontSize: 13, fontWeight: 600, marginBottom: 8, color: "var(--text-primary)" }}>Findings ({findings.length})</h4>
              {findings.map((f) => (
                <div key={f.id} style={{ padding: 8, background: "var(--bg-input)", borderRadius: 4, marginBottom: 4, border: "1px solid var(--border-color)", fontSize: 12 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 2 }}>
                    <SeverityBadge severity={f.severity} />
                    <span style={{ fontWeight: 600, color: "var(--text-primary)" }}>{f.title}</span>
                    <span style={{ color: "var(--text-secondary)" }}> - {f.category}</span>
                  </div>
                  <div style={{ color: "var(--text-secondary)", marginBottom: 2 }}>{f.description}</div>
                  {f.filePath && <div style={{ fontFamily: "monospace", fontSize: 11, color: "var(--text-secondary)" }}>{f.filePath}</div>}
                  {f.suggestedFix && <div style={{ fontSize: 11, color: "var(--color-primary)", marginTop: 2 }}>💡 {f.suggestedFix}</div>}
                </div>
              ))}
            </div>
          )}
        </>
      )}

      {!snapshot && !auditing && (
        <p style={{ color: "var(--text-secondary)", fontSize: 13 }}>Click "Run Audit" to check repository health across 10 categories: runnable, tests, CI, docs, dependencies, security, release, git hygiene, public surface, and platform compatibility.</p>
      )}
    </>
  );
}
