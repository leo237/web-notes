# Release Packaging

The app is packaged as a standalone macOS `.app` bundle. It does not need to be launched through a wrapper script.

## macOS Targets

The release script builds both supported Rust macOS targets:

- `aarch64-apple-darwin` for Apple Silicon
- `x86_64-apple-darwin` for Intel Macs

The bundle declares macOS 13.0 as the minimum deployment target. Override it for a release build with:

```bash
MACOSX_DEPLOYMENT_TARGET=13.0 scripts/package-macos.sh
```

## Build

```bash
scripts/package-macos.sh
```

Outputs are written under `target/macos-release`:

- `Web Notes-aarch64-apple-darwin.app`
- `Web Notes-x86_64-apple-darwin.app`
- `Web Notes-universal.app` when `lipo` is available

## Diagnostics

Runtime diagnostics are written to the app-owned data directory resolved by `directories::ProjectDirs`, under:

```text
diagnostics/web-notes.log
```

The log records startup, invalid note folders, missing topic folders, canceled replacements, scan failures, and note render failures.
