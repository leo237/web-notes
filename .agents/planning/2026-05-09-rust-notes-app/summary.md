# Summary

## Artifacts Created

- `rough-idea.md`
- `idea-honing.md`
- `research/framework-and-webkit.md`
- `research/filesystem-and-config.md`
- `research/packaging-and-logging.md`
- `design/detailed-design.md`
- `implementation/plan.md`

## Design Overview

The planned product is a native macOS notes viewer written in Rust. It reads note content directly from one or more configured source folders, each of which becomes a topic. Within each topic, note folders are discovered from disk, filtered to those containing `index.html`, ordered by last modified time, and rendered in a hierarchical left pane. The selected note is shown in a WebKit-based right pane.

The design intentionally keeps the first version narrow:

- Viewer-only, with no note editing or copying
- Manual refresh instead of live file watching
- Browser-style back, forward, and refresh controls
- Hidden invalid notes with owner-readable diagnostics logs
- No search in V1

## Implementation Plan Overview

The implementation plan breaks the work into seven demoable increments:

1. Bootstrap the app shell and diagnostics foundation
2. Add persisted topic configuration and first-run folder selection
3. Implement filesystem scanning and valid-note discovery
4. Build the hierarchical tree UI
5. Integrate WebKit rendering
6. Add refresh and note-navigation behavior
7. Harden missing-folder recovery and release packaging

Each step includes its own testing requirements and ends in working functionality rather than disconnected scaffolding.

## Suggested Next Steps

- Begin implementation with Step 1 from `implementation/plan.md`.
- Decide the exact crate stack for the native shell around the WebKit renderer before coding begins.
- Choose and document the minimum supported macOS version during project bootstrap.

## Areas That May Need Further Refinement

- The exact Rust shell/layout library paired with `wry`
- The precise strategy for loading local HTML in the embedded WebKit view while preserving relative asset resolution
- The final release-packaging workflow for universal macOS distribution
