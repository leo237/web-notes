# Implementation Plan

## Checklist

- [ ] Step 1: Bootstrap the macOS app shell, configuration model, and diagnostics plumbing
- [ ] Step 2: Implement topic configuration and first-run folder selection
- [x] Step 3: Implement topic scanning and valid-note discovery
- [ ] Step 4: Render the hierarchical topic and note tree in the left pane
- [ ] Step 5: Integrate WebKit note rendering in the right pane
- [ ] Step 6: Add refresh and browser-style note navigation
- [ ] Step 7: Harden missing-folder recovery, error presentation, and release packaging

Convert the design into a series of implementation steps that will build each component in a test-driven manner following agile best practices. Each step must result in a working, demoable increment of functionality. Prioritize best practices, incremental progress, and early testing, ensuring no big jumps in complexity at any stage. Make sure that each step builds on the previous steps, and ends with wiring things together. There should be no hanging or orphaned code that isn't integrated into a previous step.

## Step 1: Bootstrap the macOS app shell, configuration model, and diagnostics plumbing

Objective:

Establish the application crate structure, main event loop, base window, split layout scaffold, and file-based diagnostics setup.

Implementation guidance:

- Create the Rust application project structure and dependencies for the native shell, logging, and configuration path resolution.
- Create the main window with placeholder left and right panes so the app launches into a visible shell immediately.
- Define the core domain structs such as `TopicConfig`, `NoteEntry`, and `AppState` even if they are initially populated with placeholder data.
- Initialize persistent logging to an app-owned log location.

Test requirements:

- Add unit tests for configuration path resolution and any pure helper functions introduced in bootstrap.
- Add a smoke test or launch validation where practical to verify the application initializes without panicking.
- Verify that log initialization creates or can write to the expected file location.

How it integrates with previous work:

- This is the foundation step; it converts the design into a runnable app shell that later steps extend rather than replace.

Demo:

- Launch the app and see a native macOS window with a visible two-pane layout and functioning diagnostics/log-file setup.

## Step 2: Implement topic configuration and first-run folder selection

Objective:

Allow the app to persist configured topic folders and prompt the user with a native folder picker on first launch.

Implementation guidance:

- Implement loading and saving of configured topic folders in the app config directory.
- Detect an empty configuration on startup and trigger the native folder picker flow.
- Add “add topic folder” behavior so the configured folder list can grow over time.
- Surface configured topics in temporary placeholder form in the left pane before note scanning is added.

Test requirements:

- Add unit tests for config serialization/deserialization and topic add/replace behavior.
- Add integration tests using temporary config files to verify first-run empty config and persisted reload behavior.
- Verify cancellation behavior from the picker does not corrupt configuration state.

How it integrates with previous work:

- Builds directly on the app shell and diagnostics plumbing from Step 1 and replaces placeholder topic state with persisted topic configuration.

Demo:

- On first launch, the app prompts for a folder. After selection, the folder is saved, and subsequent launches restore the configured topics without re-prompting unless configuration is empty.

## Step 3: Implement topic scanning and valid-note discovery

Objective:

Scan configured topic folders, discover valid note folders, exclude invalid ones, and sort notes by last modified time.

Implementation guidance:

- Implement the topic scanner that enumerates child directories under each topic folder.
- Validate note folders by checking for `index.html`.
- Create in-memory `NoteEntry` values for valid notes only.
- Sort notes descending by folder modified time.
- Log invalid note folders and scan failures without failing the whole refresh.

Test requirements:

- Add unit tests for note validation, folder-name title derivation, and sorting semantics.
- Add integration tests using temporary directory fixtures with multiple topics and a mix of valid/invalid note folders.
- Verify invalid notes are absent from the returned tree data and present in diagnostics logs.

How it integrates with previous work:

- Replaces placeholder left-pane topic content with filesystem-derived topic and note data while preserving the persisted topic configuration model from Step 2.

Demo:

- Launch the app, select a topic folder, and see its valid note folders discovered and ordered newest-first in application state, with invalid folders omitted and logged.

## Step 4: Render the hierarchical topic and note tree in the left pane

Objective:

Turn scanned topic/note data into the intended expandable tree UI with selection and highlighting.

Implementation guidance:

- Render topics with disclosure arrows and notes indented below expanded topics.
- Implement expand/collapse behavior for topics.
- Implement active-topic and active-note highlighting.
- Ensure selecting a note updates application state and auto-expands the note’s topic if needed.
- Preserve or sensibly reconcile expansion state across refreshes where possible, without implementing historical tree restoration.

Test requirements:

- Add unit tests for tree-state transitions such as toggle, note selection, and auto-expansion for the active note.
- Add UI/behavior tests where possible for disclosure and highlighting logic.
- Verify that note ordering shown in the tree matches the scanner’s descending modified-time order.

How it integrates with previous work:

- Wires the scanner output from Step 3 into the visible left pane and replaces any placeholder topic rendering introduced earlier.

Demo:

- The left pane behaves like a code-editor tree: topics expand and collapse, notes appear indented beneath them, and the active topic and note are visibly highlighted.

## Step 5: Integrate WebKit note rendering in the right pane

Objective:

Load and display the selected note’s `index.html` in an embedded WebKit view.

Implementation guidance:

- Embed the WebKit-backed renderer into the right pane using the chosen Rust integration stack.
- Load the selected note’s on-disk `index.html` file into the renderer.
- Ensure relative assets resolve correctly from the note folder.
- Add a non-fatal in-pane error state for renderer failures and log those failures.

Test requirements:

- Add integration tests for note-selection-to-render-target resolution where it can be tested without full UI automation.
- Add fixture-based verification for correct resolution of `index.html` paths and relative asset assumptions.
- Manually verify successful rendering of sample HTML notes and graceful fallback on broken content.

How it integrates with previous work:

- Extends the selection behavior from Step 4 so note clicks now produce visible content in the right pane instead of state changes only.

Demo:

- Select a note in the tree and see its HTML content render in the right pane, including local relative assets where present.

## Step 6: Add refresh and browser-style note navigation

Objective:

Implement the toolbar controls for refresh, back, and forward, and connect them to note selection and rendering.

Implementation guidance:

- Add toolbar or header controls for refresh, back, and forward.
- Implement refresh as a full rescan of configured topics while keeping the app responsive.
- Implement navigation history over note selections.
- On back/forward, restore the current note by reselecting it and expanding only the containing topic as needed.
- Ensure refresh reconciles selection state when notes disappear or reorder.

Test requirements:

- Add unit tests for history stack behavior and selection reconciliation after refresh.
- Add integration tests for refresh after filesystem mutations in temporary fixtures.
- Manually verify that back, forward, and refresh behave predictably and do not require full tree-state restoration.

How it integrates with previous work:

- Builds on the scanner, tree UI, and renderer so the app now supports its core browser-style interaction model end to end.

Demo:

- Open several notes, navigate back and forward between them, modify source folders externally, click refresh, and see the tree and current selection update correctly.

## Step 7: Harden missing-folder recovery, error presentation, and release packaging

Objective:

Complete the production-readiness work for missing topic recovery, user-visible fallback states, and macOS release packaging across both architectures.

Implementation guidance:

- Detect missing configured topic folders at startup and refresh, and prompt the user to select replacement folders.
- Add clear user-facing empty/error states for no topics configured, canceled replacement, or no valid notes in a topic.
- Finalize log locations and retention behavior.
- Add release build instructions or scripts for `aarch64-apple-darwin` and `x86_64-apple-darwin`, including universal-binary assembly if desired.
- Verify the final crate/dependency stack against the chosen minimum macOS deployment target.

Test requirements:

- Add integration tests for missing-folder replacement behavior and canceled replacement flows.
- Add verification for user-facing fallback states when a topic has no valid notes.
- Validate release packaging steps on both target architectures as far as the local environment allows.

How it integrates with previous work:

- Hardens the end-to-end viewer built in Steps 1 through 6 without changing the core architecture, ensuring the app is ready for real usage and distribution.

Demo:

- Launch the app with a missing configured topic, reselect a replacement folder successfully, view correct fallback states when topics are empty or invalid, and produce release artifacts for both Intel and Apple Silicon.
