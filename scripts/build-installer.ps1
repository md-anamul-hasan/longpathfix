<#
.SYNOPSIS  Builds a self-contained LongPathFix and packages it as dist\installer\LongPathFix-Setup-<version>.exe (needs Inno Setup 6).
#>
param(
    [string]$Version,
    [switch]$PublishOnly,  # stop after filling dist\stage (CI signs the binary there before packaging)
    [switch]$SkipPublish   # package an existing dist\stage as-is
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $Version) { $Version = ([xml](Get-Content (Join-Path $root 'Directory.Build.props'))).Project.PropertyGroup.Version }
$stage = Join-Path $root 'dist\stage'

if ($PublishOnly -and $SkipPublish) { throw '-PublishOnly and -SkipPublish cannot be combined' }

$iscc = @("$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe", "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:ProgramFiles\Inno Setup 6\ISCC.exe") |
    Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc -and -not $PublishOnly) { throw 'Inno Setup 6 not found. Install it with: winget install JRSoftware.InnoSetup' }

if (-not $SkipPublish) {
    if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
    # Self-contained single file, so the installed app needs no separate .NET runtime
    dotnet publish (Join-Path $root 'LongPathFix.csproj') -c Release -r win-x64 --self-contained true `
        -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true -p:EnableCompressionInSingleFile=true `
        -p:ContinuousIntegrationBuild=true -p:DebugType=none -p:DebugSymbols=false -p:SatelliteResourceLanguages=en `
        "-p:PathMap=$root=/_/" -o $stage --nologo
    if ($LASTEXITCODE -ne 0) { throw 'dotnet publish failed' }
}

if ($PublishOnly) { return }

& $iscc "/DAppVersion=$Version" (Join-Path $root 'installer\longpathfix.iss')
if ($LASTEXITCODE -ne 0) { throw 'Inno Setup compilation failed' }

Get-Item (Join-Path $root "dist\installer\LongPathFix-Setup-$Version.exe") | Select-Object Name, @{n='MB';e={[math]::Round($_.Length/1MB,1)}}
