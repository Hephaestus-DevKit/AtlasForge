# ADR: Trusted Local Execution

## Status

Accepted.

## Context

Repository verification commands are not read-only merely because AtlasForge selects the command.
npm lifecycle scripts, Cargo build scripts, procedural macros, tests, and other repository-controlled
programs can execute arbitrary code. AI patches also require stronger guarantees than direct
application followed by a best-effort rollback.

## Decision

1. Repository-controlled commands always require explicit approval.
2. Approval records are single-use, expire, and are bound to a context hash.
3. The context includes the canonical repository path, command expansion, and relevant manifest,
   lockfile, and build-script hashes.
4. Verification output is drained concurrently and retained with a bounded tail.
5. Timeout and cancellation terminate the command process tree on Windows.
6. AI patches require a clean Git worktree.
7. AI patches are applied and verified in a temporary detached worktree first.
8. The main worktree baseline is checked again before final application.
9. Rollback is allowed only when the current file matches the recorded applied hash.
10. GitHub write operations remain disabled until they have dedicated previews and approvals.

## Consequences

- Users see actual script bodies and risk reasons before code executes.
- Repository changes invalidate previous approvals.
- Dirty worktrees are preserved.
- Patch application is slower because verification happens in isolation, but failed patches cannot
  modify the user worktree.
- Generic autonomous command execution is intentionally unavailable.
