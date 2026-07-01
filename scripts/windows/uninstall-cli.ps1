#Requires -Version 5.1
[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "Programs\stl-thumb"),

    [switch]$RemoveFromPath,

    [switch]$UndoThumbsDbPolicies
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (Test-Path -LiteralPath $InstallDir) {
    if ($PSCmdlet.ShouldProcess($InstallDir, "Remove stl-thumb install directory")) {
        Remove-Item -LiteralPath $InstallDir -Recurse -Force
    }
}

if ($RemoveFromPath) {
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (![string]::IsNullOrWhiteSpace($currentPath)) {
        $parts = $currentPath -split ";" | Where-Object {
            $_ -and ($_.TrimEnd("\") -ine $InstallDir.TrimEnd("\"))
        }
        if ($PSCmdlet.ShouldProcess("User PATH", "Remove $InstallDir")) {
            [Environment]::SetEnvironmentVariable("Path", ($parts -join ";"), "User")
        }
    }
}

if ($UndoThumbsDbPolicies) {
    $paths = @(
        @{ Path = "HKCU:\Software\Policies\Microsoft\Windows\Explorer"; Name = "DisableThumbsDBOnNetworkFolders" },
        @{ Path = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Policies\Explorer"; Name = "NoThumbnailCache" }
    )

    foreach ($item in $paths) {
        if (Test-Path $item.Path) {
            if (Get-ItemProperty -Path $item.Path -Name $item.Name -ErrorAction SilentlyContinue) {
                if ($PSCmdlet.ShouldProcess($item.Path, "Remove $($item.Name)")) {
                    Remove-ItemProperty -Path $item.Path -Name $item.Name -ErrorAction SilentlyContinue
                }
            }
        }
    }
}

Write-Host "Uninstall helper finished."
