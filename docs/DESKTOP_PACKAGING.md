# Desktop Packaging (LoreForge Client / LoreForge Server Admin)

Both `loreforge-client` and `loreforge-server-admin` build Windows installer
packages via CMake presets + `windeployqt`, then either of two independent
packaging tools: CPack (NSIS/ZIP) or Inno Setup. Both consume the exact same
`build-release/bin/` output, so "what ships" (Qt runtime, QML modules,
excluding the `*Tests` executable and debug symbols) never drifts between
the two — pick whichever installer tool fits your distribution needs.

## Building a release package

From either project directory, in a shell with the MSVC toolchain on PATH
(`vcvarsall.bat x64`):

```
cmake --preset release
cmake --build --preset release
```

- `cmake --preset release` configures a `Release` build in `build-release/`
  (separate from the `build/` directory the `default` Debug preset uses, so
  day-to-day debug builds are untouched).
- The build's `POST_BUILD` step runs `windeployqt` automatically, copying the
  required Qt DLLs, platform/image-format plugins, and QML modules next to
  the built `.exe` (`build-release/bin/`).
- The build also compiles `resources/app.rc`, embedding `resources/icon.ico`
  as the `.exe`'s icon (Windows Explorer/taskbar/Alt-Tab icon).

Then package with either tool:

```
# Option A: CPack (NSIS installer if makensis is present, else a ZIP)
cd build-release
cpack

# Option B: Inno Setup
ISCC.exe installer.iss
```

- CPack packages `build-release/bin/` into an NSIS installer `.exe` if
  `makensis` is available on the build machine, otherwise a `.zip` fallback.
- `installer.iss` (Inno Setup) produces a `Setup.exe`-style installer under
  `installer-output/` with Start Menu shortcuts, an uninstaller, and the app
  icon set as `SetupIconFile`/`UninstallDisplayIcon`. Requires the Inno Setup
  compiler (`ISCC.exe`) — install via `winget install JRSoftware.InnoSetup`.
  Run from the project directory (`loreforge-client/` or
  `loreforge-server-admin/`), not from `build-release/`.

## Known gap: code signing

The installers and executables produced by this pipeline are **unsigned**.
Windows SmartScreen will show an "unknown publisher" warning the first time a
user runs the installer or the app. Real public distribution should obtain a
code-signing certificate and sign both the app `.exe` and the CPack-produced
installer as a step after `cpack`, e.g.:

```
signtool.exe sign /f <cert.pfx> /p <password> /fd sha256 LoreForgeClient.exe
signtool.exe sign /f <cert.pfx> /p <password> /fd sha256 LFC-0.1.0-win64.exe
signtool.exe sign /f <cert.pfx> /p <password> /fd sha256 LoreForgeClient-Setup-0.1.0.exe
```

(or an equivalent cloud-HSM-backed signing service). No certificate exists
for this project yet, so this step is intentionally not automated.
