# Alpha Productization Plan

This document records the active productization boundary. Historical command transcripts and
machine-specific build notes belong in release evidence, not in the maintained architecture docs.

## Current baseline

- Windows x64 desktop application built with Tauri 2, Rust, React, and SQLite.
- Local-first repository discovery, profiling, auditing, text indexing, verification, AI planning,
  controlled single-file patching, rollback, and GitHub read synchronization.
- High-risk operations require context-bound, expiring, single-use approval and an audit trail.
- GitHub mutations, unattended repository modification, updater delivery, vector search, and native
  ARM64 packaging remain outside the alpha boundary.

## Required release evidence

- TypeScript typecheck and zero-warning ESLint.
- DOM unit tests and browser E2E tests.
- Rust formatting, clippy with warnings denied, and Rust tests on Windows x64.
- Native Tauri no-bundle build in CI.
- Dependency audit with no moderate-or-higher findings.
- Successful CodeQL and GitHub Pages deployment.
- Manual Windows smoke checklist for installer releases.

## Maintenance priorities

1. Keep path authorization and sensitive-file exclusions centralized.
2. Keep task transitions, approvals, snapshot replacement, scan reconciliation, and filesystem
   mutations transactional or explicitly recoverable.
3. Route blocking filesystem, Git, GitHub CLI, and verification work away from async UI handlers.
4. Add migrations rather than modifying released migration files.
5. Split modules when a stable domain boundary appears; avoid thin files that only move complexity.
6. Update the capability matrix whenever actual behavior changes.

## Exit criteria for the alpha phase

- Recovery behavior is exercised against prior database fixtures and interrupted operations.
- Installer signing and an updater threat model are designed and validated.
- Repository-scale performance targets and retention policies are based on real workloads.
- Public contribution, vulnerability reporting, branch protection, dependency updates, and code
  scanning operate continuously.
