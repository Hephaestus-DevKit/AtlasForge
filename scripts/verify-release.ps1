# AtlasForge Release Verification Script
# Run this before tagging a release

param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$E2E,
    [switch]$FullBuild
)

$ErrorActionPreference = "Stop"
$TotalSteps = 6
if ($E2E.IsPresent) { $TotalSteps++ }
if ($FullBuild.IsPresent) { $TotalSteps++ }
$Step = 0

Write-Host "=== AtlasForge Development Verification ===" -ForegroundColor Cyan

$Step++
Write-Host "[$Step/$TotalSteps] Running typecheck..." -ForegroundColor Yellow
npm run typecheck
if ($LASTEXITCODE -ne 0) { throw "Typecheck failed" }

$Step++
Write-Host "[$Step/$TotalSteps] Running lint..." -ForegroundColor Yellow
npm run lint
if ($LASTEXITCODE -ne 0) { throw "Lint failed" }

$Step++
Write-Host "[$Step/$TotalSteps] Running frontend tests..." -ForegroundColor Yellow
npm test
if ($LASTEXITCODE -ne 0) { throw "Tests failed" }

$Step++
Write-Host "[$Step/$TotalSteps] Running frontend build..." -ForegroundColor Yellow
npm run build
if ($LASTEXITCODE -ne 0) { throw "Build failed" }

$Step++
Write-Host "[$Step/$TotalSteps] Running Rust tests..." -ForegroundColor Yellow
$CargoArgs = @("test", "--manifest-path", "src-tauri\Cargo.toml", "--target", $Target)
if ($Target -eq "x86_64-pc-windows-msvc") {
    $CargoArgs = @("+stable-x86_64-pc-windows-msvc") + $CargoArgs
}
& cargo @CargoArgs
if ($LASTEXITCODE -ne 0) { throw "Rust tests failed" }

$Step++
Write-Host "[$Step/$TotalSteps] Running dependency audit..." -ForegroundColor Yellow
npm audit --omit=optional
if ($LASTEXITCODE -ne 0) { throw "Dependency audit failed" }

if ($E2E) {
    $Step++
    Write-Host "[$Step/$TotalSteps] Running browser E2E tests..." -ForegroundColor Yellow
    npm run test:e2e
    if ($LASTEXITCODE -ne 0) { throw "E2E tests failed" }
}

if ($FullBuild) {
    $Step++
    Write-Host "[$Step/$TotalSteps] Running Tauri bundle build..." -ForegroundColor Yellow
    $PreviousToolchain = $env:RUSTUP_TOOLCHAIN
    try {
        if ($Target -eq "x86_64-pc-windows-msvc") {
            $env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-msvc"
        }
        npm run tauri -- build --target $Target
        if ($LASTEXITCODE -ne 0) { throw "Tauri build failed" }
    } finally {
        $env:RUSTUP_TOOLCHAIN = $PreviousToolchain
    }
}

Write-Host "" -ForegroundColor Green
Write-Host "=== All checks passed! ===" -ForegroundColor Green
Write-Host "" -ForegroundColor Green
if (-not $FullBuild) {
    Write-Host "Installer build skipped. Add -FullBuild when packaging is required." -ForegroundColor White
}
Write-Host "Verify manually using docs/tauri-smoke-checklist.md" -ForegroundColor White
