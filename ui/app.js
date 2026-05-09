const topicsList = document.querySelector("#topics-list");
const noteTitle = document.querySelector("#note-title");
const noteFrame = document.querySelector("#note-frame");
const viewerEmpty = document.querySelector("#viewer-empty");
const addTopicButton = document.querySelector("#add-topic");
const refreshButton = document.querySelector("#refresh-topics");
const backButton = document.querySelector("#navigate-back");
const forwardButton = document.querySelector("#navigate-forward");

const AUTO_REFRESH_INTERVAL_MS = 5000;

const state = {
  topics: [],
  expandedTopicIds: new Set(),
  selectedTopicId: null,
  selectedNoteId: null,
  historyBack: [],
  historyForward: [],
  isRefreshing: false,
};

async function loadTopics(options = {}) {
  const { recoverMissing = true } = options;
  const endpoint = recoverMissing ? "api/topics" : "api/topics/snapshot";
  const response = await fetch(`app://app/${endpoint}`);
  if (!response.ok) {
    throw new Error(`Failed to load topics: ${response.status}`);
  }

  return response.json();
}

async function addTopic() {
  addTopicButton.disabled = true;
  try {
    const response = await fetch("app://app/api/topics/add");
    if (!response.ok) {
      throw new Error(`Failed to add topic: ${response.status}`);
    }
    applyTopics(await response.json());
  } finally {
    addTopicButton.disabled = false;
  }
}

async function refreshTopics() {
  setRefreshing(true);
  try {
    applyTopics(await loadTopics());
  } finally {
    setRefreshing(false);
  }
}

async function autoRefreshTopics() {
  if (state.isRefreshing) {
    return;
  }

  try {
    setRefreshing(true);
    applyTopics(await loadTopics({ recoverMissing: false }));
  } catch (error) {
    console.error("Automatic topic refresh failed", error);
  } finally {
    setRefreshing(false);
  }
}

function applyTopics(topics) {
  state.topics = topics;
  reconcileTreeState();
  renderTree();
}

function reconcileTreeState() {
  const topicIds = new Set(state.topics.map((snapshot) => snapshot.topic.id));
  const noteIds = new Set(
    state.topics.flatMap((snapshot) => (snapshot.notes ?? []).map((note) => note.id)),
  );
  state.expandedTopicIds = new Set(
    [...state.expandedTopicIds].filter((topicId) => topicIds.has(topicId)),
  );
  state.historyBack = state.historyBack.filter((noteId) => noteIds.has(noteId));
  state.historyForward = state.historyForward.filter((noteId) => noteIds.has(noteId));

  const selectedNote = findNote(state.selectedNoteId);
  if (selectedNote) {
    state.selectedTopicId = selectedNote.topic.id;
    state.expandedTopicIds.add(selectedNote.topic.id);
    return;
  }

  if (topicIds.has(state.selectedTopicId)) {
    state.selectedNoteId = firstNoteIdForTopic(state.selectedTopicId);
    if (state.selectedNoteId) {
      state.expandedTopicIds.add(state.selectedTopicId);
    }
    return;
  }

  const firstTopic = state.topics[0]?.topic;
  const firstNote = state.topics[0]?.notes?.[0];
  state.selectedTopicId = firstTopic?.id ?? null;
  state.selectedNoteId = firstNote?.id ?? null;

  if (firstTopic) {
    state.expandedTopicIds.add(firstTopic.id);
  }
}

function renderTree() {
  topicsList.replaceChildren();

  if (state.topics.length === 0) {
    const empty = document.createElement("p");
    empty.className = "empty-state";
    empty.textContent = "No topic folders are configured. Add a folder to begin.";
    topicsList.append(empty);
    noteTitle.textContent = "No topics";
    clearNotePreview("Add a topic folder to preview notes.");
    updateToolbar();
    return;
  }

  state.topics.forEach((snapshot) => {
    const topic = snapshot.topic;
    const notes = snapshot.notes ?? [];
    const warnings = snapshot.scan_warnings ?? [];
    const isExpanded = state.expandedTopicIds.has(topic.id);
    const isSelectedTopic = state.selectedTopicId === topic.id;

    topicsList.append(renderTopicRow(snapshot, isExpanded, isSelectedTopic));

    if (isExpanded) {
      if (notes.length === 0) {
        topicsList.append(renderTopicEmptyRow(snapshot));
      } else {
        notes.forEach((note) => {
          topicsList.append(renderNoteRow(topic, note));
        });
      }
    }
  });

  renderSelectionDetails();
  updateToolbar();
}

function renderTopicRow(snapshot, isExpanded, isSelectedTopic) {
  const topic = snapshot.topic;
  const notes = snapshot.notes ?? [];
  const warnings = snapshot.scan_warnings ?? [];
  const row = document.createElement("div");
  row.className = "tree-row topic-row";
  row.classList.toggle("is-active-topic", isSelectedTopic);
  row.title = topic.path;

  const disclosure = document.createElement("button");
  disclosure.type = "button";
  disclosure.className = "disclosure-button";
  disclosure.classList.toggle("is-expanded", isExpanded);
  disclosure.setAttribute("aria-label", `${isExpanded ? "Collapse" : "Expand"} ${topic.display_name}`);
  disclosure.setAttribute("aria-expanded", String(isExpanded));
  disclosure.addEventListener("click", () => {
    toggleTopic(topic.id);
  });

  const topicButton = document.createElement("button");
  topicButton.type = "button";
  topicButton.className = "tree-label topic-label";
  topicButton.title = topic.path;
  topicButton.addEventListener("click", () => {
    selectTopic(topic.id);
  });

  const name = document.createElement("span");
  name.className = "topic-name";
  name.textContent = topic.display_name;

  const meta = document.createElement("span");
  meta.className = "topic-meta";
  meta.textContent = topicSummary(snapshot, notes, warnings);

  topicButton.append(name);
  if (meta.textContent) {
    topicButton.append(meta);
  }
  row.append(disclosure, topicButton);
  return row;
}

function renderNoteRow(topic, note) {
  const noteButton = document.createElement("button");
  noteButton.type = "button";
  noteButton.className = "tree-row note-row";
  noteButton.classList.toggle("is-active-note", state.selectedNoteId === note.id);
  noteButton.dataset.topicId = topic.id;
  noteButton.dataset.noteId = note.id;
  noteButton.addEventListener("click", () => {
    selectNote(note.id, { recordHistory: true });
  });

  const spacer = document.createElement("span");
  spacer.className = "note-indent";

  const title = document.createElement("span");
  title.className = "note-name";
  title.textContent = note.title;

  noteButton.append(spacer, title);
  return noteButton;
}

function renderTopicEmptyRow(snapshot) {
  const row = document.createElement("p");
  row.className = "tree-empty-row";
  row.textContent = topicEmptyMessage(snapshot);
  return row;
}

function topicSummary(snapshot, notes, warnings) {
  const availability = snapshot.availability ?? "available";
  if (availability === "missing" || availability === "replacement_canceled") {
    return "Missing folder";
  }

  return warnings.length > 0 ? `${warnings.length} hidden` : "";
}

function topicEmptyMessage(snapshot) {
  const availability = snapshot.availability ?? "available";
  const warnings = snapshot.scan_warnings ?? [];

  if (availability === "replacement_canceled") {
    return "Folder missing. Replacement was canceled.";
  }

  if (availability === "missing") {
    return "Folder missing. Refresh to choose a replacement.";
  }

  return warnings.length > 0
    ? "No valid notes found. Invalid folders were hidden."
    : "No valid notes found.";
}

function toggleTopic(topicId) {
  if (state.expandedTopicIds.has(topicId)) {
    state.expandedTopicIds.delete(topicId);
  } else {
    state.expandedTopicIds.add(topicId);
  }
  renderTree();
}

function selectTopic(topicId) {
  state.selectedTopicId = topicId;
  state.selectedNoteId = null;
  renderTree();
}

function selectNote(noteId, options = {}) {
  const { recordHistory = false } = options;
  const selected = findNote(noteId);
  if (!selected) {
    return;
  }

  if (recordHistory && state.selectedNoteId !== noteId) {
    if (state.selectedNoteId) {
      state.historyBack.push(state.selectedNoteId);
    }
    state.historyForward = [];
  }

  state.selectedTopicId = selected.topic.id;
  state.selectedNoteId = noteId;
  state.expandedTopicIds.add(selected.topic.id);
  renderTree();
}

function navigateBack() {
  const previousNoteId = popExistingHistoryEntry(state.historyBack);
  if (!previousNoteId) {
    updateToolbar();
    return;
  }

  if (state.selectedNoteId) {
    state.historyForward.push(state.selectedNoteId);
  }
  selectNote(previousNoteId);
}

function navigateForward() {
  const nextNoteId = popExistingHistoryEntry(state.historyForward);
  if (!nextNoteId) {
    updateToolbar();
    return;
  }

  if (state.selectedNoteId) {
    state.historyBack.push(state.selectedNoteId);
  }
  selectNote(nextNoteId);
}

function renderSelectionDetails() {
  const selectedNote = findNote(state.selectedNoteId);
  if (selectedNote) {
    noteTitle.textContent = selectedNote.note.title;
    loadNotePreview(selectedNote.note);
    return;
  }

  clearNotePreview();

  const selectedTopic = state.topics.find((snapshot) => snapshot.topic.id === state.selectedTopicId);
  if (selectedTopic) {
    const notes = selectedTopic.notes ?? [];
    const warnings = selectedTopic.scan_warnings ?? [];
    const warningSuffix = warnings.length > 0 ? `, ${warnings.length} hidden` : "";
    noteTitle.textContent = selectedTopicTitle(selectedTopic, notes, warningSuffix);
    clearNotePreview(topicEmptyMessage(selectedTopic));
    return;
  }

  noteTitle.textContent = "Choose a note";
  clearNotePreview("Choose a note to preview.");
}

function selectedTopicTitle(snapshot, notes, warningSuffix) {
  const availability = snapshot.availability ?? "available";
  if (availability === "replacement_canceled") {
    return `${snapshot.topic.display_name} unavailable`;
  }

  if (availability === "missing") {
    return `${snapshot.topic.display_name} missing`;
  }

  return `${snapshot.topic.display_name} (${notes.length} notes${warningSuffix})`;
}

function findNote(noteId) {
  if (!noteId) {
    return null;
  }

  for (const snapshot of state.topics) {
    const note = (snapshot.notes ?? []).find((candidate) => candidate.id === noteId);
    if (note) {
      return { topic: snapshot.topic, note };
    }
  }

  return null;
}

function firstNoteIdForTopic(topicId) {
  const topic = state.topics.find((snapshot) => snapshot.topic.id === topicId);
  return topic?.notes?.[0]?.id ?? null;
}

function popExistingHistoryEntry(history) {
  while (history.length > 0) {
    const noteId = history.pop();
    if (findNote(noteId)) {
      return noteId;
    }
  }
  return null;
}

function setRefreshing(isRefreshing) {
  state.isRefreshing = isRefreshing;
  updateToolbar();
}

function updateToolbar() {
  backButton.disabled = state.historyBack.length === 0;
  forwardButton.disabled = state.historyForward.length === 0;
  refreshButton.disabled = state.isRefreshing;
}

function loadNotePreview(note) {
  const nextSrc = `app://app/notes/${note.id}/index.html`;
  viewerEmpty.hidden = true;
  noteFrame.hidden = false;
  if (noteFrame.getAttribute("src") !== nextSrc) {
    noteFrame.setAttribute("src", nextSrc);
  }
}

function clearNotePreview(message = "Choose a note to preview.") {
  noteFrame.removeAttribute("src");
  noteFrame.hidden = true;
  viewerEmpty.hidden = false;
  viewerEmpty.textContent = message;
}

addTopicButton.addEventListener("click", () => {
  addTopic().catch((error) => {
    topicsList.textContent = error.message;
    noteTitle.textContent = "Unable to add topic";
    clearNotePreview("The topic folder could not be added.");
    updateToolbar();
  });
});

refreshButton.addEventListener("click", () => {
  refreshTopics().catch((error) => {
    topicsList.textContent = error.message;
    noteTitle.textContent = "Unable to refresh";
    clearNotePreview("Refresh failed. Check the diagnostics log for details.");
    setRefreshing(false);
  });
});

backButton.addEventListener("click", navigateBack);
forwardButton.addEventListener("click", navigateForward);

loadTopics()
  .then(applyTopics)
  .catch((error) => {
    topicsList.textContent = error.message;
    noteTitle.textContent = "Unable to load topics";
    clearNotePreview("Topics could not be loaded.");
    updateToolbar();
  });

clearNotePreview();
updateToolbar();
setInterval(autoRefreshTopics, AUTO_REFRESH_INTERVAL_MS);
