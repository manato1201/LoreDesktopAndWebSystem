# Desktop Packaging (LoreForge Client / LoreForge Server Admin)

Both `loreforge-client` and `loreforge-server-admin` build Windows installer
packages via CMake presets + `windeployqt` + CPack.

## Building a release package

From either project directory, in a shell with the MSVC toolchain on PATH
(`vcvarsall.bat x64`):

```
cmake --preset release
cmake --build --preset release
cd build-release
cpack
```

- `cmake --preset release` configures a `Release` build in `build-release/`
  (separate from the `build/` directory the `default` Debug preset uses, so
  day-to-day debug builds are untouched).
- The build's `POST_BUILD` step runs `windeployqt` automatically, copying the
  required Qt DLLs, platform/image-format plugins, and QML modules next to
  the built `.exe` (`build-release/bin/`).
- `cpack` packages `build-release/bin/` (minus the `*Tests` executable and
  debug symbols) into a distributable artifact — an NSIS installer `.exe` if
  `makensis` is available on the build machine, otherwise a `.zip` fallback.

## Known gap: code signing

The installers and executables produced by this pipeline are **unsigned**.
Windows SmartScreen will show an "unknown publisher" warning the first time a
user runs the installer or the app. Real public distribution should obtain a
code-signing certificate and sign both the app `.exe` and the CPack-produced
installer as a step after `cpack`, e.g.:

```
signtool.exe sign /f <cert.pfx> /p <password> /fd sha256 LoreForgeClient.exe
signtool.exe sign /f <cert.pfx> /p <password> /fd sha256 LFC-0.1.0-win64.exe
```

(or an equivalent cloud-HSM-backed signing service). No certificate exists
for this project yet, so this step is intentionally not automated.
