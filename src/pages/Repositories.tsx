import { useEffect, useState, useMemo } from "react";
import {
  listRepositorySummaries, getRepoProfile, refreshProfiles,
  auditRepo, getHealthSnapshot, getFindings,
  resolveGitHubRepo, getGitHubIntegration, syncGitHub, getGitHubEvidence,
  detectCommands, runVerification, runBatchVerification, listVerificationRuns,
  listPatchProposals, applyPatch, rejectPatch, rollbackPatch,
  requestVerificationApproval, requestPatchApproval, decidePermissionRequest,
  reindexRepo,
  generateFixPlan, proposeFix, listFixPlans,
  listAiProviders, previewFixPlanContext,
} from "../api/ipc";
import type {
  Repository, RepoProfile, HealthSnapshot, Finding,
  CategoryScore, RecommendedTask,
  GitHubIntegration, GitHubEvidence, VerificationCommand, VerificationResult, VerificationRun,
  PatchProposal, Artifact, AiProvider, ContextPreview,
  PermissionRequest,
} from "../types";
import {
  GitBranch, ExternalLink, Code2, Package, FileText, Shield,
  Terminal, RefreshCw, ChevronRight, ChevronDown, Layers,
  Github, Play, CheckCircle2, XCircle, RotateCcw,
  Check, X, Search, ArrowUpDown,
  Activity, Wand2, Eye, AlertTriangle,
} from "lucide-react";
import { LoadingSpinner } from "../components/LoadingSpinner";
import { EmptyState } from "../components/EmptyState";
import { ApprovalModal } from "../components/ApprovalModal";

type DetailTab = "overview" | "profile" | "audit" | "fixes" | "github" | "verify" | "patches";
type SortKey = "path" | "branch" | "dirty" | "lastCommit" | "score" | "language";
type SortDir = "asc" | "desc";

interface Toast {
  id: number;
  message: string;
  type: "success" | "error" | "info";
}

type PendingApprovalAction =
  | { kind: "verification"; command: VerificationCommand }
  | { kind: "batch"; commands: VerificationCommand[] }
  | { kind: "patch"; proposalId: string };

let toastCounter = 0;

export function Repositories() {
  const [repos, setRepos] = useState<Repository[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectedRepoId, setSelectedRepoId] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<DetailTab>("profile");
  const [profile, setProfile] = useState<RepoProfile | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  // Audit state
  const [snapshot, setSnapshot] = useState<HealthSnapshot | null>(null);
  const [findings, setFindings] = useState<Finding[]>([]);
  const [auditing, setAuditing] = useState(false);

  // GitHub state
  const [integration, setIntegration] = useState<GitHubIntegration | null>(null);
  const [githubEvidence, setGitHubEvidence] = useState<GitHubEvidence | null>(null);
  const [syncing, setSyncing] = useState(false);

  // Verification state
  const [commands, setCommands] = useState<VerificationCommand[]>([]);
  const [runningCmd, setRunningCmd] = useState<string | null>(null);
  const [verifyResult, setVerifyResult] = useState<VerificationResult | null>(null);
  const [verifyRuns, setVerifyRuns] = useState<VerificationRun[]>([]);
  const [batchRunning, setBatchRunning] = useState(false);

  // Patches state
  const [patches, setPatches] = useState<PatchProposal[]>([]);

  // Fixes state
  const [fixPlans, setFixPlans] = useState<Artifact[]>([]);
  const [aiProviders, setAiProviders] = useState<AiProvider[]>([]);
  const [generatingPlan, setGeneratingPlan] = useState(false);
  const [proposingFix, setProposingFix] = useState(false);
  const [fixInstruction, setFixInstruction] = useState("");
  const [fixTargetFile, setFixTargetFile] = useState("");
  const [selectedProviderId, setSelectedProviderId] = useState<string>("");
  const [contextPreview, setContextPreview] = useState<ContextPreview | null>(null);
  const [previewingContext, setPreviewingContext] = useState(false);

  // Reindex state
  const [reindexing, setReindexing] = useState(false);

  // Filter & Sort state
  const [filterText, setFilterText] = useState("");
  const [filterDirty, setFilterDirty] = useState<"all" | "dirty" | "clean">("all");
  const [filterRemote, setFilterRemote] = useState<"all" | "has" | "none">("all");
  const [filterNoCi, setFilterNoCi] = useState(false);
  const [filterNoReadme, setFilterNoReadme] = useState(false);
  const [filterLanguage, setFilterLanguage] = useState<string>("all");
  const [sortKey, setSortKey] = useState<SortKey>("path");
  const [sortDir, setSortDir] = useState<SortDir>("asc");

  // Toast state
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [approvalRequests, setApprovalRequests] = useState<PermissionRequest[]>([]);
  const [approvalAction, setApprovalAction] = useState<PendingApprovalAction | null>(null);
  const [approvalBusy, setApprovalBusy] = useState(false);

  // Profiles cache for filter/sort
  const [profileCache, setProfileCache] = useState<Record<string, RepoProfile | null>>({});

  function showToast(message: string, type: Toast["type"] = "info") {
    const id = ++toastCounter;
    setToasts((prev) => [...prev, { id, message, type }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 4000);
  }

  useEffect(() => {
    loadRepos();
  }, []);

  async function loadRepos() {
    try {
      setLoading(true);
      setError(null);
      const summaries = await listRepositorySummaries();
      const repoList = summaries.map((summary) => summary.repository);
      setRepos(repoList);
      const cache = Object.fromEntries(
        summaries.map((summary) => [summary.repository.id, summary.profile]),
      );
      setProfileCache(cache);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load repositories");
    } finally {
      setLoading(false);
    }
  }

  async function handleSelectRepo(repoId: string) {
    if (selectedRepoId === repoId) {
      setSelectedRepoId(null);
      setProfile(null);
      setSnapshot(null);
      setFindings([]);
      setIntegration(null);
      setGitHubEvidence(null);
      setCommands([]);
      setVerifyResult(null);
      setVerifyRuns([]);
      setPatches([]);
      setFixPlans([]);
      setFixInstruction("");
      setFixTargetFile("");
      return;
    }
    setSelectedRepoId(repoId);
    setActiveTab("overview");
    setProfile(null);
    setSnapshot(null);
    setFindings([]);
    setIntegration(null);
    setGitHubEvidence(null);
    setCommands([]);
    setVerifyResult(null);
    setVerifyRuns([]);
    setPatches([]);
    setFixPlans([]);
    setFixInstruction("");
    setFixTargetFile("");
    loadTabData(repoId, "profile");
  }

  async function loadTabData(repoId: string, tab: DetailTab) {
    try {
      switch (tab) {
        case "overview":
        case "profile":
          setProfile(await getRepoProfile(repoId));
          break;
        case "audit": {
          const snap = await getHealthSnapshot(repoId);
          setSnapshot(snap);
          if (snap) setFindings(await getFindings(snap.id));
          break;
        }
        case "github":
          try {
            const resolved = await getGitHubIntegration(repoId)
              ?? await resolveGitHubRepo(repoId);
            setIntegration(resolved);
            setGitHubEvidence(await getGitHubEvidence(repoId));
          } catch {
            setIntegration(null);
            setGitHubEvidence(null);
          }
          break;
        case "verify": {
          const repo = repos.find(r => r.id === repoId);
          if (repo) setCommands(await detectCommands(repo.worktreePath));
          try { setVerifyRuns(await listVerificationRuns(repoId)); } catch { setVerifyRuns([]); }
          break;
        }
        case "patches":
          setPatches(await listPatchProposals(repoId));
          break;
        case "fixes": {
          try { setFixPlans(await listFixPlans(repoId)); } catch { setFixPlans([]); }
          try { setAiProviders(await listAiProviders()); } catch { setAiProviders([]); }
          break;
        }
      }
    } catch (e: any) {
      console.error(`Failed to load tab ${tab}:`, e);
    }
  }

  function handleTabChange(tab: DetailTab) {
    setActiveTab(tab);
    if (selectedRepoId) loadTabData(selectedRepoId, tab);
  }

  async function handleRefreshProfiles() {
    try {
      setRefreshing(true);
      setError(null);
      const count = await refreshProfiles();
      if (selectedRepoId) {
        setProfile(await getRepoProfile(selectedRepoId));
      }
      showToast(`Refreshed ${count} profiles`, "success");
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to refresh profiles");
    } finally {
      setRefreshing(false);
    }
  }

  async function handleAudit() {
    if (!selectedRepoId) return;
    try {
      setAuditing(true);
      const snap = await auditRepo(selectedRepoId);
      setSnapshot(snap);
      setFindings(await getFindings(snap.id));
      showToast("Audit completed", "success");
    } catch (e: any) {
      setError(e?.toString() ?? "Audit failed");
    } finally {
      setAuditing(false);
    }
  }

  async function handleSyncGitHub() {
    if (!selectedRepoId) return;
    try {
      setSyncing(true);
      const result = await syncGitHub(selectedRepoId);
      setGitHubEvidence(await getGitHubEvidence(selectedRepoId));
      showToast(`GitHub synced: ${JSON.stringify(result)}`, "success");
    } catch (e: any) {
      setError(e?.toString() ?? "GitHub sync failed");
    } finally {
      setSyncing(false);
    }
  }

  async function handleRunVerification(cmd: VerificationCommand) {
    if (!selectedRepo) return;
    try {
      const request = await requestVerificationApproval(
        selectedRepo.id,
        selectedRepo.worktreePath,
        cmd.command,
      );
      setApprovalRequests([request]);
      setApprovalAction({ kind: "verification", command: cmd });
    } catch (e: any) {
      setError(e?.toString() ?? "Could not prepare verification approval");
    }
  }

  async function handleBatchVerification() {
    if (!selectedRepo) return;
    const selectedCommands = commands.filter((command) => command.category !== "install");
    if (selectedCommands.length === 0) {
      showToast("No non-install verification commands are available", "info");
      return;
    }
    try {
      const requests = await Promise.all(
        selectedCommands.map((command) =>
          requestVerificationApproval(
            selectedRepo.id,
            selectedRepo.worktreePath,
            command.command,
          ),
        ),
      );
      setApprovalRequests(requests);
      setApprovalAction({ kind: "batch", commands: selectedCommands });
    } catch (e: any) {
      setError(e?.toString() ?? "Could not prepare batch approval");
    }
  }

  async function handleApplyPatch(id: string) {
    try {
      const request = await requestPatchApproval(id);
      setApprovalRequests([request]);
      setApprovalAction({ kind: "patch", proposalId: id });
    } catch (e: any) {
      setError(e?.toString() ?? "Could not prepare patch approval");
      showToast("Patch approval could not be prepared", "error");
    }
  }

  async function denyPendingApproval() {
    const requests = approvalRequests;
    setApprovalRequests([]);
    setApprovalAction(null);
    await Promise.allSettled(
      requests.map((request) => decidePermissionRequest(request.id, false)),
    );
  }

  async function approvePendingAction() {
    if (!approvalAction || !selectedRepo) return;
    setApprovalBusy(true);
    try {
      const approved = await Promise.all(
        approvalRequests.map((request) => decidePermissionRequest(request.id, true)),
      );
      if (approvalAction.kind === "verification") {
        setRunningCmd(approvalAction.command.command);
        setVerifyResult(null);
        const result = await runVerification(
          approvalAction.command.command,
          selectedRepo.worktreePath,
          selectedRepo.id,
          approved[0].id,
        );
        setVerifyResult(result);
        setVerifyRuns(await listVerificationRuns(selectedRepo.id));
      } else if (approvalAction.kind === "batch") {
        setBatchRunning(true);
        await runBatchVerification(
          approvalAction.commands,
          selectedRepo.worktreePath,
          selectedRepo.id,
          approved.map((request) => request.id),
        );
        setVerifyRuns(await listVerificationRuns(selectedRepo.id));
        showToast(`Batch verification complete (${approvalAction.commands.length} commands)`, "success");
      } else {
        await applyPatch(approvalAction.proposalId, approved[0].id);
        setPatches(await listPatchProposals(selectedRepo.id));
        showToast("Patch applied after isolated verification", "success");
      }
      setApprovalRequests([]);
      setApprovalAction(null);
    } catch (e: any) {
      setError(e?.toString() ?? "Approved operation failed");
      showToast("Approved operation failed", "error");
    } finally {
      setRunningCmd(null);
      setBatchRunning(false);
      setApprovalBusy(false);
    }
  }

  async function handleRejectPatch(id: string) {
    try { await rejectPatch(id, "Rejected by user"); if (selectedRepoId) setPatches(await listPatchProposals(selectedRepoId)); showToast("Patch rejected", "info"); }
    catch (e: any) { setError(e?.toString() ?? "Reject failed"); showToast("Reject failed", "error"); }
  }

  async function handleRollbackPatch(id: string) {
    try { await rollbackPatch(id); if (selectedRepoId) setPatches(await listPatchProposals(selectedRepoId)); showToast("Patch rolled back", "info"); }
    catch (e: any) { setError(e?.toString() ?? "Rollback failed"); showToast("Rollback failed", "error"); }
  }

  async function handleReindex() {
    if (!selectedRepoId) return;
    try {
      setReindexing(true);
      const stats = await reindexRepo(selectedRepoId);
      showToast(
        `Index updated: ${stats.indexedDocuments} changed, ${stats.skippedDocuments} unchanged`,
        "success",
      );
    } catch (e: any) {
      showToast("Reindex failed: " + (e?.toString() ?? "unknown error"), "error");
    } finally {
      setReindexing(false);
    }
  }

  async function handleGenerateFixPlan() {
    if (!selectedRepoId || !snapshot || !selectedProviderId) return;
    try {
      setGeneratingPlan(true);
      const plan = await generateFixPlan(selectedRepoId, snapshot.id, selectedProviderId);
      showToast(`Fix plan generated (tokens: ${plan.tokensIn}→${plan.tokensOut})`, "success");
      setFixPlans(await listFixPlans(selectedRepoId));
    } catch (e: any) {
      setError(e?.toString() ?? "Generate fix plan failed");
      showToast("Fix plan generation failed", "error");
    } finally {
      setGeneratingPlan(false);
    }
  }

  async function handleProposeFix() {
    if (!selectedRepoId || !selectedProviderId || !fixInstruction.trim()) return;
    try {
      setProposingFix(true);
      const proposal = await proposeFix(selectedRepoId, selectedProviderId, fixInstruction, undefined, fixTargetFile || undefined);
      showToast(`Fix proposed: ${proposal.description}`, "success");
      setPatches(await listPatchProposals(selectedRepoId));
      setFixInstruction("");
      setFixTargetFile("");
    } catch (e: any) {
      setError(e?.toString() ?? "Propose fix failed");
      showToast("Fix proposal failed", "error");
    } finally {
      setProposingFix(false);
    }
  }

  async function handlePreviewContext() {
    if (!selectedRepoId || !snapshot) return;
    try {
      setPreviewingContext(true);
      setContextPreview(null);
      const preview = await previewFixPlanContext(selectedRepoId, snapshot.id);
      setContextPreview(preview);
      showToast(`Context preview: ${preview.sections.length} sections, ~${preview.totalTokensEstimate} tokens`, "info");
    } catch (e: any) {
      setError(e?.toString() ?? "Context preview failed");
      showToast("Context preview failed", "error");
    } finally {
      setPreviewingContext(false);
    }
  }

  // Filter & Sort logic
  const availableLanguages = useMemo(() => {
    const langs = new Set<string>();
    Object.values(profileCache).forEach((p) => {
      if (p) p.languages.forEach((l) => langs.add(l));
    });
    return Array.from(langs).sort();
  }, [profileCache]);

  const filteredRepos = useMemo(() => {
    let result = repos;

    // Text filter
    if (filterText) {
      const lower = filterText.toLowerCase();
      result = result.filter((r) =>
        r.worktreePath.toLowerCase().includes(lower) ||
        (r.currentBranch ?? "").toLowerCase().includes(lower) ||
        (r.remoteOriginUrl ?? "").toLowerCase().includes(lower)
      );
    }

    // Dirty/clean filter
    if (filterDirty === "dirty") result = result.filter((r) => r.dirtyState);
    else if (filterDirty === "clean") result = result.filter((r) => !r.dirtyState);

    // Remote filter
    if (filterRemote === "has") result = result.filter((r) => r.remoteOriginUrl);
    else if (filterRemote === "none") result = result.filter((r) => !r.remoteOriginUrl);

    // No CI filter
    if (filterNoCi) {
      result = result.filter((r) => {
        const p = profileCache[r.id];
        return !p || p.ciSystems.length === 0;
      });
    }

    // No README filter
    if (filterNoReadme) {
      result = result.filter((r) => {
        const p = profileCache[r.id];
        return !p || !p.hasReadme;
      });
    }

    // Language filter
    if (filterLanguage !== "all") {
      result = result.filter((r) => {
        const p = profileCache[r.id];
        return p && p.languages.includes(filterLanguage);
      });
    }

    // Sort
    result = [...result].sort((a, b) => {
      let cmp = 0;
      switch (sortKey) {
        case "path":
          cmp = repoName(a.worktreePath).localeCompare(repoName(b.worktreePath));
          break;
        case "branch":
          cmp = (a.currentBranch ?? "").localeCompare(b.currentBranch ?? "");
          break;
        case "dirty":
          cmp = Number(a.dirtyState) - Number(b.dirtyState);
          break;
        case "lastCommit": {
          const da = a.lastCommitAt ? new Date(a.lastCommitAt).getTime() : 0;
          const db = b.lastCommitAt ? new Date(b.lastCommitAt).getTime() : 0;
          cmp = da - db;
          break;
        }
        case "score":
          cmp = 0; // scores not available in list view
          break;
        case "language": {
          const la = profileCache[a.id]?.languages[0] ?? "";
          const lb = profileCache[b.id]?.languages[0] ?? "";
          cmp = la.localeCompare(lb);
          break;
        }
      }
      return sortDir === "asc" ? cmp : -cmp;
    });

    return result;
  }, [repos, filterText, filterDirty, filterRemote, filterNoCi, filterNoReadme, filterLanguage, sortKey, sortDir, profileCache]);

  function toggleSort(key: SortKey) {
    if (sortKey === key) {
      setSortDir((d) => d === "asc" ? "desc" : "asc");
    } else {
      setSortKey(key);
      setSortDir("asc");
    }
  }

  const selectedRepo = repos.find((r) => r.id === selectedRepoId);
  const selectedProfile = (selectedRepoId ? profileCache[selectedRepoId] : null) ?? profile;

  const tabs: { key: DetailTab; label: string; icon: any }[] = [
    { key: "overview", label: "Overview", icon: Activity },
    { key: "profile", label: "Profile", icon: Code2 },
    { key: "audit", label: "Health", icon: Shield },
    { key: "fixes", label: "AI Fix", icon: Wand2 },
    { key: "verify", label: "Verify", icon: CheckCircle2 },
    { key: "patches", label: "Patches", icon: Layers },
    { key: "github", label: "GitHub", icon: Github },
  ];

  return (
    <div style={{ position: "relative" }}>
      {/* Toast container */}
      <div style={{ position: "fixed", top: 16, right: 16, zIndex: 1000, display: "flex", flexDirection: "column", gap: 8 }}>
        {toasts.map((t) => (
          <div
            key={t.id}
            className={`badge ${t.type === "success" ? "badge-success" : t.type === "error" ? "badge-danger" : "badge-info"}`}
            style={{
              padding: "10px 16px",
              borderRadius: "var(--radius-sm)",
              fontSize: 13,
              fontWeight: 500,
              maxWidth: 360,
              boxShadow: "var(--shadow-md)",
              display: "block",
            }}
          >
            {t.message}
          </div>
        ))}
      </div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 24 }}>
        <h1 style={{ fontSize: 24, fontWeight: 700 }}>Repositories</h1>
        <button
          onClick={handleRefreshProfiles}
          disabled={refreshing}
          className="btn btn-primary"
        >
          <RefreshCw size={16} className={refreshing ? "spin-slow" : ""} />
          {refreshing ? "Refreshing..." : "Refresh Profiles"}
        </button>
      </div>

      {error && (
        <div className="badge badge-danger" style={{ display: "block", padding: 12, borderRadius: "var(--radius-sm)", marginBottom: 16, fontSize: 13, width: "100%" }}>
          {error}
        </div>
      )}

      {loading ? (
        <LoadingSpinner message="Loading repositories..." />
      ) : repos.length === 0 ? (
        <EmptyState
          icon={GitBranch}
          title="No repositories discovered yet"
          description="Add workspace roots and start a scan from the Dashboard."
        />
      ) : (
        <div style={{ display: "grid", gridTemplateColumns: selectedRepoId ? "1fr 420px" : "1fr", gap: 16 }}>
          {/* Repo List */}
          <div style={{ background: "var(--bg-card)", borderRadius: 8, border: "1px solid var(--border-color)", overflow: "hidden" }}>
            {/* Filter bar */}
            <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", display: "flex", flexWrap: "wrap", gap: 8, alignItems: "center", background: "var(--bg-app)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 4, flex: "1 1 180px", minWidth: 140 }}>
                <Search size={14} color="var(--text-secondary)" />
                <input
                  type="text"
                  placeholder="Filter repos..."
                  value={filterText}
                  onChange={(e) => setFilterText(e.target.value)}
                  style={{ border: "1px solid var(--border-color)", borderRadius: 4, padding: "4px 8px", fontSize: 12, width: "100%", outline: "none", background: "var(--bg-input)", color: "var(--text-primary)" }}
                />
              </div>
              <select value={filterDirty} onChange={(e) => setFilterDirty(e.target.value as any)} style={{ fontSize: 11, padding: "3px 6px", border: "1px solid var(--border-color)", borderRadius: 4, background: "var(--bg-input)", color: "var(--text-primary)" }}>
                <option value="all">All status</option>
                <option value="dirty">Dirty only</option>
                <option value="clean">Clean only</option>
              </select>
              <select value={filterRemote} onChange={(e) => setFilterRemote(e.target.value as any)} style={{ fontSize: 11, padding: "3px 6px", border: "1px solid var(--border-color)", borderRadius: 4, background: "var(--bg-input)", color: "var(--text-primary)" }}>
                <option value="all">All remotes</option>
                <option value="has">Has remote</option>
                <option value="none">No remote</option>
              </select>
              {availableLanguages.length > 0 && (
                <select value={filterLanguage} onChange={(e) => setFilterLanguage(e.target.value)} style={{ fontSize: 11, padding: "3px 6px", border: "1px solid var(--border-color)", borderRadius: 4, background: "var(--bg-input)", color: "var(--text-primary)" }}>
                  <option value="all">All languages</option>
                  {availableLanguages.map((l) => <option key={l} value={l}>{l}</option>)}
                </select>
              )}
              <label htmlFor="af-filter-noci" style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 11, color: "var(--text-secondary)", cursor: "pointer" }}>
                <input type="checkbox" id="af-filter-noci" checked={filterNoCi} onChange={(e) => setFilterNoCi(e.target.checked)} /> No CI
              </label>
              <label htmlFor="af-filter-noreadme" style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 11, color: "var(--text-secondary)", cursor: "pointer" }}>
                <input type="checkbox" id="af-filter-noreadme" checked={filterNoReadme} onChange={(e) => setFilterNoReadme(e.target.checked)} /> No README
              </label>
            </div>

            <div style={{ overflowX: "auto" }}>
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
                <thead>
                  <tr style={{ background: "var(--bg-app)", borderBottom: "1px solid var(--border-color)" }}>
                    <th style={{ width: 32, padding: "10px 8px" }}></th>
                    <SortableHeader label="Path" sortKey="path" currentKey={sortKey} dir={sortDir} onSort={toggleSort} />
                    <SortableHeader label="Branch" sortKey="branch" currentKey={sortKey} dir={sortDir} onSort={toggleSort} />
                    <th style={{ textAlign: "left", padding: "10px 12px", color: "var(--text-secondary)", fontWeight: 600 }}>Head SHA</th>
                    <th style={{ textAlign: "left", padding: "10px 12px", color: "var(--text-secondary)", fontWeight: 600 }}>Remote</th>
                    <SortableHeader label="Status" sortKey="dirty" currentKey={sortKey} dir={sortDir} onSort={toggleSort} />
                    <SortableHeader label="Last Commit" sortKey="lastCommit" currentKey={sortKey} dir={sortDir} onSort={toggleSort} />
                  </tr>
                </thead>
                <tbody>
                  {filteredRepos.map((repo) => (
                  <tr
                    key={repo.id}
                    onClick={() => handleSelectRepo(repo.id)}
                    style={{
                      borderBottom: "1px solid var(--border-color)",
                      background: selectedRepoId === repo.id ? "rgba(99, 102, 241, 0.08)" : "transparent",
                      cursor: "pointer",
                    }}
                  >
                    <td style={{ padding: "10px 4px", textAlign: "center", color: "var(--text-secondary)" }}>
                      {selectedRepoId === repo.id ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                    </td>
                    <td style={{ padding: "10px 12px", fontFamily: "monospace", fontSize: 12, maxWidth: 280, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {repoName(repo.worktreePath)}
                      <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 2 }}>{repo.worktreePath}</div>
                    </td>
                    <td style={{ padding: "10px 12px" }}>
                      <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                        <GitBranch size={14} color="var(--text-secondary)" />
                        {repo.currentBranch ?? "—"}
                      </span>
                    </td>
                    <td style={{ padding: "10px 12px", fontFamily: "monospace", fontSize: 11, color: "var(--text-secondary)" }}>
                      {repo.headSha ? repo.headSha.slice(0, 8) : "—"}
                    </td>
                    <td style={{ padding: "10px 12px", maxWidth: 180, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {repo.remoteOriginUrl ? (
                        <span style={{ display: "inline-flex", alignItems: "center", gap: 4, fontSize: 12, color: "var(--color-primary)" }}>
                          <ExternalLink size={12} />
                          {repo.remoteOriginUrl.replace(/\.git$/, "").replace(/^https?:\/\//, "")}
                        </span>
                      ) : "—"}
                    </td>
                    <td style={{ padding: "10px 12px" }}>
                      {repo.dirtyState ? (
                        <span style={{ padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600, background: "var(--color-warning-bg)", color: "var(--color-warning-text)", border: "1px solid var(--color-warning-border)" }}>dirty</span>
                      ) : (
                        <span style={{ padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600, background: "var(--color-success-bg)", color: "var(--color-success-text)", border: "1px solid var(--color-success-border)" }}>clean</span>
                      )}
                      {repo.aheadBehind && (repo.aheadBehind.ahead > 0 || repo.aheadBehind.behind > 0) && (
                        <span style={{ marginLeft: 4, fontSize: 11, color: "var(--text-secondary)" }}>
                          ↑{repo.aheadBehind.ahead} ↓{repo.aheadBehind.behind}
                        </span>
                      )}
                    </td>
                    <td style={{ padding: "10px 12px", color: "var(--text-secondary)", fontSize: 12 }}>
                      {repo.lastCommitAt ? new Date(repo.lastCommitAt).toLocaleDateString() : "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            </div>
            {filteredRepos.length === 0 && repos.length > 0 && (
              <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 13 }}>
                No repositories match current filters.
              </div>
            )}
          </div>

          {/* Detail Panel */}
          {selectedRepoId && selectedRepo && (
            <div style={{ background: "var(--bg-card)", borderRadius: 8, border: "1px solid var(--border-color)", overflow: "hidden", maxHeight: "calc(100vh - 120px)", display: "flex", flexDirection: "column" }}>
              {/* Summary Header */}
              <div style={{ padding: "12px 16px", borderBottom: "1px solid var(--border-color)" }}>
                <h3 style={{ fontSize: 16, fontWeight: 700, marginBottom: 4, display: "flex", alignItems: "center", gap: 8 }}>
                  <Code2 size={18} color="var(--color-primary)" />
                  {repoName(selectedRepo.worktreePath)}
                </h3>
                <p style={{ fontSize: 11, color: "var(--text-secondary)", fontFamily: "monospace", marginBottom: 6 }}>
                  {selectedRepo.worktreePath}
                </p>
                <div style={{ display: "flex", flexWrap: "wrap", gap: 6, alignItems: "center" }}>
                  {selectedRepo.currentBranch && (
                    <span style={{ display: "inline-flex", alignItems: "center", gap: 3, fontSize: 11, color: "var(--text-secondary)" }}>
                      <GitBranch size={11} /> {selectedRepo.currentBranch}
                    </span>
                  )}
                  <span style={{ padding: "1px 6px", borderRadius: 3, fontSize: 10, fontWeight: 600, background: selectedRepo.dirtyState ? "var(--color-warning-bg)" : "var(--color-success-bg)", color: selectedRepo.dirtyState ? "var(--color-warning-text)" : "var(--color-success-text)", border: selectedRepo.dirtyState ? "1px solid var(--color-warning-border)" : "1px solid var(--color-success-border)" }}>
                    {selectedRepo.dirtyState ? "dirty" : "clean"}
                  </span>
                  {selectedRepo.remoteOriginUrl && (
                    <span style={{ display: "inline-flex", alignItems: "center", gap: 3, fontSize: 11, color: "var(--color-primary)" }}>
                      <ExternalLink size={11} /> {selectedRepo.remoteOriginUrl.replace(/\.git$/, "").replace(/^https?:\/\//, "").split("/").slice(-2).join("/")}
                    </span>
                  )}
                  {selectedProfile && selectedProfile.languages.length > 0 && (
                    <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>{selectedProfile.languages.join(", ")}</span>
                  )}
                  {snapshot && (
                    <span style={{ padding: "1px 6px", borderRadius: 3, fontSize: 10, fontWeight: 600, background: snapshot.score >= 80 ? "var(--color-success-bg)" : snapshot.score >= 50 ? "var(--color-warning-bg)" : "var(--color-danger-bg)", color: snapshot.score >= 80 ? "var(--color-success-text)" : snapshot.score >= 50 ? "var(--color-warning-text)" : "var(--color-danger-text)", border: snapshot.score >= 80 ? "1px solid var(--color-success-border)" : snapshot.score >= 50 ? "1px solid var(--color-warning-border)" : "1px solid var(--color-danger-border)" }}>
                      Score: {snapshot.score}
                    </span>
                  )}
                  {verifyResult && (
                    <span style={{ padding: "1px 6px", borderRadius: 3, fontSize: 10, fontWeight: 600, background: verifyResult.success ? "var(--color-success-bg)" : "var(--color-danger-bg)", color: verifyResult.success ? "var(--color-success-text)" : "var(--color-danger-text)", border: verifyResult.success ? "1px solid var(--color-success-border)" : "1px solid var(--color-danger-border)" }}>
                      Verify: {verifyResult.success ? "pass" : "fail"}
                    </span>
                  )}
                </div>
              </div>

              {/* Tabs */}
              <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", overflow: "auto" }}>
                {tabs.map((tab) => (
                  <button
                    key={tab.key}
                    onClick={() => handleTabChange(tab.key)}
                    style={{
                      display: "flex", alignItems: "center", gap: 4,
                      padding: "8px 12px", border: "none", background: "transparent",
                      cursor: "pointer", fontSize: 12, fontWeight: activeTab === tab.key ? 600 : 400,
                      color: activeTab === tab.key ? "var(--color-primary)" : "var(--text-secondary)",
                      borderBottom: activeTab === tab.key ? "2px solid var(--color-primary)" : "2px solid transparent",
                    }}
                  >
                    <tab.icon size={14} />
                    {tab.label}
                  </button>
                ))}
              </div>

              {/* Tab Content */}
              <div style={{ padding: 16, overflow: "auto", flex: 1 }}>
                {activeTab === "overview" && (
                  <OverviewTab
                    profile={selectedProfile}
                    snapshot={snapshot}
                    findings={findings}
                    verifyResult={verifyResult}
                    reindexing={reindexing}
                    onAudit={handleAudit}
                    onReindex={handleReindex}
                    onSync={handleSyncGitHub}
                  />
                )}
                {activeTab === "profile" && <ProfileTab profile={selectedProfile} />}
                {activeTab === "audit" && (
                  <AuditTab
                    snapshot={snapshot}
                    findings={findings}
                    auditing={auditing}
                    onAudit={handleAudit}
                  />
                )}
                {activeTab === "fixes" && (
                  <FixesTab
                    fixPlans={fixPlans}
                    aiProviders={aiProviders}
                    selectedProviderId={selectedProviderId}
                    onSelectProvider={setSelectedProviderId}
                    generatingPlan={generatingPlan}
                    onGeneratePlan={handleGenerateFixPlan}
                    proposingFix={proposingFix}
                    onProposeFix={handleProposeFix}
                    fixInstruction={fixInstruction}
                    onFixInstructionChange={setFixInstruction}
                    fixTargetFile={fixTargetFile}
                    onFixTargetFileChange={setFixTargetFile}
                    hasSnapshot={!!snapshot}
                    contextPreview={contextPreview}
                    previewingContext={previewingContext}
                    onPreviewContext={handlePreviewContext}
                  />
                )}
                {activeTab === "github" && (
                  <GitHubTab
                    integration={integration}
                    evidence={githubEvidence}
                    syncing={syncing}
                    onSync={handleSyncGitHub}
                  />
                )}
                {activeTab === "verify" && (
                  <VerifyTab
                    commands={commands}
                    runningCmd={runningCmd}
                    result={verifyResult}
                    onRun={handleRunVerification}
                    runs={verifyRuns}
                    batchRunning={batchRunning}
                    onBatchRun={handleBatchVerification}
                  />
                )}
                {activeTab === "patches" && (
                  <PatchesTab
                    patches={patches}
                    onApply={handleApplyPatch}
                    onReject={handleRejectPatch}
                    onRollback={handleRollbackPatch}
                  />
                )}
              </div>
            </div>
          )}
        </div>
      )}
      {approvalAction && approvalRequests.length > 0 && (
        <ApprovalModal
          requests={approvalRequests}
          busy={approvalBusy}
          onApprove={approvePendingAction}
          onDeny={denyPendingApproval}
        />
      )}
    </div>
  );
}

// --- Overview Tab ---

function OverviewTab({ profile, snapshot, findings, verifyResult, reindexing, onAudit, onReindex, onSync }: {
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

function ProfileTab({ profile }: { profile: RepoProfile | null }) {
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

function AuditTab({ snapshot, findings, auditing, onAudit }: {
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

// --- Fixes Tab ---

function FixesTab({ fixPlans, aiProviders, selectedProviderId, onSelectProvider,
  generatingPlan, onGeneratePlan, proposingFix, onProposeFix,
  fixInstruction, onFixInstructionChange, fixTargetFile, onFixTargetFileChange,
  hasSnapshot, contextPreview, previewingContext, onPreviewContext }: {
  fixPlans: Artifact[]; aiProviders: AiProvider[]; selectedProviderId: string;
  onSelectProvider: (id: string) => void; generatingPlan: boolean;
  onGeneratePlan: () => void; proposingFix: boolean; onProposeFix: () => void;
  fixInstruction: string; onFixInstructionChange: (v: string) => void;
  fixTargetFile: string; onFixTargetFileChange: (v: string) => void;
  hasSnapshot: boolean;
  contextPreview: ContextPreview | null;
  previewingContext: boolean;
  onPreviewContext: () => void;
}) {
  return (
    <div>
      <div style={{ marginBottom: 16 }}>
        <h3 style={{ fontSize: 14, fontWeight: 600, color: "var(--text-primary)", marginBottom: 8 }}>
          <Wand2 size={14} style={{ verticalAlign: "middle", marginRight: 6 }} />
          AI Fix Assistant
        </h3>

        {/* Provider selector */}
        {/* Provider selector */}
        <div style={{ marginBottom: 12 }}>
          <label htmlFor="af-ai-provider" style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>
            AI Provider
          </label>
          {aiProviders.length === 0 ? (
            <p style={{ fontSize: 11, color: "var(--text-secondary)", fontStyle: "italic" }}>
              No AI providers configured. Add API keys in Settings.
            </p>
          ) : (
            <select
              id="af-ai-provider"
              value={selectedProviderId}
              onChange={(e) => onSelectProvider(e.target.value)}
              style={{ fontSize: 12, padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", width: "100%", background: "var(--bg-input)", color: "var(--text-primary)" }}
            >
              <option value="">Select provider...</option>
              {aiProviders.map((p) => (
                <option key={p.id} value={p.id}>{p.name} ({p.adapterType})</option>
              ))}
            </select>
          )}
        </div>

        {/* Generate Fix Plan */}
        <div style={{ marginBottom: 12, padding: 12, background: "var(--bg-app)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
          <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)", marginBottom: 6 }}>Generate Fix Plan</h4>
          <p style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>
            Analyze audit findings and generate a prioritized fix plan using AI.
          </p>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <button
              onClick={onGeneratePlan}
              disabled={!selectedProviderId || !hasSnapshot || generatingPlan}
              style={{
                display: "flex", alignItems: "center", gap: 4, padding: "6px 12px",
                background: (!selectedProviderId || !hasSnapshot || generatingPlan) ? "var(--border-color)" : "var(--color-accent)",
                color: (!selectedProviderId || !hasSnapshot || generatingPlan) ? "var(--text-secondary)" : "#fff",
                border: "none", borderRadius: 4, cursor: (!selectedProviderId || !hasSnapshot || generatingPlan) ? "not-allowed" : "pointer",
                fontSize: 12, fontWeight: 600,
              }}
            >
              <Wand2 size={12} />
              {generatingPlan ? "Generating..." : "Generate Fix Plan"}
            </button>
            <button
              onClick={onPreviewContext}
              disabled={!hasSnapshot || previewingContext}
              style={{
                display: "flex", alignItems: "center", gap: 4, padding: "6px 12px",
                background: (!hasSnapshot || previewingContext) ? "var(--border-color)" : "var(--color-primary)",
                color: (!hasSnapshot || previewingContext) ? "var(--text-secondary)" : "#fff",
                border: "none", borderRadius: 4, cursor: (!hasSnapshot || previewingContext) ? "not-allowed" : "pointer",
                fontSize: 12, fontWeight: 600,
              }}
            >
              <Eye size={12} />
              {previewingContext ? "Loading..." : "Preview Context"}
            </button>
          </div>
          {!hasSnapshot && (
            <p style={{ fontSize: 10, color: "var(--color-warning-text)", marginTop: 4 }}>Run a health audit first to enable fix plan generation.</p>
          )}
        </div>

        {/* Context Preview */}
        {contextPreview && (
          <div style={{ marginBottom: 12, padding: 12, background: "var(--color-info-bg)", borderRadius: 6, border: "1px solid var(--color-info-border)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 8 }}>
              <Eye size={14} color="var(--color-info-text)" />
              <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--color-info-text)", margin: 0 }}>Context Preview</h4>
            </div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 6 }}>
              <strong>Purpose:</strong> {contextPreview.purpose}
            </div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 6 }}>
              <strong>Tokens:</strong> ~{contextPreview.totalTokensEstimate} / {contextPreview.maxTokens} max
            </div>
            {contextPreview.secretsFound.length > 0 && (
              <div style={{ display: "flex", alignItems: "center", gap: 4, marginBottom: 6, padding: "4px 8px", background: "var(--color-warning-bg)", borderRadius: 4, border: "1px solid var(--color-warning-border)" }}>
                <AlertTriangle size={12} color="var(--color-warning-text)" />
                <span style={{ fontSize: 11, color: "var(--color-warning-text)", fontWeight: 600 }}>
                  {contextPreview.secretsFound.length} secret(s) detected — will be redacted before sending to AI
                </span>
              </div>
            )}
            {contextPreview.secretCountAfterRedaction > 0 && (
              <div style={{ display: "flex", alignItems: "center", gap: 4, marginBottom: 6, padding: "4px 8px", background: "var(--color-danger-bg)", borderRadius: 4, border: "1px solid var(--color-danger-border)" }}>
                <AlertTriangle size={12} color="var(--color-danger-text)" />
                <span style={{ fontSize: 11, color: "var(--color-danger-text)", fontWeight: 600 }}>
                  {contextPreview.secretCountAfterRedaction} secret(s) remain after redaction — review before proceeding
                </span>
              </div>
            )}
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4, fontWeight: 600 }}>Sections:</div>
            {contextPreview.sections.map((s, i) => (
              <div key={i} style={{ padding: "6px 8px", marginBottom: 4, background: "var(--bg-input)", borderRadius: 4, border: "1px solid var(--border-color)" }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 2 }}>
                  <span style={{ fontSize: 11, fontWeight: 600, color: "var(--text-primary)" }}>{s.label}</span>
                  <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>~{s.tokensEstimate} tokens</span>
                </div>
                <div style={{ fontSize: 10, color: "var(--text-secondary)", marginBottom: 2 }}>Source: {s.source}</div>
                <pre style={{ fontSize: 10, color: "var(--text-secondary)", background: "var(--bg-app)", padding: 4, borderRadius: 3, overflow: "auto", maxHeight: 80, whiteSpace: "pre-wrap", margin: 0 }}>
                  {s.contentPreview}
                </pre>
              </div>
            ))}
            {contextPreview.promptPreview && (
              <details style={{ marginTop: 8 }}>
                <summary style={{ fontSize: 11, color: "var(--color-primary)", cursor: "pointer", fontWeight: 600 }}>Full Prompt Preview</summary>
                <pre style={{ fontSize: 10, color: "var(--text-secondary)", background: "var(--bg-input)", padding: 8, borderRadius: 4, overflow: "auto", maxHeight: 300, whiteSpace: "pre-wrap", marginTop: 4 }}>
                  {contextPreview.promptPreview}
                </pre>
              </details>
            )}
          </div>
        )}

        {/* Propose Fix */}
        <div style={{ marginBottom: 12, padding: 12, background: "var(--bg-app)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
          <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)", marginBottom: 6 }}>Propose a Fix</h4>
          <div style={{ marginBottom: 8 }}>
            <label htmlFor="af-fix-instruction" style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>
              Instruction
            </label>
            <textarea
              id="af-fix-instruction"
              value={fixInstruction}
              onChange={(e) => onFixInstructionChange(e.target.value)}
              placeholder="Describe what you want to fix (e.g., 'Fix the missing return type in src/main.ts')"
              rows={3}
              style={{ fontSize: 12, padding: "6px 8px", borderRadius: 4, border: "1px solid var(--border-color)", width: "100%", resize: "vertical", fontFamily: "inherit", background: "var(--bg-input)", color: "var(--text-primary)" }}
            />
          </div>
          <div style={{ marginBottom: 8 }}>
            <label htmlFor="af-fix-target" style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>
              Target File (optional)
            </label>
            <input
              id="af-fix-target"
              value={fixTargetFile}
              onChange={(e) => onFixTargetFileChange(e.target.value)}
              placeholder="e.g., src/main.ts"
              style={{ fontSize: 12, padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", width: "100%", background: "var(--bg-input)", color: "var(--text-primary)" }}
            />
          </div>
          <button
            onClick={onProposeFix}
            disabled={!selectedProviderId || !fixInstruction.trim() || proposingFix}
            style={{
              display: "flex", alignItems: "center", gap: 4, padding: "6px 12px",
              background: (!selectedProviderId || !fixInstruction.trim() || proposingFix) ? "var(--border-color)" : "var(--color-accent)",
              color: (!selectedProviderId || !fixInstruction.trim() || proposingFix) ? "var(--text-secondary)" : "#fff",
              border: "none", borderRadius: 4, cursor: (!selectedProviderId || !fixInstruction.trim() || proposingFix) ? "not-allowed" : "pointer",
              fontSize: 12, fontWeight: 600,
            }}
          >
            <Wand2 size={12} />
            {proposingFix ? "Proposing..." : "Propose Fix"}
          </button>
        </div>
      </div>

      {/* Fix Plans list */}
      <div>
        <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)", marginBottom: 8 }}>Fix Plans</h4>
        {fixPlans.length === 0 ? (
          <EmptyText>No fix plans generated yet.</EmptyText>
        ) : (
          fixPlans.map((fp) => (
            <div key={fp.id} style={{ padding: 10, marginBottom: 8, background: "var(--bg-input)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                <Wand2 size={12} color="var(--color-accent)" />
                <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{fp.artifactType}: {fp.title}</span>
              </div>
              {fp.content && (
                <pre style={{ fontSize: 11, color: "var(--text-secondary)", background: "var(--bg-app)", padding: 8, borderRadius: 4, overflow: "auto", maxHeight: 200, whiteSpace: "pre-wrap" }}>
                  {typeof fp.content === "string" ? fp.content : JSON.stringify(fp.content, null, 2)}
                </pre>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

// --- GitHub Tab ---

function GitHubTab({ integration, evidence, syncing, onSync }: {
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

function VerifyTab({ commands, runningCmd, result, onRun, runs, batchRunning, onBatchRun }: {
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

function PatchesTab({ patches, onApply, onReject, onRollback }: {
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

// --- Shared Components ---

function SortableHeader({ label, sortKey: key, currentKey, dir, onSort }: {
  label: string; sortKey: SortKey; currentKey: SortKey; dir: SortDir; onSort: (k: SortKey) => void;
}) {
  const active = currentKey === key;
  return (
    <th
      style={{ textAlign: "left", padding: "10px 12px", color: active ? "var(--color-primary)" : "var(--text-secondary)", fontWeight: 600, cursor: "pointer", userSelect: "none" }}
      onClick={() => onSort(key)}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 2 }}>
        {label}
        <ArrowUpDown size={11} style={{ opacity: active ? 1 : 0.3, transform: active && dir === "desc" ? "scaleY(-1)" : "none" }} />
      </div>
    </th>
  );
}

function ProfileSection({ icon: Icon, title, color, children }: { icon: any; title: string; color: string; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: 16 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
        <Icon size={14} color={color} />
        <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{title}</span>
      </div>
      <div style={{ paddingLeft: 20 }}>{children}</div>
    </div>
  );
}

function Tag({ label, color }: { label: string; color: string }) {
  return (
    <span style={{ padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600, background: color + "18", color, border: `1px solid ${color}33` }}>
      {label}
    </span>
  );
}

function MiniBadge({ label, active }: { label: string; active: boolean }) {
  return (
    <span className={`badge ${active ? "badge-success" : "badge-neutral"}`}>
      {active ? `✓ ${label}` : `✗ ${label}`}
    </span>
  );
}

function SeverityBadge({ severity }: { severity: string }) {
  let badgeClass = "badge-neutral";
  if (severity === "critical" || severity === "high") {
    badgeClass = "badge-danger";
  } else if (severity === "medium") {
    badgeClass = "badge-warning";
  } else if (severity === "low") {
    badgeClass = "badge-info";
  }
  return (
    <span className={`badge ${badgeClass}`} style={{ textTransform: "lowercase" }}>
      {severity}
    </span>
  );
}

function PatchStatusBadge({ status }: { status: string }) {
  let badgeClass = "badge-neutral";
  if (status === "proposed") {
    badgeClass = "badge-info";
  } else if (status === "applied") {
    badgeClass = "badge-success";
  } else if (status === "rejected") {
    badgeClass = "badge-danger";
  } else if (status === "rolled_back") {
    badgeClass = "badge-warning";
  }
  return (
    <span className={`badge ${badgeClass}`} style={{ marginLeft: 8, textTransform: "lowercase" }}>
      {status}
    </span>
  );
}

function EmptyText({ children }: { children: React.ReactNode }) {
  return <p style={{ fontSize: 11, color: "var(--text-muted)", fontStyle: "italic" }}>{children}</p>;
}

function repoName(path: string) {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}
