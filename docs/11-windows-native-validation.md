# Windows Native Validation

This note records the current Windows-native validation state. The first supported baseline is Windows x64 (`x86_64-pc-windows-msvc`) because Windows on ARM can run x64 apps under emulation.

## Current Result

Verified on Windows from:

```text
C:\Users\24377\Desktop\新建文件夹\AtlasForge
```

Passing:

```powershell
npm run typecheck
npm run lint
npm test
npm run build
npm audit --omit=optional
npm run tauri -- --version
cargo +stable-x86_64-pc-windows-msvc check --target x86_64-pc-windows-msvc
npm run tauri -- build --target x86_64-pc-windows-msvc  # with RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc
```

Observed:

```text
tauri-cli 2.11.2
npm 11.12.1
WebView2 detected
rustc 1.96.0
cargo 1.96.0
Rust toolchain: stable-aarch64-pc-windows-msvc default, stable-x86_64-pc-windows-msvc installed
```

Generated x64 artifacts:

```text
C:\Users\24377\Desktop\新建文件夹\AtlasForge\src-tauri\target\x86_64-pc-windows-msvc\release\atlasforge.exe
C:\Users\24377\Desktop\新建文件夹\AtlasForge\src-tauri\target\x86_64-pc-windows-msvc\release\bundle\msi\AtlasForge_0.1.0_x64_en-US.msi
C:\Users\24377\Desktop\新建文件夹\AtlasForge\src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis\AtlasForge_0.1.0_x64-setup.exe
```

The executable PE header was verified as `x64`.

Default ARM64-native build remains deferred. On this machine, `npm run tauri -- build` without an x64 target can select the ARM64 Rust host and fail if native ARM64 MSVC linker components are not installed.

## Required Environment Work

For a fresh Windows machine, install Rustup:

```powershell
winget install --id Rustlang.Rustup --source winget --accept-package-agreements --accept-source-agreements
```

Install Visual Studio Build Tools 2022 with C++ tools:

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools --source winget --accept-package-agreements --accept-source-agreements --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

For the current x64 baseline on Windows ARM, also install the x64 Rust target/toolchain:

```powershell
rustup target add x86_64-pc-windows-msvc
rustup toolchain install stable-x86_64-pc-windows-msvc --force-non-host
```

## ARM64 Windows Note

This machine reports Windows as `aarch64` while Node/PowerShell can run as `X64` under emulation. That is acceptable for the first validation pass. Native ARM64 is a later hardening target, not part of the current x64 baseline.

For native ARM64 Windows builds, install or add the ARM64 C++ build tools in Visual Studio Installer:

```text
Visual Studio Installer -> Build Tools 2022 -> Modify -> Individual components
```

Select the latest ARM64 MSVC build tools and a Windows SDK. Microsoft currently lists ARM64-related Build Tools components such as:

```text
Microsoft.VisualStudio.Component.VC.Tools.ARM64
Microsoft.VisualStudio.Component.VC.Tools.ARM64EC
Microsoft.VisualStudio.Component.VC.14.44.17.14.ARM64
```

Observed missing file when ARM64 C++ tools are absent:

```text
C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostarm64\arm64\link.exe
```

Command-line install/modify option:

```powershell
& "C:\Program Files (x86)\Microsoft Visual Studio\Installer\setup.exe" modify `
  --installPath "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools" `
  --passive --norestart `
  --add Microsoft.VisualStudio.Component.VC.Tools.ARM64
```

If the installer UI opens, use Modify and select the latest `MSVC v143 - VS 2022 C++ ARM64/ARM64EC build tools` component.

Then add the Rust ARM64 target:

```powershell
rustup target add aarch64-pc-windows-msvc
```

Native ARM64 Tauri build command:

```powershell
npm run tauri -- build --target aarch64-pc-windows-msvc
```

An x64 build is still acceptable for early validation because Windows on ARM can run x64 apps under emulation. The native ARM64 build is the better final target for this machine.

After installation, restart the terminal. If `cargo` is still not found, check:

```powershell
$env:USERPROFILE\.cargo\bin\cargo.exe --version
$env:USERPROFILE\.cargo\bin\rustup.exe show
```

If those work but `cargo` is not on PATH, add this user PATH entry:

```text
%USERPROFILE%\.cargo\bin
```

## Hard Validation Commands

Run after Rust and MSVC Build Tools are visible in a fresh Windows terminal:

```powershell
rustup show
cargo --version
rustc --version
npm run typecheck
npm run lint
npm test
npm run build
npm audit --omit=optional
```

Then run the x64 Rust check from `src-tauri`:

```powershell
cmd /c "call ""C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"" && set PATH=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;%USERPROFILE%\.cargo\bin;%PATH% && cargo +stable-x86_64-pc-windows-msvc check --target x86_64-pc-windows-msvc"
```

Run the x64 Tauri build from the repo root:

```powershell
cmd /c "call ""C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"" && set ""RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc"" && set PATH=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;%USERPROFILE%\.cargo\bin;%PATH% && npm run tauri -- build --target x86_64-pc-windows-msvc"
```

## AtomCode Prompt

```text
You are working on AtlasForge at:
C:\Users\24377\Desktop\新建文件夹\AtlasForge

Goal:
Keep the Windows x64 baseline green. Do not work on native ARM64 yet.

Known current state:
- `npm run typecheck`, `npm run lint`, `npm test`, `npm run build`, and `npm audit --omit=optional` pass.
- `cargo +stable-x86_64-pc-windows-msvc check --target x86_64-pc-windows-msvc` passes.
- `npm run tauri -- build --target x86_64-pc-windows-msvc` passes when `RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc` is set.
- Generated x64 artifacts are under `src-tauri\target\x86_64-pc-windows-msvc\release\bundle`.

Validation command:
cmd /c "call ""C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"" && set ""RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc"" && set PATH=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;%USERPROFILE%\.cargo\bin;%PATH% && npm run tauri -- build --target x86_64-pc-windows-msvc"

If compilation fails after the environment is ready, fix only real code/config issues. Do not bypass checks by deleting scripts or weakening safety gates.

Deferred:
- Native ARM64 build can be handled later by installing ARM64 MSVC components and validating `--target aarch64-pc-windows-msvc`.
```
