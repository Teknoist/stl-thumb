# Windows support notes

This fork is currently focusing on making the Windows path easier to build, test, package, install, and repair.

## Build on Windows

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

The Windows GitHub Actions workflow also builds the same target and uploads a portable ZIP artifact.

## Portable package

After a successful GitHub Actions run, download the `stl-thumb-windows-x64` artifact. The ZIP contains:

- `stl-thumb.exe`
- Windows helper scripts
- this documentation
- quick-start text

Basic usage:

```powershell
.\stl-thumb.exe --help
.\stl-thumb.exe C:\Models\part.stl C:\Models\part.png -s 512
```

## Install the CLI for the current Windows user

```powershell
.\scripts\windows\install-cli.ps1 -SourceDir . -AddToPath
```

Open a new terminal afterwards:

```powershell
stl-thumb --help
```

## Repair thumbnail cache problems

Explorer can keep old thumbnails cached even after the renderer is fixed. Use:

```powershell
.\scripts\windows\repair-thumbnail-cache.ps1 -ClearExplorerThumbnailCache -RestartExplorer
```

## SMB / network share deletion problems

On Windows network shares, Explorer may create `Thumbs.db` files. Those files can become hidden/system files and may block deleting folders from another computer while Explorer or another client still has the folder open.

Recommended setting for shared STL folders:

```powershell
.\scripts\windows\repair-thumbnail-cache.ps1 -DisableNetworkThumbsDb
```

Then sign out/in or restart Windows if Explorer still creates `Thumbs.db`.

To remove existing `Thumbs.db` files from a shared folder:

```powershell
.\scripts\windows\repair-thumbnail-cache.ps1 -RemoveThumbsDbInPath "Z:\SharedModels"
```

If removal fails, close Explorer windows on all PCs that are browsing that share, then retry:

```powershell
.\scripts\windows\repair-thumbnail-cache.ps1 -RemoveThumbsDbInPath "Z:\SharedModels" -RestartExplorer
```

## Stronger cache disabling option

If network-only disabling is not enough, there is also a stronger current-user option:

```powershell
.\scripts\windows\repair-thumbnail-cache.ps1 -DisableAllThumbsDb
```

Use this only if you really want to reduce thumbnail cache file creation. It may make folders slower to open because Windows has to regenerate thumbnails more often.

## Current limitation

This branch currently improves the Windows build/package/repair path. A full modern Windows Explorer thumbnail provider installer is still a separate milestone. The next Windows code milestone is to make CLI output atomic so failed renders do not leave half-written thumbnail files behind.
