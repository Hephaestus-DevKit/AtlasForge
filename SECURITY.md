# Security Policy

## Supported versions

AtlasForge is pre-1.0 software. Security fixes are applied to the latest tagged release and `main`.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability or include credentials, private repository
content, local paths, or exploit details in discussions. Use GitHub's private vulnerability reporting
for this repository. Include the affected version, reproduction steps, impact, and any suggested
mitigation. You should receive an acknowledgement within five business days.

## Security boundary

AtlasForge is local-first but processes untrusted repository content. Filesystem reads, verification
commands, AI context, patch application, rollback, Git operations, and GitHub operations must remain
behind the documented path, approval, audit, timeout, and recovery controls. See
`docs/06-security-and-permissions.md` for the threat model.
