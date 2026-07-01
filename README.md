# stl-thumb

[![Build Status](https://github.com/Teknoist/stl-thumb/actions/workflows/build-ci.yml/badge.svg)](https://github.com/Teknoist/stl-thumb/actions/workflows/build-ci.yml)
[![Windows CI](https://github.com/Teknoist/stl-thumb/actions/workflows/windows-ci.yml/badge.svg)](https://github.com/Teknoist/stl-thumb/actions/workflows/windows-ci.yml)
[![Documentation](https://img.shields.io/docsrs/stl-thumb/latest)](https://docs.rs/stl-thumb/latest/stl_thumb/)
[![Crates.io](https://img.shields.io/crates/v/stl-thumb.svg)](https://crates.io/crates/stl-thumb)

Stl-thumb is a fast lightweight thumbnail generator for 3D model(STL, OBJ, 3MF) files. It can show previews for model files in your file manager on Linux and Windows. It is written in Rust and uses OpenGL.

![Screenshot](https://user-images.githubusercontent.com/3131268/116009182-f3f89c80-a5cc-11eb-817d-91e8a9fad279.png)

## Windows-focused fork notes

This fork is currently improving the Windows path first:

- GitHub Actions Windows x64 build
- portable ZIP artifact
- current-user CLI install/uninstall helper scripts
- Explorer thumbnail cache repair helper
- SMB/network-share `Thumbs.db` mitigation helper
- Windows troubleshooting documentation

See [docs/windows.md](docs/windows.md) and [docs/windows-roadmap.md](docs/windows-roadmap.md).

## Installation

### Windows

Stl-thumb requires 64 bit Windows 7 or later.

For this fork, the `windows-ci` workflow produces a portable Windows x64 ZIP artifact. Download the artifact from GitHub Actions, extract it, then run:

```powershell
.\stl-thumb.exe --help
.\stl-thumb.exe C:\Models\part.stl C:\Models\part.png -s 512
```

To install the CLI for the current user:

```powershell
.\scripts\windows\install-cli.ps1 -SourceDir . -AddToPath
```

If Explorer thumbnails or network-share deletion get stuck because of cached thumbnails or `Thumbs.db`, run:

```powershell
.\scripts\windows\repair-thumbnail-cache.ps1 -ClearExplorerThumbnailCache -RestartExplorer
.\scripts\windows\repair-thumbnail-cache.ps1 -DisableNetworkThumbsDb
.\scripts\windows\repair-thumbnail-cache.ps1 -RemoveThumbsDbInPath "Z:\SharedModels"
```

The original installer may tell the Windows shell to refresh the thumbnail cache, however this does not always seem to work. If your icons do not change then try clearing the thumbnail cache.

### Linux

Stl-thumb works with Gnome and most other similar desktop environments. If you are using the KDE desktop environment then you will also need to install the separate [`stl-thumb-kde`](https://github.com/unlimitedbacon/stl-thumb-kde) package.

Make sure that your file manager is set to generate previews for files larger than 1 MB. Most file managers have this setting under the Preview tab in their Preferences.

#### Arch

A package is available [in the AUR](https://aur.archlinux.org/packages/stl-thumb/). Install it manually or using your favorite AUR helper.

```
$ yay -S stl-thumb
```

#### Debian / Ubuntu

[Download the .deb package](https://github.com/unlimitedbacon/stl-thumb/releases/latest) for your platform (usually amd64) and install it. Packages are also available for armhf (Raspberry Pi) and arm64 (Pine64 and other SBCs).

```
$ sudo apt install ./stl-thumb_0.5.0_amd64.deb
```

#### openSUSE

For openSUSE Tumbleweed there is a user repo available:

```
$ sudo zypper ar -f obs://home:jubalh:stl stl
$ sudo zypper ref
$ sudo zypper install stl-thumb
```

## Building

### Building the tool itself:
If you get errors about fontconfig being missing, install the development package

You can build the debug version with:
```
$ cargo build
```
When you are done, build the release version with:
```
$ cargo build --release
```

### Building on Windows

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

### Building the .deb-package:
```
$ cargo install cargo-deb #this is an additional dependency
$ cargo deb
```
### Building the .rpm-package:
```
$ cargo install generate-rpm #this is an additional dependency
$ cargo generate-rpm
```

## Command Line Usage

```
$ stl-thumb <MODEL_FILE> [IMG_FILE]
```

### Options

| Option        | Description                                                                                                                                                                           |
| ------------- |---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| <MODEL_FILE>    | The model file you want a picture of. Use - to read from stdin instead of a file.                                                                                                     |
| <IMG_FILE>    | The thumbnail image file that will be created. Use - to write to stdout instead of a file.                                                                                            |
| -s, --size \<size\>   | Specify width of the image. It will always be a square.                                                                                                                               |
| -f, --format \<format\> | The format of the image file. If not specified it will be determined from the file extension, or default to PNG if there is no extension. Supported formats: PNG, JPEG, GIF, ICO, BMP |
| -m, --material \<ambient\> \<diffuse\> \<specular\> | Colors for rendering the mesh using the Phong reflection model. Requires 3 colors as rgb hex values: ambient, diffuse, and specular. Defaults to blue.                                |
| -b, --background \<color> | The background color with transparency (rgba). Default is ffffff00.                                                                                                                   |
| -a, --antialiasing [none, fxaa] | Anti-aliasing method. Default is FXAA, which is fast but may introduce artifacts.                                                                                                     |
| --recalc-normals | Force recalculation of face normals. Use when dealing with malformed STL files.                                                                                                       |
| -x            | Display the image in a window instead of saving a file.                                                                                                                               |
| -h, --help    | Prints help information.                                                                                                                                                              |
| -V, --version | Prints version information.                                                                                                                                                           |
| -v[v][v]      | Increase message verbosity. Levels: Errors, Warnings, Info, Debugging                                                                                                                 |
