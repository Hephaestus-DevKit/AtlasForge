# Changelog

All notable changes to AtlasForge will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.1.1] - 2026-08-05

### Fixed

- Align AI provider adapter values with the persisted SQLite contract.
- Enforce one case-insensitive sensitive-path policy across indexing and Tool Broker reads.
- Redact Tool Broker results before returning them and bound filesystem reads.
- Persist patch recovery metadata before mutation; add approved, hash-bound rollback and startup recovery.
- Enforce valid job terminal transitions and atomically consume batch approvals and replace health snapshots.
- Reconcile missing repository assets without deleting their audit history.
- Prevent stale repository detail requests and profile caches from overwriting current UI state.

### Changed

- Move automation scheduling into the trusted Rust lifecycle and make task polling non-overlapping.
- Upgrade the supported runtime to Node.js 24 LTS and current compatible Tauri, Playwright, Vite, and Vitest releases.
- Use official GitHub Pages actions with minimal permissions and add native Tauri build, Rust formatting, clippy, CodeQL, Dependabot, and repository governance files.
- Add jsdom interaction coverage and keyboard/focus behavior for repository tables, tabs, and approval dialogs.
- Remove generated Tauri schemas and machine-specific historical build transcripts from source control.

### Security

- Filter excluded and sensitive directory entries from `fs.list`, block them in `fs.read`, and keep raw secret-bearing tool output out of caller responses.
- Require a context-bound, expiring, single-use approval before rollback modifies a worktree.

## [0.1.0] - 2026-08-02

### Added

- Workspace root management with folder picker and validated include/exclude globs.
- Repository scanning with stable identity, profiling, health audit, and bounded parallel workers.
- Verification command detection, isolated execution, approval review, and evidence capture.
- Incremental knowledge indexing, redaction, chunking, and full-text search.
- AI provider configuration, report generation, patch proposal, approval, and rollback workflows.
- GitHub read integration for workflows, pull requests, releases, and sync diagnostics.
- Automation rules, persistent jobs, event timelines, notifications, and audit records.
- Browser demo, responsive application shell, error boundaries, loading states, and empty states.
- Windows x64 NSIS/MSI packaging configuration.

### Fixed

- Split the repository workspace into feature-scoped tabs; separate Rust workspace, GitHub, verification, and automation command modules from the central dispatcher.
- Remove committed editor-generated `.atomcode` state, including its binary repository index, and ignore future machine-local cache files.
- Replace the README's stale demo URL and broad product claims with the 0.1.0 capability boundary, architecture, security model, and reproducible gates.
- Preserve GitHub integration and evidence identity across repeated resolution, normalize PR states, reject malformed identifiers, and surface database sync failures.
- Repair FTS path cleanup so reindexing and file deletion do not leave stale search terms.
- Match excluded workspace paths case-insensitively on Windows and avoid Unicode path slicing failures.
- Redact modern GitHub/OpenAI key formats and Authorization headers from stored or indexed content.
- Show exact isolated verification commands, risks, and lifecycle scripts during patch approval.
- Bound list query sizes and stop ignoring workspace scan timestamp write failures.
- Use hash routing for static hosting so deep links and refreshes work on GitHub Pages.
- Pin GitHub Actions dependencies to immutable commits and run browser E2E coverage in Windows CI.
- Enforce workspace include globs, reject invalid root settings, and fail closed on invalid scan patterns.
- Bound repository scanning and file indexing workers and move blocking operations off the async runtime.
- Only advertise executable Tool Broker entries and truncate UTF-8 output safely.
- Replace unverified performance claims with reproducible, measurable behavior.

### Security

- Removed the unnecessary React Router dependency and updated compatible transitive patches; `npm audit` reports zero known vulnerabilities.
