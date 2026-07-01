#Requires -Version 5.1
[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [string]$SourceDir = (Resolve-Path (Join-Path $PSScriptRoot "..\..")),

    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "Programs\stl-thumb"),

    [switch]$AddToPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$sourceExe = Join-Path $SourceDir "stl-thumb.exe"
if (!(Test-Path -LiteralPath $sourceExe)) {
    $sourceExe = Join-Path (Resolve-Path (Join-Path $PSScriptRoot "..\..")) "target\release\stl-thumb.exe"
}
if (!(Test-Path -LiteralPath $sourceExe)) {
    throw "stl-thumb.exe was not found. Pass -SourceDir pointing to the portable package folder or build with cargo build --release first."
}

if ($PSCmdlet.ShouldProcess($InstallDir, "Install stl-thumb.exe")) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item -LiteralPath $sourceExe -Destination (Join-Path $InstallDir "stl-thumb.exe") -Force
}

if ($AddToPath) {
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @()
    if (![string]::IsNullOrWhiteSpace($currentPath)) {
        $parts = $currentPath -split ";" | Where-Object { $_ -and $_.Trim() }
    }

    $alreadyPresent = $false
    foreach ($part in $parts) {
        if ($part.TrimEnd("\") -ieq $InstallDir.TrimEnd("\")) {
            $alreadyPresent = $true
            break
        }
    }

    if (!$alreadyPresent) {
        $newPath = (($parts + $InstallDir) -join ";")
        if ($PSCmdlet.ShouldProcess("User PATH", "Add $InstallDir")) {
            [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        }
    }
}

Write-Host "Installed stl-thumb to: $InstallDir"
Write-Host "Open a new terminal and run: stl-thumb --help"
