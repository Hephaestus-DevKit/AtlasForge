// Shared types for IPC between frontend and backend
// These must match the Rust struct field names (snake_case → camelCase via serde default)

export type AccessMode = "read_only" | "read_write";

export interface WorkspaceRoot {
  id: string;
  path: string;
  label: string;
  accessMode: AccessMode;
  scanEnabled: boolean;
  includeGlobs: string[];
  excludeGlobs: string[];
  createdAt: string;
  lastScannedAt: string | null;
}

export interface ProjectAsset {
  id: string;
  rootId: string;
  path: string;
  kind: string;
  name: string;
  primaryLanguage: string | null;
  lastObservedAt: string;
}

export interface Repository {
  id: string;
  assetId: string;
  worktreePath: string;
  gitDirPath: string;
  isBare: boolean;
  isWorktree: boolean;
  defaultBranch: string | null;
  currentBranch: string | null;
  headSha: string | null;
  remoteOriginUrl: string | null;
  dirtyState: boolean;
  aheadBehind: { ahead: number; behind: number } | null;
  lastCommitAt: string | null;
}

export interface RepositorySummary {
  repository: Repository;
  profile: RepoProfile | null;
  healthScore: number | null;
  lastVerificationSuccess: boolean | null;
}

export type JobStatus = "pending" | "running" | "completed" | "failed" | "cancelled";

export interface Job {
  id: string;
  type: string;
  status: string;
  input: string;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
  errorMessage: string | null;
  progress: number;
  progressTotal: number;
  parentJobId: string | null;
}

export type JobEventType =
  | "job_created"
  | "job_started"
  | "job_cancelled"
  | "job_failed"
  | "job_completed"
  | "scan_started"
  | "scan_root_started"
  | "scan_repo_discovered"
  | "scan_repo_profiled"
  | "scan_repo_profile_failed"
  | "scan_repo_completed"
  | "scan_root_completed"
  | "scan_root_scanned"
  | "scan_completed"
  | "root_scanned"
  | "root_scan_error"
  | "root_skipped"
  | "scan_summary"
  | "audit_started"
  | "audit_completed"
  | "audit_failed"
  | "reindex_started"
  | "reindex_completed"
  | "reindex_failed"
  | "verification_started"
  | "verification_completed"
  | "verification_failed"
  | "github_sync_started"
  | "github_sync_completed"
  | "github_sync_failed"
  | "ai_call_started"
  | "ai_call_completed"
  | "ai_call_failed"
  | "patch_apply_started"
  | "patch_apply_completed"
  | "patch_apply_failed";

export interface JobEvent {
  id: string;
  jobId: string;
  seq: number;
  type: string;
  payload: string;
  createdAt: string;
}

export interface AuditEntry {
  id: string;
  action: string;
  subject: string;
  scope: string;
  capability: string;
  riskLevel: string;
  detail: string;
  createdAt: string;
}

export interface AddRootInput {
  path: string;
  label: string;
  accessMode: AccessMode;
  scanEnabled: boolean;
  includeGlobs: string[];
  excludeGlobs: string[];
}

export interface ScanResult {
  jobId: string;
  reposDiscovered: number;
  errors: string[];
}

export interface RepoProfile {
  id: string;
  repoId: string;
  languages: string[];
  frameworks: string[];
  packageManagers: string[];
  scripts: Record<string, string>;
  ciSystems: string[];
  hasReadme: boolean;
  hasLicense: boolean;
  licenseType: string | null;
  detectedAt: string;
}

// --- Tool Broker types ---

export interface ToolInfo {
  id: string;
  name: string;
  category: string;
  description: string;
  riskLevel: string;
  requiresPermission: boolean;
  dryRunSupported: boolean;
}

export interface ToolResult {
  success: boolean;
  output: string;
  error: string | null;
  wasDryRun: boolean;
}

// --- Indexer types (matches Rust indexer::SearchResult) ---

export interface SearchResult {
  chunkId: string;
  content: string;
  heading: string | null;
  startLine: number | null;
  endLine: number | null;
  chunkType: string;
  path: string;
  repoId: string;
  rank: number;
}

export interface IndexStats {
  documents: number;
  chunks: number;
  indexedDocuments: number;
  skippedDocuments: number;
  errors: string[];
}

export interface IndexedDocument {
  id: string;
  path: string;
  mimeType: string;
  language: string | null;
  sizeBytes: number;
  indexedAt: string;
}

export interface ToolInvocation {
  id: string;
  toolName: string;
  input: string;
  output: string | null;
  status: string;
  riskLevel: string;
  permissionDecision: string | null;
  errorMessage: string | null;
  createdAt: string;
  completedAt: string | null;
}

// --- Auditor types (matches Rust auditor structs) ---

export interface CategoryScore {
  score: number;
  maxScore: number;
  weight: number;
  findings: Finding[];
}
export interface RecommendedTask {
  title: string;
  category: string;
  priority: string;
  description: string;
  severity: string;
  autoFixable: boolean;
}

export interface HealthSnapshot {
  id: string;
  repoId: string;
  scanId: string | null;
  score: number;
  categoryScores: string; // JSON string of Record<string, CategoryScore>
  recommendedTasks: string; // JSON string of RecommendedTask[]
  createdAt: string;
}

export interface Finding {
  id: string;
  category: string;
  severity: string;
  title: string;
  description: string;
  evidence: string;
  filePath: string | null;
  suggestedFix: string | null;
  autoFixable: boolean;
}

// --- AI Provider types (matches Rust ai_provider::AiProvider) ---

export interface AiProvider {
  id: string;
  name: string;
  adapterType: "ollama" | "openai_compatible" | "anthropic" | "custom";
  baseUrl: string;
  apiKeyRef: string | null;
  defaultModel: string;
  availableModels: string[];
  isLocal: boolean;
  isDefault: boolean;
  enabled: boolean;
  config: Record<string, unknown>;
}

export interface AiResponse {
  content: string;
  model: string;
  tokensIn: number;
  tokensOut: number;
  finishReason: string | null;
}

export interface ProviderProbe {
  reachable: boolean;
  message: string;
  latencyMs: number;
  models: string[];
}

// --- AI Fix types (matches Rust ai_fix structs) ---

export interface Artifact {
  id: string;
  jobId: string;
  artifactType: string;
  title: string;
  content: string;
  filePath: string | null;
  metadata: Record<string, unknown>;
}

export interface FixPlan {
  id: string;
  jobId: string;
  repoId: string;
  snapshotId: string;
  providerId: string;
  model: string;
  planContent: string;
  contextSummary: string;
  tokensIn: number;
  tokensOut: number;
  createdAt: string;
}

export interface ContextPreviewSection {
  label: string;
  source: string;
  tokensEstimate: number;
  contentPreview: string;
}

export interface SecretMatch {
  label: string;
  position: number;
  preview: string;
}

export interface ContextPreview {
  purpose: string;
  sections: ContextPreviewSection[];
  totalTokensEstimate: number;
  maxTokens: number;
  promptPreview: string;
  secretsFound: SecretMatch[];
  secretCountAfterRedaction: number;
}

export interface PatchProposal {
  id: string;
  jobId: string;
  artifactId: string | null;
  repoId: string;
  filePath: string;
  patchContent: string;
  description: string;
  status: string;
  appliedAt: string | null;
  rolledBackAt: string | null;
  verificationResult: string | null;
}

// --- GitHub types (matches Rust github structs) ---

export interface GhAuthStatus {
  authenticated: boolean;
  username: string | null;
  message: string;
}

export interface GitHubIntegration {
  id: string;
  repoId: string;
  githubOwner: string;
  githubRepo: string;
  isFork: boolean;
  defaultBranch: string | null;
  visibility: string | null;
  lastSyncedAt: string | null;
}

export interface GitHubPR {
  id: string;
  integrationId: string;
  prNumber: number;
  title: string;
  state: string;
  author: string | null;
  branch: string | null;
  url: string | null;
}

export interface GitHubRelease {
  id: string;
  integrationId: string;
  releaseId: string;
  tagName: string;
  name: string | null;
  isDraft: boolean;
  isPrerelease: boolean;
  publishedAt: string | null;
  url: string | null;
}

export interface GitHubWorkflowRun {
  id: string;
  integrationId: string;
  runId: string;
  workflowName: string;
  branch: string | null;
  status: string;
  conclusion: string | null;
  triggeredAt: string | null;
  completedAt: string | null;
  url: string | null;
}

export interface GitHubEvidence {
  workflowRuns: GitHubWorkflowRun[];
  pullRequests: GitHubPR[];
  releases: GitHubRelease[];
  syncErrors: string[];
}

export interface GitHubSyncResult {
  workflows: number;
  prs: number;
  releases: number;
}

// --- Verification types (matches Rust verification structs) ---

export interface VerificationCommand {
  name: string;
  command: string;
  timeoutSecs: number;
  category: string;
  riskLevel: string;
  requiresApproval: boolean;
  expandedCommand: string;
  riskExplanation: string;
}

export interface PermissionRequest {
  id: string;
  jobId: string | null;
  repoId: string | null;
  capability: string;
  scope: string;
  riskLevel: string;
  command: string | null;
  contextHash: string;
  details: Record<string, unknown>;
  status: "pending" | "approved" | "denied" | "consumed" | "expired";
  createdAt: string;
  expiresAt: string;
  decidedAt: string | null;
}

export interface VerificationResult {
  success: boolean;
  command: string;
  exitCode: number | null;
  stdout: string;
  stderr: string;
  durationMs: number;
  timedOut: boolean;
}

export interface VerificationRun {
  id: string;
  repoId: string;
  jobId: string | null;
  command: string;
  cwd: string;
  category: string;
  riskLevel: string;
  success: boolean;
  exitCode: number | null;
  durationMs: number;
  timedOut: boolean;
  stdoutTail: string;
  stderrTail: string;
  createdAt: string;
}

// --- Automation types (matches Rust automations structs) ---

export interface AutomationRule {
  id: string;
  name: string;
  description: string;
  triggerType: "schedule" | "ci_failure" | "new_commit" | "drift_detected" | "manual";
  triggerConfig: { intervalMinutes?: number };
  actionType: "scan" | "audit" | "fix" | "notify" | "github_sync";
  actionConfig: Record<string, unknown>;
  targetRepoIds: string[];
  targetRootIds: string[];
  maxRiskLevel: "none" | "low" | "medium" | "high" | "critical";
  autoApply: boolean;
  enabled: boolean;
  lastTriggeredAt: string | null;
  lastRunJobId: string | null;
  runCount: number;
}

export interface Notification {
  id: string;
  ruleId: string | null;
  jobId: string | null;
  notificationType: "info" | "warning" | "error" | "success";
  title: string;
  message: string;
  read: boolean;
  actionUrl: string | null;
  createdAt: string;
}

export interface ScanErrorRecord {
  id: string;
  rootId: string;
  path: string | null;
  errorType: string;
  message: string;
  createdAt: string;
}
