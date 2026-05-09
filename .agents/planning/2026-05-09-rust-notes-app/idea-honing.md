# Idea Honing

## Question 1

What should the app treat as a "note" when importing from the folder: one HTML file per note, a folder per note with assets, or both?

## Answer 1

Each note is always represented by a folder. That folder contains the note title and an `index.html` file with the actual content.

## Question 2

After the initial import, should the app keep reading notes directly from that original folder on disk, or should it copy them into an app-managed data directory and work from there?

## Answer 2

The app should not copy note data. It should directly read and manage data from the original folder on disk. External agents may write to that folder separately, and the app itself is viewer-only.

## Question 3

When external agents update the folder, should the app detect changes automatically and refresh the note list/content live, or is a manual refresh action enough?

## Answer 3

The app should use a manual refresh button rather than live automatic updates. It should also provide back and forward navigation buttons similar to a browser.

## Question 4

What should be used as the note title in the left sidebar: the folder name, a value read from inside the note folder, or the HTML document title from `index.html`?

## Answer 4

The app should use the folder name as the note title in the left sidebar.

## Question 5

If a note folder is missing `index.html` or contains broken HTML/assets, should the app hide that note entirely, show it in the list with an error state, or fail the whole refresh?

## Answer 5

The app should hide invalid notes from the UI. If a note folder is missing `index.html` or has broken HTML/assets, the app should record the problem in an error log that the app owner can inspect separately for debugging.

## Question 6

Do you want to commit to WebKit for rendering now, or should the design compare options and leave the final browser framework choice open until research?

## Answer 6

The app should use WebKit for rendering.

## Question 7

On first launch, how should the user choose the notes folder: a native folder picker, a config file, a command-line argument, or some combination?

## Answer 7

On first launch, the user should choose the notes folder using a native folder picker.

## Question 8

On later launches, if the configured folder is missing or inaccessible, should the app prompt the user to pick a new folder immediately, open with an empty state, or exit with an error?

## Answer 8

If a configured folder is missing, the app should prompt the user to select a new folder. The user should also be able to add more folders over time. Each configured folder becomes a topic or subject, and the notes remain grouped within that topic.

## Question 9

How should topics be presented in the UI: a separate topic list above the note list, tabs, a dropdown, or something else?

## Answer 9

Topics should be shown hierarchically in the left pane. Each topic should have an arrow to its left, similar to a code editor disclosure control. Clicking the arrow expands the topic to reveal its notes indented underneath; clicking it again collapses the topic and changes the arrow back to the collapsed right-pointing state.

## Question 10

Within each topic, how should notes be ordered in the tree: alphabetical by folder name, filesystem order, last modified time, or some custom order?

## Answer 10

Within each topic, notes should be ordered by last modified time, with the most recent note shown at the top.

## Question 11

When the user uses the back and forward buttons, should navigation move through note selections only, or also remember topic expand/collapse state and scroll position like a fuller browser history?

## Answer 11

The active topic and the open note should both be highlighted on the left side of the UI.

## Question 12

For back and forward navigation, should the app track only which note was open, or should it also restore the left-pane tree state such as expanded topic and scroll position?

## Answer 12

The app does not need to remember prior tree state or scroll position for back and forward navigation. It can simply expand the topic that contains the currently open note and highlight that note.

## Question 13

Do you want any search capability in this first version, either across topic names/note names or inside note content, or should search be out of scope?

## Answer 13

Search is out of scope for the first version.

## Clarification Checkpoint

The current clarified requirements cover note structure, folder management, multi-topic organization, refresh/navigation behavior, ordering, rendering technology, invalid note handling, and search scope.
