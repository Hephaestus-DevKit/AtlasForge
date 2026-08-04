# AtlasForge Capability Matrix

This file is the current implementation source of truth. Planning documents describe intent;
this matrix describes what the application actually does.

| Capability | Status | Current behavior | Evidence / gate | Next dependency |
| --- | --- | --- | --- | --- |
| Source control baseline | Implemented | `main` is synchronized to the public GitHub repository; Windows CI and Pages are required release evidence. | `git status`, GitHub Actions | Protect the branch when contributor workflow requires it. |
| Job lifecycle | Implemented with bounded scope | Validated transitions cover pending, running, completed, failed, and cancelled states. Verification processes receive cancellation signals. Interrupted running jobs are marked failed on startup. | Rust job and verification tests | Tauri events can replace adaptive UI polling if log volume becomes high. |
| Job retry | Implemented with safety limits | Scan, reindex, audit, and GitHub read-sync jobs can be replayed by a background worker. Jobs requiring fresh prompts, secrets, or approvals must be restarted from their feature page. | Rust job tests | Add replay handlers only when inputs are complete and idempotent. |
| Verification execution | Implemented | Commands are detected from manifests, output is drained concurrently and bounded, and timeout/cancellation terminates the process tree on Windows. | Large-output and cancellation tests | Add non-Windows process-group coverage when those targets are supported. |
| Verification approval | Implemented | Repository-controlled commands require a single-use, expiring approval bound to repository path, expanded command, and relevant file hashes. npm lifecycle hooks are shown. | Permission and mock IPC tests | None for local verification. |
| AI patch application | Implemented | A clean Git worktree is required. Patches are verified in a temporary detached worktree; backup and recovery hashes are persisted before mutation, and baseline drift invalidates approval. | Isolated worktree and recovery tests | Multi-file patch transactions remain intentionally unsupported. |
| Patch rollback | Implemented | Rollback requires a context-bound single-use approval and is refused after later edits. Reverse patch or persisted backup restoration must match the baseline hash. | Hash-guarded rollback test | None for single-file text patches. |
| Permission audit | Implemented | Approval, denial, consumption, patch, tool, and verification actions are recorded. | Database and permission tests | A dedicated audit viewer is optional operability work. |
| GitHub reads | Implemented | Cached integration, workflow runs, pull requests, releases, and sync errors are available. | Rust tests, typecheck, build | Add pagination only if real repositories need it. |
| GitHub writes | Disabled | PR creation, release creation, and workflow reruns cannot be enabled through an environment variable. | Command boundary | Requires dedicated previews and approval UI. |
| Database location | Implemented | Data uses the Tauri app-data directory. The legacy home-directory database is migrated once. | Startup path | None. |
| Database durability | Implemented | Startup quick-check detects corruption. A consistent online backup is created before pending migrations and older backups are pruned. | Database tests | A restore UI is only valuable after real recovery demand. |
| Repository list loading | Implemented | Repository, profile, latest health score, and latest verification state are returned by one IPC query. | IPC tests and typecheck | None. |
| Workspace discovery | Implemented | Include/exclude globs are validated and enforced case-insensitively on Windows; successful scans mark missing assets unavailable without destroying history. | Scanner/security tests and migration 018 | File watching is optional at current scale. |
| Text indexing | Implemented | Central sensitive-path policy and root exclusions are enforced case-insensitively, text is redacted, stale files and FTS path terms are removed, unchanged files are skipped, and processing is bounded. | Indexer/security tests and migration 017 | File watching is optional at current scale. |
| Tool Broker registry | Implemented with measured scope | Only `fs.list`, `fs.read`, `git.status`, and `git.diff` are advertised. Filesystem output is filtered and redacted before it returns to a caller. | Tool Broker tests | Verification stays on its dedicated approval-aware command path. |
| Audit quality | Implemented with measured scope | Deterministic project checks include GitHub Actions minimum permissions and immutable Action pinning, with positive and negative fixtures. | Auditor tests | Add rules only with representative fixtures and acceptable false-positive behavior. |
| Frontend IPC and DOM tests | Implemented | Vitest uses Tauri mock IPC and jsdom interaction tests for repository summaries, approvals, verification, batch ordering, patches, focus, and keyboard dismissal. | `npm test` | Extend race coverage as feature state grows. |
| Packaging and updater | Deferred | Development verification is the active gate. Installer branding and updater work are outside the current phase. | `npm run verify-dev -- -E2E` | Revisit when feature scope is stable. |

## Deliberate Non-goals

- Autonomous GitHub mutations without a dedicated operation preview.
- Running repository code without explicit approval.
- Applying AI patches to dirty working trees.
- Multi-user cloud state, plugin marketplace, background Windows service, or vector search.
- Native ARM64 packaging before the x86_64 workflow is stable.
