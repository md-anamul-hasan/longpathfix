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

& (Join-Path $PSScriptRoot 'build-installer.ps1') -SkipFrontendBuild:$SkipFrontendBuild -SkipCargoBuild:$SkipCargoBuild

$PortableDir = Join-Path $RepoRoot "dist\portable\LongPathFix-$Version-win64"
if (Test-Path $PortableDir) {
    Remove-Item -Recurse -Force $PortableDir
}
New-Item -ItemType Directory -Force $PortableDir | Out-Null

Copy-Item (Join-Path $RepoRoot 'target\release\longpathfix.exe') (Join-Path $PortableDir 'LongPathFix.exe') -Force
Copy-Item (Join-Path $RepoRoot 'LICENSE') (Join-Path $PortableDir 'LICENSE') -Force
Copy-Item (Join-Path $RepoRoot 'THIRD_PARTY_NOTICES.md') (Join-Path $PortableDir 'THIRD_PARTY_NOTICES.md') -Force
Copy-Item (Join-Path $RepoRoot 'README.md') (Join-Path $PortableDir 'README.md') -Force

$ZipPath = Join-Path $RepoRoot "dist\release\LongPathFix-$Version-win64-portable.zip"
if (Test-Path $ZipPath) {
    Remove-Item -Force $ZipPath
}
Compress-Archive -Path "$PortableDir\*" -DestinationPath $ZipPath -CompressionLevel Optimal

$ReleaseDir = Join-Path $RepoRoot 'dist\release'
$Files = Get-ChildItem $ReleaseDir -File | Where-Object { $_.Name -ne 'SHA256SUMS.txt' } | Sort-Object Name
$SumLines = foreach ($File in $Files) {
    $Hash = (Get-FileHash $File.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    "$Hash *$($File.Name)"
}
$SumsPath = Join-Path $ReleaseDir 'SHA256SUMS.txt'
[System.IO.File]::WriteAllLines($SumsPath, $SumLines, [System.Text.UTF8Encoding]::new($false))

Write-Host "Release packaging complete:" -ForegroundColor Green
Get-ChildItem $ReleaseDir | Format-Table Name, Length -AutoSize
