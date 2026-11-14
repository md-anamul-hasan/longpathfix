param(
    [switch]$SkipFrontendBuild,
    [switch]$SkipCargoBuild
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $RepoRoot

$CargoToml = Get-Content (Join-Path $RepoRoot 'Cargo.toml') -Raw
if ($CargoToml -match '(?m)^version\s*=\s*"([^"]+)"') {
    $Version = $Matches[1]
} else {
    throw 'Could not parse workspace version from Cargo.toml'
}

if (-not $SkipFrontendBuild) {
    Write-Host "Building frontend (src-ui)..." -ForegroundColor Cyan
    Push-Location (Join-Path $RepoRoot 'src-ui')
    try {
        npm run build
        if ($LASTEXITCODE -ne 0) { throw "Frontend build failed with exit code $LASTEXITCODE" }
    } finally {
        Pop-Location
    }
}

if (-not $SkipCargoBuild) {
    Write-Host "Building native Rust release binary (v$Version)..." -ForegroundColor Cyan
    cargo build --release -p longpathfix --locked
    if ($LASTEXITCODE -ne 0) { throw "Cargo release build failed with exit code $LASTEXITCODE" }
}

$ExePath = Join-Path $RepoRoot 'target\release\longpathfix.exe'
if (-not (Test-Path $ExePath)) {
    throw "Expected release binary not found at $ExePath"
}

$IsccCandidates = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
)
$Iscc = $IsccCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $Iscc) {
    $Cmd = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($Cmd) { $Iscc = $Cmd.Source }
}
if (-not $Iscc) {
    throw 'Inno Setup 6 (ISCC.exe) was not found.'
}

New-Item -ItemType Directory -Force (Join-Path $RepoRoot 'dist\release') | Out-Null
Write-Host "Compiling Inno Setup installer ($Iscc)..." -ForegroundColor Cyan
& $Iscc "/DMyAppVersion=$Version" (Join-Path $RepoRoot 'installer\longpathfix.iss')
if ($LASTEXITCODE -ne 0) { throw "ISCC failed with exit code $LASTEXITCODE" }

$InstallerPath = Join-Path $RepoRoot "dist\release\LongPathFix-Setup-$Version.exe"
if (-not (Test-Path $InstallerPath)) {
    throw "Installer was not generated at $InstallerPath"
}
Write-Host "Installer created: $InstallerPath" -ForegroundColor Green
