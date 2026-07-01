#Requires -Version 5.1
[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [switch]$RestartExplorer,

    [switch]$ClearExplorerThumbnailCache,

    [switch]$DisableNetworkThumbsDb,

    [switch]$DisableAllThumbsDb,

    [string]$RemoveThumbsDbInPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Stop-ExplorerIfRequested {
    if ($RestartExplorer) {
        Write-Host "Stopping Explorer so locked thumbnail cache files can be removed..."
        Get-Process explorer -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 800
    }
}

function Start-ExplorerIfRequested {
    if ($RestartExplorer) {
        Write-Host "Starting Explorer..."
        Start-Process explorer.exe
    }
}

function Set-DwordValue {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][int]$Value
    )

    if (!(Test-Path $Path)) {
        New-Item -Path $Path -Force | Out-Null
    }
    New-ItemProperty -Path $Path -Name $Name -Value $Value -PropertyType DWord -Force | Out-Null
}

try {
    if ($DisableNetworkThumbsDb) {
        $policyPath = "HKCU:\Software\Policies\Microsoft\Windows\Explorer"
        if ($PSCmdlet.ShouldProcess($policyPath, "Disable Thumbs.db creation on network folders")) {
            Set-DwordValue -Path $policyPath -Name "DisableThumbsDBOnNetworkFolders" -Value 1
            Write-Host "Disabled Thumbs.db creation on network folders for the current user."
        }
    }

    if ($DisableAllThumbsDb) {
        $policyPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Policies\Explorer"
        if ($PSCmdlet.ShouldProcess($policyPath, "Disable thumbnail cache files for the current user")) {
            Set-DwordValue -Path $policyPath -Name "NoThumbnailCache" -Value 1
            Write-Host "Disabled thumbnail cache files for the current user."
        }
    }

    if ($ClearExplorerThumbnailCache) {
        Stop-ExplorerIfRequested
        $cacheDir = Join-Path $env:LOCALAPPDATA "Microsoft\Windows\Explorer"
        if (Test-Path $cacheDir) {
            $patterns = @("thumbcache_*.db", "iconcache_*.db")
            foreach ($pattern in $patterns) {
                Get-ChildItem -LiteralPath $cacheDir -Filter $pattern -Force -ErrorAction SilentlyContinue | ForEach-Object {
                    if ($PSCmdlet.ShouldProcess($_.FullName, "Remove Explorer cache file")) {
                        try {
                            Remove-Item -LiteralPath $_.FullName -Force -ErrorAction Stop
                            Write-Host "Removed $($_.FullName)"
                        } catch {
                            Write-Warning "Could not remove $($_.FullName): $($_.Exception.Message)"
                        }
                    }
                }
            }
        }
    }

    if (![string]::IsNullOrWhiteSpace($RemoveThumbsDbInPath)) {
        $target = Resolve-Path -LiteralPath $RemoveThumbsDbInPath
        Write-Host "Searching for Thumbs.db under $target"
        Get-ChildItem -LiteralPath $target -Filter "Thumbs.db" -Force -Recurse -ErrorAction SilentlyContinue | ForEach-Object {
            if ($PSCmdlet.ShouldProcess($_.FullName, "Remove Thumbs.db")) {
                try {
                    attrib -h -s -r $_.FullName 2>$null
                    Remove-Item -LiteralPath $_.FullName -Force -ErrorAction Stop
                    Write-Host "Removed $($_.FullName)"
                } catch {
                    Write-Warning "Could not remove $($_.FullName). Close Explorer windows or other PCs using this share, then retry. $($_.Exception.Message)"
                }
            }
        }
    }
} finally {
    Start-ExplorerIfRequested
}

Write-Host "Done. For policy changes, sign out/in or restart Windows if Explorer still creates Thumbs.db."
