# Windows Native Validation

Windows 10/11 x64 (`x86_64-pc-windows-msvc`) is the supported native baseline.

## Prerequisites

- Node.js 24 LTS
- Rust stable with the `x86_64-pc-windows-msvc` target
- Visual Studio 2022 Build Tools with Desktop development with C++
- WebView2 Runtime

Do not encode an individual machine's Visual Studio version or absolute workspace path in scripts.
Use a Developer PowerShell or let the CI runner initialize MSVC.

## Reproducible gates

```powershell
npm ci
npm run verify-dev -- -E2E
npm run tauri -- build --no-bundle --target x86_64-pc-windows-msvc
```

For a release candidate, also build the installers:

```powershell
npm run verify-release -- -E2E -FullBuild
```

Expected bundles are generated below
`src-tauri/target/x86_64-pc-windows-msvc/release/bundle/`. These files are build artifacts and are
not committed.

## Manual smoke checklist

1. Start the application and confirm the app-data database opens or migrates successfully.
2. Add read-only and read-write roots, including a path with Unicode characters.
3. Scan repositories and verify include/exclude globs, mixed-case sensitive directories, and stale
   repository reconciliation.
4. Run a verification command and confirm preview, single-use approval, timeout, cancellation, and
   audit evidence.
5. Configure local and OpenAI-compatible providers using environment-variable references only.
6. Generate, approve, apply, and roll back a single-file patch; restart during prepared operations to
   exercise recovery.
7. Confirm GitHub read synchronization works and GitHub mutations remain disabled.
8. Confirm keyboard navigation and focus restoration for tables, tabs, and approval dialogs.

Record the tested commit, Windows build, Node version, Rust toolchain, and result in the release notes;
do not record personal filesystem paths.
