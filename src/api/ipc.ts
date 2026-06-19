import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  WorkspaceRoot,
  Repository,
  Job,
  JobEvent,
  AddRootInput,
  ScanResult,
  RepoProfile,
  ToolInfo,
  ToolResult,
  SearchResult,
  IndexStats,
  HealthSnapshot,
  Finding,
  AiProvider,
  AiResponse,
  Artifact,
  FixPlan,
  PatchProposal,
  GhAuthStatus,
  GitHubIntegration,
  GitHubPR,
  GitHubRelease,
  VerificationCommand,
  VerificationResult,
  VerificationRun,
  AutomationRule,
  Notification,
  ScanErrorRecord,
  ContextPreview,
} from "../types";

// --- Greeting (test IPC) ---

export async function greet(name: string): Promise<string> {
  return invoke<string>("greet", { name });
}

// --- Workspace Root commands ---

export async function listWorkspaceRoots(): Promise<WorkspaceRoot[]> {
  return invoke<WorkspaceRoot[]>("list_workspace_roots");
}

export async function addWorkspaceRoot(input: AddRootInput): Promise<WorkspaceRoot> {
  return invoke<WorkspaceRoot>("add_workspace_root", { input });
}

export async function removeWorkspaceRoot(id: string): Promise<void> {
  return invoke("remove_workspace_root", { id });
}

export async function updateWorkspaceRoot(
  id: string,
  updates: AddRootInput,
): Promise<WorkspaceRoot> {
  return invoke<WorkspaceRoot>("update_workspace_root", { id, updates });
}

// --- Repository commands ---

export async function listRepositories(): Promise<Repository[]> {
  return invoke<Repository[]>("list_repositories");
}

// --- Job/Scan commands ---

export async function startScan(rootIds?: string[]): Promise<ScanResult> {
  return invoke<ScanResult>("start_scan", { rootIds });
}

export async function listJobs(limit?: number): Promise<Job[]> {
  return invoke<Job[]>("list_jobs", { limit: limit ?? 50 });
}

export async function getJobEvents(jobId: string): Promise<JobEvent[]> {
  return invoke<JobEvent[]>("get_job_events", { jobId });
}

// --- Repo Profile commands ---

export async function getRepoProfile(repoId: string): Promise<RepoProfile | null> {
  return invoke<RepoProfile | null>("get_repo_profile", { repoId });
}

export async function refreshProfiles(): Promise<number> {
  return invoke<number>("refresh_profiles");
}

// --- Job Engine commands ---

export async function cancelJob(jobId: string): Promise<Job> {
  return invoke<Job>("cancel_job_cmd", { jobId });
}

export async function retryJob(jobId: string): Promise<Job> {
  return invoke<Job>("retry_job_cmd", { jobId });
}

export async function getJobDetail(jobId: string): Promise<Job> {
  return invoke<Job>("get_job_detail", { jobId });
}

// --- Tool Broker commands ---

export async function listTools(): Promise<ToolInfo[]> {
  return invoke<ToolInfo[]>("list_tools_cmd");
}

export async function invokeTool(
  jobId: string,
  toolName: string,
  input: any,
  dryRun?: boolean,
): Promise<ToolResult> {
  return invoke<ToolResult>("invoke_tool_cmd", { jobId, toolName, input, dryRun });
}

export async function listInvocations(jobId: string): Promise<any[]> {
  return invoke<any[]>("list_invocations_cmd", { jobId });
}

// --- Indexer commands ---

export async function searchIndex(query: string, limit?: number, repoId?: string): Promise<SearchResult[]> {
  return invoke<SearchResult[]>("search_index_cmd", { query, limit, repoId });
}

export async function listDocuments(repoId: string): Promise<any[]> {
  return invoke<any[]>("list_documents_cmd", { repoId });
}

export async function reindexRepo(repoId: string): Promise<IndexStats> {
  return invoke<IndexStats>("reindex_repo_cmd", { repoId });
}

// --- Auditor commands ---

export async function auditRepo(repoId: string): Promise<HealthSnapshot> {
  return invoke<HealthSnapshot>("audit_repo_cmd", { repoId });
}

export async function getHealthSnapshot(repoId: string): Promise<HealthSnapshot | null> {
  return invoke<HealthSnapshot | null>("get_health_snapshot_cmd", { repoId });
}

export async function getFindings(snapshotId: string): Promise<Finding[]> {
  return invoke<Finding[]>("get_findings_cmd", { snapshotId });
}

// --- AI Provider commands ---

export async function listAiProviders(): Promise<AiProvider[]> {
  return invoke<AiProvider[]>("list_ai_providers_cmd");
}

export async function detectLocalProviders(): Promise<AiProvider[]> {
  return invoke<AiProvider[]>("detect_local_providers_cmd");
}

export async function upsertAiProvider(provider: AiProvider): Promise<void> {
  return invoke("upsert_ai_provider_cmd", { provider });
}

export async function deleteAiProvider(id: string): Promise<void> {
  return invoke("delete_ai_provider_cmd", { id });
}

export async function callAi(
  providerId: string,
  prompt: string,
  model?: string,
): Promise<AiResponse> {
  return invoke<AiResponse>("call_ai_cmd", { providerId, prompt, model });
}

// --- AI Fix commands ---

export async function listArtifacts(jobId: string): Promise<Artifact[]> {
  return invoke<Artifact[]>("list_artifacts_cmd", { jobId });
}

export async function listPatchProposals(repoId: string): Promise<PatchProposal[]> {
  return invoke<PatchProposal[]>("list_patch_proposals_cmd", { repoId });
}

export async function applyPatch(proposalId: string): Promise<PatchProposal> {
  return invoke<PatchProposal>("apply_patch_cmd", { proposalId });
}

export async function rejectPatch(proposalId: string, reason: string): Promise<void> {
  return invoke("reject_patch_cmd", { proposalId, reason });
}

export async function rollbackPatch(proposalId: string): Promise<void> {
  return invoke("rollback_patch_cmd", { proposalId });
}

// --- AI Fix Plan commands ---

export async function generateFixPlan(
  repoId: string,
  snapshotId: string,
  providerId: string,
  model?: string,
): Promise<FixPlan> {
  return invoke<FixPlan>("generate_fix_plan_cmd", { repoId, snapshotId, providerId, model });
}

export async function proposeFix(
  repoId: string,
  providerId: string,
  fixInstruction: string,
  model?: string,
  targetFile?: string,
): Promise<PatchProposal> {
  return invoke<PatchProposal>("propose_fix_cmd", { repoId, providerId, model, fixInstruction, targetFile });
}

export async function listFixPlans(repoId: string): Promise<Artifact[]> {
  return invoke<Artifact[]>("list_fix_plans_cmd", { repoId });
}

export async function previewFixPlanContext(
  repoId: string,
  snapshotId: string,
): Promise<ContextPreview> {
  return invoke<ContextPreview>("preview_fix_plan_context_cmd", { repoId, snapshotId });
}

// --- GitHub commands ---

export async function checkGhAuth(): Promise<GhAuthStatus> {
  return invoke<GhAuthStatus>("check_gh_auth_cmd");
}

export async function resolveGitHubRepo(repoId: string): Promise<GitHubIntegration> {
  return invoke<GitHubIntegration>("resolve_github_repo_cmd", { repoId });
}

export async function syncGitHub(repoId: string): Promise<any> {
  return invoke("sync_github_cmd", { repoId });
}

export async function createPr(
  repoId: string,
  title: string,
  body: string,
  head: string,
  base: string,
  draft?: boolean,
): Promise<GitHubPR> {
  return invoke<GitHubPR>("create_pr_cmd", { repoId, title, body, head, base, draft });
}

export async function createRelease(
  repoId: string,
  tag: string,
  name: string,
  body: string,
  draft?: boolean,
  prerelease?: boolean,
): Promise<GitHubRelease> {
  return invoke<GitHubRelease>("create_release_cmd", { repoId, tag, name, body, draft, prerelease });
}

export async function rerunWorkflow(repoId: string, runId: string): Promise<void> {
  return invoke("rerun_workflow_cmd", { repoId, runId });
}

// --- Verification commands ---

export async function detectCommands(worktreePath: string): Promise<VerificationCommand[]> {
  return invoke<VerificationCommand[]>("detect_commands_cmd", { worktreePath });
}

export async function runVerification(
  command: string,
  cwd: string,
  repoId?: string,
): Promise<VerificationResult> {
  return invoke<VerificationResult>("run_verification_cmd", { command, cwd, repoId });
}

export async function runBatchVerification(
  commands: VerificationCommand[],
  cwd: string,
  repoId?: string,
): Promise<VerificationResult[]> {
  const commandNames = commands.map((command) => command.command);
  return invoke<VerificationResult[]>("run_batch_verification_cmd", { commandNames, cwd, repoId });
}

export async function listVerificationRuns(repoId: string, limit?: number): Promise<VerificationRun[]> {
  return invoke<VerificationRun[]>("list_verification_runs_cmd", { repoId, limit });
}
// --- Automation commands ---

export async function listAutomationRules(): Promise<AutomationRule[]> {
  return invoke<AutomationRule[]>("list_automation_rules_cmd");
}

export async function createAutomationRule(rule: AutomationRule): Promise<void> {
  return invoke("create_automation_rule_cmd", { rule });
}

export async function updateAutomationRule(rule: AutomationRule): Promise<void> {
  return invoke("update_automation_rule_cmd", { rule });
}

export async function deleteAutomationRule(id: string): Promise<void> {
  return invoke("delete_automation_rule_cmd", { id });
}

export async function listNotifications(unreadOnly?: boolean, limit?: number): Promise<Notification[]> {
  return invoke<Notification[]>("list_notifications_cmd", { unreadOnly, limit });
}

export async function markNotificationRead(id: string): Promise<void> {
  return invoke("mark_notification_read_cmd", { id });
}

export async function markAllNotificationsRead(): Promise<number> {
  return invoke<number>("mark_all_notifications_read_cmd");
}

export async function tickScheduler(): Promise<string[]> {
  return invoke<string[]>("tick_scheduler_cmd");
}

// --- Scan Error commands ---

export async function listScanErrors(rootId: string): Promise<ScanErrorRecord[]> {
  return invoke<ScanErrorRecord[]>("list_scan_errors_cmd", { rootId });
}

// --- Dialog helpers ---

/**
 * Open a native folder picker dialog.
 * Returns the selected folder path, or null if the user cancelled.
 */
export async function pickFolder(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select Workspace Root",
  });
  return selected as string | null;
}
