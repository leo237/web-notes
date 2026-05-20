# Web Notes

Web Notes is a small macOS desktop app for browsing locally stored HTML notes. It scans one or more topic folders, lists valid notes in a left sidebar, and renders the selected note in an embedded WebKit-backed webview.

The app is written in Rust with [`tao`](https://crates.io/crates/tao) for the native window and [`wry`](https://crates.io/crates/wry) for the embedded browser view. The UI is plain HTML, CSS, and JavaScript bundled into the binary at compile time.

## Features

- Native macOS desktop window with a two-panel notes browser.
- First-run folder picker for selecting an initial notes topic.
- Add additional topic folders from the app.
- Automatic scanning of topic folders for direct HTML note files.
- Automatic sidebar refresh so newly added notes appear without manual refresh.
- Embedded rendering of each selected HTML note.
- Support for relative note assets such as CSS, JavaScript, images, icons, fonts, and JSON.
- Back and forward navigation between recently selected notes.
- Manual refresh action for immediate re-scanning.
- Missing-folder recovery by prompting for a replacement folder.
- Diagnostics log for startup, scan, missing-folder, and render failures.
- Release packaging script for Apple Silicon, Intel, and universal macOS app bundles.

## Note File Format

Web Notes expects notes to be grouped by topic folder. Each direct `.html` or `.htm` file in a topic folder is treated as one note.

```text
My Topic/
  project-plan.html
  meeting-notes.html
  styles.css
  chart.js
  images/
    overview.png
  Scratch/
    draft.txt
```

In this example:

- `project-plan.html` is a valid note.
- `meeting-notes.html` is a valid note.
- `Scratch/` is ignored because only direct HTML files are scanned as notes.
- `draft.txt` is ignored because it is not an HTML file.

The note title is read from the HTML content in this order:

1. The document `<title>`.
2. The first non-empty `<h1>`.
3. The HTML file name without its extension.

Notes are sorted newest first by the HTML file's modified time, then by title, then by file path.

### Assets

Note assets are served relative to the selected HTML file's containing folder through the app's custom protocol. A note can reference local sibling files normally:

```html
<!doctype html>
<html>
  <head>
    <link rel="stylesheet" href="styles.css" />
  </head>
  <body>
    <img src="images/overview.png" alt="Overview" />
    <script src="chart.js"></script>
  </body>
</html>
```

Supported content types include:

- HTML
- CSS
- JavaScript
- JSON
- PNG, JPEG, GIF, SVG, WebP, ICO
- WOFF and WOFF2 fonts

Asset paths are constrained to the selected note file's containing folder. Absolute paths and paths that escape with `..` are rejected.

## Requirements

- macOS 13.0 or newer for packaged releases.
- Rust toolchain with Cargo.
- macOS WebKit support available through `wry`.

For release packaging, install both Rust macOS targets:

```bash
rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin
```

The packaging script also calls `rustup target add` automatically.

## Development

Build the app:

```bash
cargo build
```

Run the app:

```bash
cargo run
```

Run tests:

```bash
cargo test
```

Format the code:

```bash
cargo fmt
```

Check the project:

```bash
cargo check
```

## First Run

On first launch, Web Notes asks you to select a folder. The selected folder becomes the first topic folder and is persisted in the app config.

If the picker is canceled, the app starts with no configured topics. You can add a topic later with the `+` button in the sidebar.

## App Data

Web Notes uses `directories::ProjectDirs` with:

```text
qualifier:    dev
organization: openai
application:  web-notes
```

The config file is stored under the platform config directory:

```text
config.json
```

The config contains the topic registry:

```json
{
  "topics": [
    {
      "id": "my-topic",
      "display_name": "My Topic",
      "path": "/Users/example/Notes/My Topic"
    }
  ]
}
```

Topic IDs are derived from folder names and made unique with numeric suffixes when needed. Replacing a missing topic folder keeps the existing topic ID stable.

## Diagnostics

Runtime diagnostics are written under the app-owned local data directory:

```text
diagnostics/web-notes.log
```

The log records:

- Diagnostics startup.
- Application startup.
- Missing topic folders.
- Canceled missing-folder replacements.
- Invalid note files.
- Topic scan failures.
- Note render failures.

See [RELEASE.md](RELEASE.md) for release-specific diagnostics notes.

## macOS Packaging

Build release app bundles:

```bash
scripts/package-macos.sh
```

Outputs are written to:

```text
target/macos-release/
```

The script builds:

- `Web Notes-aarch64-apple-darwin.app`
- `Web Notes-x86_64-apple-darwin.app`
- `Web Notes-universal.app` when `lipo` is available

Override the minimum deployment target for a release build:

```bash
MACOSX_DEPLOYMENT_TARGET=13.0 scripts/package-macos.sh
```

The bundle metadata lives in [packaging/macos/Info.plist](packaging/macos/Info.plist).

## Architecture

```text
src/
  main.rs          app entry point
  app.rs           native window, webview, custom protocol, note asset serving
  config.rs        topic config loading, saving, and ID generation
  diagnostics.rs   diagnostics log initialization and append helpers
  domain.rs        shared app data types and navigation state
  notes.rs         topic scanning and note validation
  paths.rs         config and diagnostics path resolution
ui/
  index.html       bundled web UI shell
  app.js           sidebar, selection, refresh, and navigation behavior
  styles.css       app styling
```

At runtime, `src/app.rs` registers an `app://app` custom protocol. The protocol serves the bundled UI and note content:

- `app://app/index.html`
- `app://app/app.js`
- `app://app/styles.css`
- `app://app/api/topics`
- `app://app/api/topics/snapshot`
- `app://app/api/topics/add`
- `app://app/notes/{note_id}/index.html`
- `app://app/notes/{note_id}/{asset_path}`

The web UI fetches topic snapshots from the API endpoints and displays selected notes in an iframe. Internally, `app://app/notes/{note_id}/index.html` maps to the selected HTML file so the iframe URL can stay stable while notes are stored as direct files. Automatic refreshes use the non-interactive snapshot endpoint so background scans do not open folder picker dialogs.

## Current Limitations

- The app is focused on browsing existing HTML notes; it does not edit notes.
- Only direct `.html` and `.htm` files in a topic folder are scanned as notes.
- Nested folders are ignored as note entries, though they can still contain referenced assets.
- App bundles are not currently signed or notarized by the packaging script.
- The UI is currently desktop-oriented and optimized for macOS app window usage.
