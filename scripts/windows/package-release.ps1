#Requires -Version 5.1
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$BinaryPath,

    [string]$OutputDir = "dist",

    [string]$PackageName = "stl-thumb-windows-x64"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$binary = Resolve-Path -LiteralPath $BinaryPath
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$outRoot = Join-Path $repoRoot $OutputDir
$packageDir = Join-Path $outRoot $PackageName
$zipPath = Join-Path $outRoot "$PackageName.zip"

if (Test-Path $packageDir) {
    Remove-Item -LiteralPath $packageDir -Recurse -Force
}
New-Item -ItemType Directory -Path $packageDir | Out-Null
New-Item -ItemType Directory -Path (Join-Path $packageDir "scripts\windows") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $packageDir "docs") -Force | Out-Null

Copy-Item -LiteralPath $binary -Destination (Join-Path $packageDir "stl-thumb.exe") -Force

$copyIfExists = @(
    "README.md",
    "LICENSE",
    "docs\windows.md",
    "scripts\windows\install-cli.ps1",
    "scripts\windows\repair-thumbnail-cache.ps1",
    "scripts\windows\uninstall-cli.ps1"
)

foreach ($relative in $copyIfExists) {
    $src = Join-Path $repoRoot $relative
    if (Test-Path $src) {
        $dst = Join-Path $packageDir $relative
        New-Item -ItemType Directory -Path (Split-Path $dst -Parent) -Force | Out-Null
        Copy-Item -LiteralPath $src -Destination $dst -Force
    }
}

$quickStart = @"
stl-thumb Windows portable package
=================================

Quick test:
  .\stl-thumb.exe --help
  .\stl-thumb.exe C:\path\model.stl C:\path\thumb.png -s 512

Install CLI to Program Files:
  PowerShell as Administrator:
  .\scripts\windows\install-cli.ps1 -SourceDir . -AddToPath

Repair stale Windows thumbnail cache:
  .\scripts\windows\repair-thumbnail-cache.ps1 -RestartExplorer

More details: docs\windows.md
"@
Set-Content -LiteralPath (Join-Path $packageDir "WINDOWS-QUICKSTART.txt") -Value $quickStart -Encoding UTF8

if (Test-Path $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -Path (Join-Path $packageDir "*") -DestinationPath $zipPath -Force
Write-Host "Created $zipPath"
