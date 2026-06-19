# Alpha Productization Plan

This document is the execution plan for turning the current AtlasForge Alpha 0 baseline into a usable local-first engineering workbench.

It is intentionally organized by dependency and acceptance, not by calendar time. AtomCode can execute the work package by package. A reviewer should be able to audit every package from the validation evidence.

## Current Baseline

AtlasForge currently has:

- React + Vite + TypeScript frontend.
- Tauri v2 Rust backend.
- SQLite migrations and local database under the user's home directory.
- IPC coverage for workspace roots, repository scanning, profiling, audits, indexing, verification, AI providers, patch proposals, GitHub integration, automations, notifications, and jobs.
- A basic UI for Dashboard, Assets, Repositories, Tasks, Knowledge, Automations, and Settings.
- Windows x64 build passing with generated exe, MSI, and NSIS installer.

Validated gates:

```powershell
npm run typecheck
npm run lint
npm test
npm run build
npm audit --omit=optional
```

Windows x64 Rust/Tauri gates:

```powershell
cmd /c "call ""C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"" && set PATH=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;%USERPROFILE%\.cargo\bin;%PATH% && cargo +stable-x86_64-pc-windows-msvc check --target x86_64-pc-windows-msvc"
```

```powershell
cmd /c "call ""C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"" && set ""RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc"" && set PATH=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;%USERPROFILE%\.cargo\bin;%PATH% && npm run tauri -- build --target x86_64-pc-windows-msvc"
```

## Product Standard

AtlasForge is not complete when it merely compiles. It is complete enough for alpha use when this loop works without manual database edits or hidden commands:

1. Add an authorized workspace root.
2. Scan for repositories.
3. Open a repository and see a credible profile.
4. Run a health audit and inspect findings with evidence.
5. Detect runnable verification commands.
6. Run verification and preserve the result as job evidence.
7. Search indexed repository content without exposing secret files.
8. Ask AI for a bounded report or fix plan using only reviewed context.
9. Review a patch proposal before applying it.
10. Re-run verification after patch application.
11. Roll back or mark failure with clear evidence.
12. Build AtlasForge itself into x64 artifacts.

## Non-Goals For This Pass

Do not spend this pass on:

- Native ARM64 build. Windows x64 remains the active baseline.
- Fully autonomous GitHub push/release.
- Background daemon/service mode.
- Cloud sync.
- Multi-user accounts.
- A marketing site.
- Semantic/vector index as a prerequisite for the core loop.
- Large UI redesign that delays the core product flow.

These can be planned later once the alpha loop is working.

## Cross-Cutting Rules

- Do not weaken safety gates to make tests pass.
- Do not write real secrets into files, docs, fixtures, tests, logs, or screenshots.
- Do not let frontend code directly execute shell, mutate files, or call GitHub write operations.
- All write-like operations must go through backend commands, path authorization, risk labeling, and audit records.
- Keep GitHub write commands blocked by `ATLASFORGE_ENABLE_GITHUB_WRITE=1` until permission review is implemented.
- Keep x64 as the build target unless the user explicitly resumes ARM64 work.
- Prefer small, reviewable packages with focused tests.
- Update docs when behavior changes.

## Completion Waves

Each wave has a product purpose, implementation tasks, acceptance criteria, and reviewer notes.

### Wave 1: First-Run Workspace Flow

Purpose:
Make first launch usable without knowing internal paths or commands.

Tasks:

- Add a native folder picker for workspace root selection.
- Install and configure the Tauri dialog plugin if needed.
- Add required Tauri v2 permissions/capabilities for dialog use.
- Keep manual path entry as a fallback.
- Validate path existence, directory status, duplicate roots, and access mode before save.
- Show clear errors for invalid paths, duplicate roots, denied paths, and scan failures.
- Add a first-run empty state that guides the user directly to adding a root.
- Add "Scan selected roots" and "Scan all enabled roots" entry points from Assets and Dashboard.

Acceptance:

- A new user can add a root from the UI without typing a path.
- Adding the same root twice produces a clear error.
- A root can be read-only or read-write.
- Removing a root removes associated assets/repositories without leaving a broken UI.
- `npm run typecheck`, `npm run lint`, `npm test`, and `npm run build` pass.

Reviewer focus:

- Confirm new plugin permissions are explicit.
- Confirm no broad filesystem permission was added accidentally.
- Confirm frontend still cannot bypass backend path authorization.

### Wave 2: Repository Scan Reliability

Purpose:
Make repository discovery trustworthy on real local directories.

Tasks:

- Preserve stable repository IDs across scans. Do not create a new repo ID every time the same worktree is scanned.
- Preserve stable project asset IDs across scans.
- Avoid deleting and reinserting repository rows unless necessary.
- Handle nested repos without duplicate discovery.
- Handle `.git` directories and `.git` worktree files correctly.
- Add scan error records that can be shown in the UI.
- Add scan summary events with root ID, repo count, skipped count, and errors.
- Avoid walking heavy ignored directories such as `node_modules`, `.git`, `target`, `dist`, `.next`, `.cache`.
- Add backend tests or fixture-based tests for clean repo, dirty repo, nested repo, worktree-style `.git` file, and no-remote repo.

Acceptance:

- Running scan twice does not duplicate repositories.
- Existing repo details update in place.
- Dirty state, branch, remote, head SHA, ahead/behind, and last commit are visible.
- Scan results survive app restart.
- At least one automated test covers stable repo identity across rescans.

Reviewer focus:

- Check database uniqueness constraints and upsert behavior.
- Check Windows path normalization.
- Check scan cannot escape authorized roots.

### Wave 3: Durable Job Evidence

Purpose:
Turn scan, audit, index, verification, AI, and patch operations into auditable jobs.

Tasks:

- Normalize job status transitions: pending, running, completed, failed, cancelled.
- Add explicit job error message and progress fields to frontend types if schema already supports them.
- Ensure every long-running command creates job events with ordered sequence numbers.
- Ensure failed operations write job events and audit entries.
- Convert scan, audit, reindex, verification, GitHub sync, AI call, and patch apply into jobs or job-linked events.
- Improve Tasks page event rendering: parse JSON payloads, show timestamps, status, target, and error summary.
- Add a job detail drawer/panel with events, artifacts, tool invocations, and verification results.
- Ensure retry creates a new job linked to the original.

Acceptance:

- Starting a scan creates a job and useful event timeline.
- Running an audit creates or links to a job.
- Running verification creates or links to a job and stores stdout/stderr summary.
- Failed jobs include an actionable error message.
- Tasks page can be refreshed without losing history.

Reviewer focus:

- Check event sequence generation for races and duplicate sequence numbers.
- Check jobs never claim success when a backend operation failed.
- Check raw logs are truncated before storage if needed.

### Wave 4: Repository Detail Product Loop

Purpose:
Make Repositories the main operating surface.

Tasks:

- Add filters for dirty, clean, no remote, no CI, no README, high risk, language, root, and branch.
- Add sorting for path, branch, dirty, last commit, score, and language.
- Add a compact repository summary header with path, branch, dirty state, remote, profile, last audit score, and last verification result.
- Add tabs for Overview, Profile, Health, Verify, Files, Patches, GitHub, and Settings.
- Parse `HealthSnapshot.categoryScores` and `recommendedTasks` JSON into typed frontend structures.
- Show category scores with evidence, not just total score.
- Add actions: Run Audit, Reindex, Detect Commands, Run Selected Verification, Sync GitHub Read Data.
- Make actions disabled with clear reason when prerequisites are missing.
- Replace `alert()` calls with in-app toasts or status banners.

Acceptance:

- User can select a repo and complete profile -> audit -> verify from one place.
- Audit findings show severity, category, evidence, suggested fix, and file path.
- Verification result is visible after running a command.
- UI remains usable with long paths, long logs, and empty data.

Reviewer focus:

- Check frontend state does not hide backend errors.
- Check JSON parsing has fallback handling.
- Check table layout does not break at common desktop widths.

### Wave 5: Verification Engine v1

Purpose:
Make verification evidence reliable enough to trust audit and patch workflows.

Tasks:

- Store verification results in the database, linked to repo and job.
- Add schema if needed for verification runs.
- Detect commands from package manifests more accurately:
  - npm, pnpm, yarn, bun.
  - Cargo.
  - Python with project-specific scripts, but do not assume a Conda environment inside AtlasForge yet.
  - Go.
- Do not run install commands by default. Treat install commands as high friction and user-approved.
- Add command risk level and timeout.
- Add output truncation and tail summary.
- Add cancellation support where feasible.
- Add "run selected commands" rather than only one command at a time.
- Show command cwd, exit code, duration, timeout, stdout tail, stderr tail.

Acceptance:

- Verification run records are saved and visible after refresh.
- Failed command has clear stderr tail and exit code.
- Timeout kills the child process and marks the result timed out.
- AtlasForge's own detected commands can run successfully.

Reviewer focus:

- Check command execution still goes through authorized cwd.
- Check command strings are not concatenated from untrusted UI fields without validation.
- Check Windows command execution and Unix command execution both have explicit paths.

### Wave 6: Indexing and Knowledge v1

Purpose:
Make local project content searchable without leaking sensitive files.

Tasks:

- Add a visible Reindex action per repo and per root.
- Store index stats and latest index errors.
- Add document list UI under repository Files or Knowledge tab.
- Add global Knowledge search with repo filter and result preview.
- Improve exclude rules for secret and binary files:
  - `.env*`
  - key/cert files.
  - large generated files.
  - build artifacts.
  - dependency directories.
- Add a "why not indexed" or skipped count summary.
- Add tests for secret exclusion and FTS search.

Acceptance:

- Searching for text in README/package/config files returns path and snippet.
- `.env`, private keys, and binary assets are not indexed.
- Reindex can be run repeatedly without duplicate chunks.
- Search gracefully handles empty query and no results.

Reviewer focus:

- Check indexer deletes/replaces stale chunks correctly.
- Check secret exclusion uses path/file rules before reading content where possible.
- Check search output does not expose ignored files.

### Wave 7: Audit Quality v1

Purpose:
Make health reports credible enough to guide maintenance.

Tasks:

- Improve audit categories:
  - Runnable.
  - Tests.
  - CI.
  - Docs.
  - Dependencies.
  - Security.
  - Release readiness.
  - Git hygiene.
  - Public surface.
  - Platform compatibility.
- Make each finding include:
  - severity.
  - category.
  - title.
  - description.
  - evidence.
  - file path when applicable.
  - suggested fix.
  - auto-fixability.
- Avoid findings that are misleading for private/local repos.
- Add score calculation explanation to UI.
- Add fixtures and tests for representative repos.
- Generate recommended tasks from findings without requiring AI.

Acceptance:

- A real repo produces a useful report with no obvious hallucinated facts.
- Each high or critical finding has evidence.
- Report can be saved, reopened, and compared after re-audit.
- Audit is useful even without AI provider configured.

Reviewer focus:

- Check no network call is required for local audit.
- Check scores are deterministic for same input.
- Check findings do not overstate risk without evidence.

### Wave 8: AI Report, Then AI Fix

Purpose:
Introduce AI only after the non-AI evidence loop is strong.

Phase A: AI report and planning.

Tasks:

- Build a real ContextPack pipeline from profile, audit findings, selected files, and verification summaries.
- Redact or reject secrets before provider calls.
- Add UI for provider configuration:
  - Ollama local.
  - OpenAI-compatible endpoint.
  - API key environment variable name only, never raw key.
- Add "Generate Fix Plan" from audit findings.
- Store AI output as an artifact linked to a job.
- Treat malformed model output as failed/needs review.

Acceptance:

- With no provider, the app still works and explains how to configure one.
- With a provider, user can generate a fix plan for a repo.
- Prompt/context sent to AI can be previewed or summarized.
- Secret-like prompt content is rejected or redacted.

Phase B: patch proposal.

Tasks:

- Ask AI for unified diff only when user explicitly starts a fix task.
- Validate patch format before saving proposal.
- Save proposal as artifact and patch_proposal row.
- Show patch diff in UI with file path, description, risk, and status.
- Apply only after user approves.
- Apply through backend path authorization and git apply check.
- Run selected verification after applying.
- Support rollback using stored reverse/apply logic.

Acceptance:

- AI cannot apply patches directly.
- Patch outside authorized root is rejected.
- Patch apply failure marks conflict and preserves proposal.
- Applied patch can be rolled back.
- Verification result after apply is stored.

Reviewer focus:

- Check prompt injection from repository files cannot bypass tool broker.
- Check provider errors never leave partial writes.
- Check raw API keys are not persisted.

### Wave 9: GitHub Read, Then Controlled Write

Purpose:
Use GitHub as evidence first. Writes stay gated.

Phase A: read integration.

Tasks:

- Improve `gh auth status` parsing and error messages.
- Resolve GitHub remote robustly for HTTPS and SSH URLs.
- Store workflow runs, PRs, releases, and last sync status.
- Display recent workflow runs, open PRs, and releases in repo detail.
- Handle auth missing, repo missing, rate limit, no runs, and no releases.

Acceptance:

- GitHub tab shows readable status for repos with GitHub remote.
- Auth missing does not crash UI.
- Sync failures are visible and job-linked.

Phase B: write integration, still gated.

Tasks:

- Keep `ATLASFORGE_ENABLE_GITHUB_WRITE=1` gate.
- Add permission review UI before create PR, rerun workflow, create release.
- Show owner/repo, branch, tag, title, body, and risk before executing.
- Use dry-run preview first.
- After mutation, read back via `gh` or API and store evidence.

Acceptance:

- No GitHub mutation runs without explicit gate and UI approval.
- A successful mutation is verified by reading remote state back.
- A failed mutation never reports success.

Reviewer focus:

- Check branch/tag/repo target confirmation.
- Check local dirty state and remote mismatch warnings before PR/release.
- Check audit log for every mutation.

### Wave 10: Permission Review and Audit Surface

Purpose:
Make high-risk actions understandable and reviewable.

Tasks:

- Add permission review modal for high/critical tools.
- Show action type, target, command, cwd, risk, rollback option, and scope.
- Add approve once, approve for job, deny.
- Persist approval decision only for the intended scope.
- Add audit log page or panel.
- Redact sensitive values from tool inputs/outputs before display.
- Log denied actions too.

Acceptance:

- High-risk shell/GitHub/file-write action opens review instead of executing silently.
- Denied actions are recorded and visible.
- Audit log can be filtered by repo, job, action, risk, and result.

Reviewer focus:

- Check approval scope cannot leak across jobs/repos.
- Check redaction covers tokens, keys, env vars, and Authorization headers.
- Check default policy is conservative.

### Wave 11: UI Hardening and Smoke Tests

Purpose:
Make the app feel like a product instead of a prototype.

Tasks:

- Replace inline styles gradually with reusable layout and UI primitives if it reduces duplication.
- Keep design dense and operational, not marketing-style.
- Add stable dimensions for tables, side panels, buttons, and log viewers.
- Add toasts/status banners.
- Add loading and empty states for every page.
- Add error boundaries.
- Add keyboard-safe forms and accessible labels.
- Add Playwright or another browser smoke test path for web UI.
- Add a Tauri/manual smoke checklist for packaged exe.

Acceptance:

- Main views do not show blank screens.
- Long paths/logs do not break layout.
- Core flow can be smoke-tested: add root, scan, open repo, audit, verify, search.
- Screenshot or manual notes demonstrate packaged app launches.

Reviewer focus:

- Check text overflow, table resizing, and modal layering.
- Check buttons are disabled while operations are running.
- Check no user-visible raw JSON unless it is intentionally in a details view.

### Wave 12: AtlasForge Self-Release Readiness

Purpose:
Make AtlasForge itself pass the standards it applies to other repos.

Tasks:

- Fix Tauri bundle identifier warning. Avoid identifier ending in `.app`; use something like `com.atlasforge.desktop`.
- Add app metadata and icon polish.
- Add README sections for install, development, validation, security model, and x64 build.
- Add release notes template.
- Add a release verification script if useful.
- Keep generated artifacts ignored unless explicitly publishing them.
- Ensure `.gitignore` excludes `dist`, `node_modules`, `src-tauri/target`, local DB, and local env files.
- Add public-facing screenshots only after UI is stable.

Acceptance:

- Fresh clone can install dependencies and run validation commands.
- Windows x64 installer builds.
- README accurately describes current product and limitations.
- Release artifacts are reproducible from commands.

Reviewer focus:

- Check version consistency.
- Check installer launches.
- Check public docs do not overclaim AI autonomy or GitHub write safety.

## High-Value Tests To Add

Backend Rust tests:

- `security::authorize_path` allows only roots and rejects outside paths.
- `security::authorize_write` rejects read-only roots.
- scanner stable identity across rescans.
- scanner handles `.git` file worktrees.
- profile detector for Node, Rust, Python, docs-only repos.
- indexer excludes `.env` and key files.
- audit produces evidence-bearing findings.
- AI provider rejects raw-looking secrets in `apiKeyRef`.
- patch apply rejects invalid or outside-root patches.

Frontend tests:

- smoke render for all routes.
- type-safe parsing of health snapshot category JSON.
- repository filters.
- job event rendering.
- verification result rendering.
- empty/error states.

End-to-end/manual smoke:

- Launch app.
- Add root.
- Scan.
- Open repo.
- Run audit.
- Reindex.
- Search.
- Detect commands.
- Run verification.
- Check Tasks timeline.
- Build x64 package.

## Dirty Work For AtomCode

These are good tasks for AtomCode because they are repetitive but important:

- Add fixture repos for scanner/profile/audit tests.
- Add typed JSON parsing helpers for frontend.
- Convert repeated inline UI styles into small reusable components.
- Add loading/error/empty states across pages.
- Add persistent verification result schema and UI.
- Add audit log UI.
- Add migration tests.
- Add Playwright smoke tests.
- Update README and docs after behavior changes.
- Run full validation after each wave and paste evidence into a short completion note.

## Required Evidence After Each Wave

AtomCode should report:

- Files changed.
- Behavior changed.
- Validation commands run.
- Test output summary.
- Known residual warnings or risks.
- Screenshots or manual smoke notes if UI changed.
- Any skipped validation and why.

Minimum validation:

```powershell
npm run typecheck
npm run lint
npm test
npm run build
```

When Rust changed:

```powershell
cmd /c "call ""C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"" && set PATH=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;%USERPROFILE%\.cargo\bin;%PATH% && cargo +stable-x86_64-pc-windows-msvc check --target x86_64-pc-windows-msvc"
```

Before claiming release/build readiness:

```powershell
cmd /c "call ""C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"" && set ""RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc"" && set PATH=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;%USERPROFILE%\.cargo\bin;%PATH% && npm run tauri -- build --target x86_64-pc-windows-msvc"
```

## AtomCode Master Prompt

Use this prompt to hand the work to AtomCode:

```text
You are working on AtlasForge at:
C:\Users\24377\Desktop\新建文件夹\AtlasForge

Read these first:
- README.md
- AGENTS.md
- docs/11-windows-native-validation.md
- docs/12-alpha-productization-plan.md
- docs/07-ui-and-workflows.md
- docs/08-implementation-backlog.md
- docs/09-validation-and-quality-gates.md

Current status:
- AtlasForge is an Alpha 0 Tauri v2 + React + TypeScript app.
- Windows x64 is the active build target. Do not work on native ARM64 unless explicitly asked.
- Frontend validation passes: npm run typecheck, npm run lint, npm test, npm run build.
- x64 Rust check passes with stable-x86_64-pc-windows-msvc.
- x64 Tauri build passes when RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc is set.

Goal:
Turn the current buildable baseline into a usable alpha product by executing docs/12-alpha-productization-plan.md wave by wave.

Execution rules:
1. Start with Wave 1 and continue in dependency order.
2. Keep changes focused. Do not jump to GitHub write, ARM64, cloud sync, or autonomous release work.
3. Preserve the safety model:
   - frontend must not execute shell or mutate files directly;
   - path writes must go through backend authorization;
   - GitHub writes remain gated by ATLASFORGE_ENABLE_GITHUB_WRITE=1;
   - never persist raw API keys or secrets.
4. Add or update tests for each behavior change.
5. Update docs when behavior or validation commands change.
6. Do not weaken validation scripts, lint rules, permissions, or security checks just to pass.
7. Do not remove existing planned modules just because they are not fully wired yet; wire them into the product loop or leave clear notes.

Required validation after each meaningful package:
- npm run typecheck
- npm run lint
- npm test
- npm run build

When Rust/backend/Tauri changed, also run:
cmd /c "call ""C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"" && set PATH=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;%USERPROFILE%\.cargo\bin;%PATH% && cargo +stable-x86_64-pc-windows-msvc check --target x86_64-pc-windows-msvc"

Before claiming product/build readiness, also run:
cmd /c "call ""C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"" && set ""RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc"" && set PATH=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;%USERPROFILE%\.cargo\bin;%PATH% && npm run tauri -- build --target x86_64-pc-windows-msvc"

Report back with:
- completed waves/packages;
- files changed;
- validation commands and outcomes;
- residual warnings/risks;
- anything that needs human review.
```

