# Contributing to AtlasForge

AtlasForge targets Windows 10/11 x64 with Node.js 24 LTS and stable Rust for
`x86_64-pc-windows-msvc`.

1. Fork the repository and create a focused branch from `main`.
2. Keep UI code behind `src/api`, and keep filesystem, shell, Git, GitHub, and permission decisions
   in the trusted Rust layer.
3. Add tests for behavior changes, especially path handling, migrations, approvals, jobs, and
   recovery paths.
4. Run `npm ci` and `npm run verify-dev -- -E2E` before opening a pull request.
5. Update the capability matrix, ADRs, or security documentation when a boundary changes.

Never commit tokens, private keys, local databases, user repository content, or machine-specific
paths. High-risk changes must preserve preview, explicit approval, audit evidence, and rollback or
compensating recovery.
