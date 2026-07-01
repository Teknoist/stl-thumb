[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = 'Medium')]
param(
    [ValidateSet('Protect', 'Restore', 'Status')]
    [string] $Mode = 'Protect',

    [string[]] $CleanupPath,

    [switch] $Recurse,

    [switch] $RestartExplorer
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$registryPath = 'HKCU:\Software\Policies\Microsoft\Windows\Explorer'
$valueName = 'DisableThumbsDBOnNetworkFolders'

function Test-NetworkPath {
    param(
        [Parameter(Mandatory)]
        [System.IO.DirectoryInfo] $Directory
    )

    if ($Directory.FullName.StartsWith('\\')) {
        return $true
    }

    $root = [System.IO.Path]::GetPathRoot($Directory.FullName)
    if ([string]::IsNullOrWhiteSpace($root)) {
        return $false
    }

    $driveName = $root.TrimEnd('\').TrimEnd(':')
    $psDrive = Get-PSDrive -Name $driveName -ErrorAction SilentlyContinue
    if ($null -eq $psDrive) {
        return $false
    }

    $displayRoot = $psDrive.PSObject.Properties['DisplayRoot']
    return $null -ne $displayRoot -and
        $null -ne $displayRoot.Value -and
        $displayRoot.Value.StartsWith('\\')
}

function Get-PolicyValue {
    if (-not (Test-Path -LiteralPath $registryPath)) {
        return $null
    }

    $properties = Get-ItemProperty -LiteralPath $registryPath -ErrorAction SilentlyContinue
    if ($null -eq $properties) {
        return $null
    }

    $property = $properties.PSObject.Properties[$valueName]
    if ($null -eq $property) {
        return $null
    }

    return $property.Value
}

function Remove-NetworkThumbsDb {
    [CmdletBinding(SupportsShouldProcess = $true)]
    param(
        [Parameter(Mandatory)]
        [string] $Path
    )

    $directory = Get-Item -LiteralPath $Path -Force
    if (-not $directory.PSIsContainer -or $directory.PSProvider.Name -ne 'FileSystem') {
        throw "Cleanup path must be a file-system directory: $Path"
    }
    if (-not (Test-NetworkPath -Directory $directory)) {
        throw "Refusing to clean a local path. Provide a UNC path or mapped network drive: $Path"
    }

    $files = @(Get-ChildItem -LiteralPath $directory.FullName -Filter 'Thumbs.db' -File -Force -Recurse:$Recurse)
    $deleted = 0
    $failed = 0

    foreach ($file in $files) {
        try {
            if ($PSCmdlet.ShouldProcess($file.FullName, 'Delete stale SMB thumbnail cache')) {
                Remove-Item -LiteralPath $file.FullName -Force
                $deleted++
            }
        }
        catch {
            $failed++
            Write-Warning "Could not delete '$($file.FullName)'. Close Explorer windows using this share and retry. $($_.Exception.Message)"
        }
    }

    [pscustomobject]@{
        Path    = $directory.FullName
        Found   = $files.Count
        Deleted = $deleted
        Failed  = $failed
    }
}

switch ($Mode) {
    'Protect' {
        if ($PSCmdlet.ShouldProcess($registryPath, 'Disable Thumbs.db caching on network folders for the current user')) {
            $null = New-Item -Path $registryPath -Force
            $null = New-ItemProperty -LiteralPath $registryPath -Name $valueName -PropertyType DWord -Value 1 -Force
        }
    }
    'Restore' {
        if ((Get-PolicyValue) -ne $null -and
            $PSCmdlet.ShouldProcess("$registryPath\$valueName", 'Restore the Windows default network thumbnail-cache policy')) {
            Remove-ItemProperty -LiteralPath $registryPath -Name $valueName
        }
    }
    'Status' { }
}

$policyValue = Get-PolicyValue
[pscustomobject]@{
    Scope     = 'CurrentUser'
    Protected = $policyValue -eq 1
    Value     = $policyValue
    Registry  = "$registryPath\$valueName"
}

if ($RestartExplorer -and $Mode -ne 'Status') {
    if ($PSCmdlet.ShouldProcess('explorer.exe', 'Restart Windows Explorer so the policy takes effect immediately')) {
        Get-Process -Name explorer -ErrorAction SilentlyContinue | Stop-Process -Force
    }
}
elseif ($Mode -ne 'Status' -and -not $WhatIfPreference) {
    Write-Host 'Sign out or restart Windows Explorer before testing the new policy.'
}

foreach ($path in $CleanupPath) {
    Remove-NetworkThumbsDb -Path $path
}
