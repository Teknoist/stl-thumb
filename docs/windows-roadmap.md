# Windows roadmap

## Done in `windows-ci`

- Add a native GitHub Actions Windows x64 build.
- Upload a portable Windows ZIP artifact.
- Add current-user CLI install/uninstall helpers.
- Add Explorer thumbnail cache repair helper.
- Add SMB/network-share `Thumbs.db` mitigation helper.
- Document Windows build, package, repair, and SMB workflows.

## Next: prevent undeletable/half-written thumbnail outputs

Problem:

- When a thumbnail render is interrupted or fails during write, a partial output file can remain.
- On SMB/network shares, Explorer may also leave `Thumbs.db` files that block folder deletion from another PC.

Planned code change:

- Encode the image fully in memory first.
- Write to a temporary file in the same directory as the requested output.
- Flush and close the temporary file.
- Rename it into place only after the write succeeds.
- Clean up the temporary file on error.
- Never create the final output path before the render has fully succeeded.

This makes CLI thumbnail output much safer on Windows and network shares.

## Next: full Explorer thumbnail provider

The current branch focuses on the CLI/package path. A complete Windows Explorer thumbnail provider should be handled as a separate milestone:

- Decide whether to keep the old native shell-extension approach or replace it.
- Add explicit register/unregister commands or an installer.
- Add registry repair tooling for `.stl`, `.obj`, and `.3mf` thumbnail associations.
- Add tests/docs for conflicts with other thumbnail providers such as slicers or CAD tools.

## Next: Windows diagnostics

Add:

```powershell
stl-thumb doctor
```

The command should report:

- executable path
- Windows version
- GPU/OpenGL renderer
- supported file associations
- thumbnail provider registration state
- thumbnail cache policy state
- common conflict hints
