use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TopicConfig {
    pub id: String,
    pub display_name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NoteEntry {
    pub id: String,
    pub topic_id: String,
    pub title: String,
    pub folder_path: PathBuf,
    pub index_html_path: PathBuf,
    pub modified_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ScanWarning {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TopicSnapshot {
    pub topic: TopicConfig,
    pub notes: Vec<NoteEntry>,
    pub scan_warnings: Vec<ScanWarning>,
    pub availability: TopicAvailability,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TopicAvailability {
    Available,
    Missing,
    ReplacementCanceled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub struct AppState {
    pub topics: Vec<TopicSnapshot>,
    pub selected_topic_id: Option<String>,
    pub selected_note_id: Option<String>,
    pub expanded_topic_ids: BTreeSet<String>,
    pub history_back: Vec<String>,
    pub history_forward: Vec<String>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl AppState {
    pub fn from_topics(topics: Vec<TopicSnapshot>) -> Self {
        let selected_topic_id = topics.first().map(|topic| topic.topic.id.clone());
        let selected_note_id = topics
            .first()
            .and_then(|topic| topic.notes.first())
            .map(|note| note.id.clone());
        let expanded_topic_ids = selected_topic_id.iter().cloned().collect();

        Self {
            topics,
            selected_topic_id,
            selected_note_id,
            expanded_topic_ids,
            history_back: Vec::new(),
            history_forward: Vec::new(),
        }
    }

    pub fn from_topic_configs(topics: Vec<TopicConfig>) -> Self {
        Self::from_topics(
            topics
                .into_iter()
                .map(|topic| TopicSnapshot {
                    topic,
                    notes: Vec::new(),
                    scan_warnings: Vec::new(),
                    availability: TopicAvailability::Available,
                })
                .collect(),
        )
    }

    pub fn toggle_topic(&mut self, topic_id: &str) {
        if self.expanded_topic_ids.contains(topic_id) {
            self.expanded_topic_ids.remove(topic_id);
        } else if self.topic_exists(topic_id) {
            self.expanded_topic_ids.insert(topic_id.to_string());
        }
    }

    pub fn select_topic(&mut self, topic_id: &str) {
        if self.topic_exists(topic_id) {
            self.selected_topic_id = Some(topic_id.to_string());
            self.selected_note_id = None;
        }
    }

    pub fn select_note(&mut self, note_id: &str) {
        self.select_note_with_history(note_id, true);
    }

    pub fn navigate_back(&mut self) -> bool {
        let Some(note_id) = self.history_back.pop() else {
            return false;
        };

        if !self.note_exists(&note_id) {
            return self.navigate_back();
        }

        if let Some(current_note_id) = self.selected_note_id.clone() {
            self.history_forward.push(current_note_id);
        }

        self.select_note_with_history(&note_id, false);
        true
    }

    pub fn navigate_forward(&mut self) -> bool {
        let Some(note_id) = self.history_forward.pop() else {
            return false;
        };

        if !self.note_exists(&note_id) {
            return self.navigate_forward();
        }

        if let Some(current_note_id) = self.selected_note_id.clone() {
            self.history_back.push(current_note_id);
        }

        self.select_note_with_history(&note_id, false);
        true
    }

    fn select_note_with_history(&mut self, note_id: &str, record_history: bool) {
        if let Some(topic_id) = self.topic_id_for_note(note_id).map(str::to_string) {
            if record_history && self.selected_note_id.as_deref() != Some(note_id) {
                if let Some(current_note_id) = self.selected_note_id.clone() {
                    self.history_back.push(current_note_id);
                }
                self.history_forward.clear();
            }

            self.selected_topic_id = Some(topic_id.clone());
            self.selected_note_id = Some(note_id.to_string());
            self.expanded_topic_ids.insert(topic_id);
        }
    }

    pub fn apply_refresh(&mut self, topics: Vec<TopicSnapshot>) {
        self.topics = topics;
        self.expanded_topic_ids
            .retain(|topic_id| self.topics.iter().any(|topic| topic.topic.id == *topic_id));
        self.prune_history();

        let selected_note_still_exists = self
            .selected_note_id
            .as_deref()
            .and_then(|note_id| self.topic_id_for_note(note_id))
            .map(str::to_string);

        if let (Some(note_id), Some(topic_id)) =
            (self.selected_note_id.clone(), selected_note_still_exists)
        {
            self.selected_topic_id = Some(topic_id.clone());
            self.expanded_topic_ids.insert(topic_id);
            self.selected_note_id = Some(note_id);
            return;
        }

        if let Some(topic_id) = self.selected_topic_id.clone() {
            if self.topic_exists(&topic_id) {
                self.selected_note_id = self.first_note_id_for_topic(&topic_id);
                if self.selected_note_id.is_some() {
                    self.expanded_topic_ids.insert(topic_id);
                }
                return;
            }
        }

        let replacement = Self::from_topics(self.topics.clone());
        self.selected_topic_id = replacement.selected_topic_id;
        self.selected_note_id = replacement.selected_note_id;
        self.expanded_topic_ids
            .extend(replacement.expanded_topic_ids);
    }

    fn topic_exists(&self, topic_id: &str) -> bool {
        self.topics.iter().any(|topic| topic.topic.id == topic_id)
    }

    fn note_exists(&self, note_id: &str) -> bool {
        self.topic_id_for_note(note_id).is_some()
    }

    fn topic_id_for_note(&self, note_id: &str) -> Option<&str> {
        self.topics.iter().find_map(|topic| {
            topic
                .notes
                .iter()
                .any(|note| note.id == note_id)
                .then_some(topic.topic.id.as_str())
        })
    }

    fn first_note_id_for_topic(&self, topic_id: &str) -> Option<String> {
        self.topics
            .iter()
            .find(|topic| topic.topic.id == topic_id)
            .and_then(|topic| topic.notes.first())
            .map(|note| note.id.clone())
    }

    fn prune_history(&mut self) {
        let note_ids: BTreeSet<String> = self
            .topics
            .iter()
            .flat_map(|topic| topic.notes.iter().map(|note| note.id.clone()))
            .collect();

        self.history_back
            .retain(|note_id| note_ids.contains(note_id));
        self.history_forward
            .retain(|note_id| note_ids.contains(note_id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_first_topic_and_note_when_state_is_built() {
        let topic = TopicSnapshot {
            topic: TopicConfig {
                id: String::from("topic-a"),
                display_name: String::from("Topic A"),
                path: PathBuf::from("/tmp/topic-a"),
            },
            notes: vec![NoteEntry {
                id: String::from("note-a"),
                topic_id: String::from("topic-a"),
                title: String::from("Note A"),
                folder_path: PathBuf::from("/tmp/topic-a/note-a"),
                index_html_path: PathBuf::from("/tmp/topic-a/note-a/index.html"),
                modified_at: SystemTime::UNIX_EPOCH,
            }],
            scan_warnings: Vec::new(),
            availability: TopicAvailability::Available,
        };

        let state = AppState::from_topics(vec![topic]);

        assert_eq!(state.selected_topic_id.as_deref(), Some("topic-a"));
        assert_eq!(state.selected_note_id.as_deref(), Some("note-a"));
        assert!(state.expanded_topic_ids.contains("topic-a"));
    }

    #[test]
    fn builds_placeholder_state_from_configured_topics() {
        let state = AppState::from_topic_configs(vec![TopicConfig {
            id: String::from("topic-a"),
            display_name: String::from("Topic A"),
            path: PathBuf::from("/tmp/topic-a"),
        }]);

        assert_eq!(state.topics.len(), 1);
        assert_eq!(state.selected_topic_id.as_deref(), Some("topic-a"));
        assert_eq!(state.selected_note_id, None);
        assert!(state.expanded_topic_ids.contains("topic-a"));
    }

    #[test]
    fn toggles_existing_topics_only() {
        let mut state = AppState::from_topics(vec![topic_with_notes("topic-a", &[])]);

        assert!(state.expanded_topic_ids.contains("topic-a"));

        state.toggle_topic("topic-a");
        assert!(!state.expanded_topic_ids.contains("topic-a"));

        state.toggle_topic("missing");
        assert!(!state.expanded_topic_ids.contains("missing"));

        state.toggle_topic("topic-a");
        assert!(state.expanded_topic_ids.contains("topic-a"));
    }

    #[test]
    fn selecting_note_sets_topic_and_expands_parent() {
        let mut state = AppState::from_topics(vec![
            topic_with_notes("topic-a", &["note-a"]),
            topic_with_notes("topic-b", &["note-b"]),
        ]);
        state.expanded_topic_ids.remove("topic-b");

        state.select_note("note-b");

        assert_eq!(state.selected_topic_id.as_deref(), Some("topic-b"));
        assert_eq!(state.selected_note_id.as_deref(), Some("note-b"));
        assert!(state.expanded_topic_ids.contains("topic-b"));
    }

    #[test]
    fn selecting_topic_clears_active_note() {
        let mut state = AppState::from_topics(vec![topic_with_notes("topic-a", &["note-a"])]);

        state.select_topic("topic-a");

        assert_eq!(state.selected_topic_id.as_deref(), Some("topic-a"));
        assert_eq!(state.selected_note_id, None);
    }

    #[test]
    fn refresh_preserves_surviving_note_selection_and_expansion() {
        let mut state = AppState::from_topics(vec![
            topic_with_notes("topic-a", &["note-a"]),
            topic_with_notes("topic-b", &["note-b"]),
        ]);
        state.select_note("note-b");
        state.expanded_topic_ids.remove("topic-a");

        state.apply_refresh(vec![topic_with_notes("topic-b", &["note-b"])]);

        assert_eq!(state.selected_topic_id.as_deref(), Some("topic-b"));
        assert_eq!(state.selected_note_id.as_deref(), Some("note-b"));
        assert!(state.expanded_topic_ids.contains("topic-b"));
        assert!(!state.expanded_topic_ids.contains("topic-a"));
    }

    #[test]
    fn refresh_selects_first_available_note_when_selection_disappears() {
        let mut state = AppState::from_topics(vec![topic_with_notes("topic-a", &["note-a"])]);

        state.apply_refresh(vec![topic_with_notes("topic-b", &["note-b"])]);

        assert_eq!(state.selected_topic_id.as_deref(), Some("topic-b"));
        assert_eq!(state.selected_note_id.as_deref(), Some("note-b"));
        assert!(state.expanded_topic_ids.contains("topic-b"));
    }

    #[test]
    fn note_selection_records_back_history_and_clears_forward_history() {
        let mut state = AppState::from_topics(vec![topic_with_notes(
            "topic-a",
            &["note-a", "note-b", "note-c"],
        )]);

        state.select_note("note-b");
        state.navigate_back();
        state.select_note("note-c");

        assert_eq!(state.selected_note_id.as_deref(), Some("note-c"));
        assert_eq!(state.history_back, vec![String::from("note-a")]);
        assert!(state.history_forward.is_empty());
    }

    #[test]
    fn back_and_forward_reselect_notes_and_expand_parent_topic() {
        let mut state = AppState::from_topics(vec![
            topic_with_notes("topic-a", &["note-a"]),
            topic_with_notes("topic-b", &["note-b"]),
        ]);
        state.select_note("note-b");
        state.expanded_topic_ids.remove("topic-a");

        assert!(state.navigate_back());
        assert_eq!(state.selected_topic_id.as_deref(), Some("topic-a"));
        assert_eq!(state.selected_note_id.as_deref(), Some("note-a"));
        assert!(state.expanded_topic_ids.contains("topic-a"));

        assert!(state.navigate_forward());
        assert_eq!(state.selected_topic_id.as_deref(), Some("topic-b"));
        assert_eq!(state.selected_note_id.as_deref(), Some("note-b"));
        assert!(state.expanded_topic_ids.contains("topic-b"));
    }

    #[test]
    fn refresh_prunes_history_entries_for_removed_notes() {
        let mut state = AppState::from_topics(vec![topic_with_notes(
            "topic-a",
            &["note-a", "note-b", "note-c"],
        )]);
        state.select_note("note-b");
        state.select_note("note-c");

        state.apply_refresh(vec![topic_with_notes("topic-a", &["note-c"])]);

        assert_eq!(state.selected_note_id.as_deref(), Some("note-c"));
        assert!(state.history_back.is_empty());
        assert!(state.history_forward.is_empty());
    }

    #[test]
    fn refresh_replaces_removed_selected_note_with_first_note_in_same_topic() {
        let mut state =
            AppState::from_topics(vec![topic_with_notes("topic-a", &["note-a", "note-b"])]);
        state.select_note("note-b");

        state.apply_refresh(vec![topic_with_notes("topic-a", &["note-a"])]);

        assert_eq!(state.selected_topic_id.as_deref(), Some("topic-a"));
        assert_eq!(state.selected_note_id.as_deref(), Some("note-a"));
        assert!(state.expanded_topic_ids.contains("topic-a"));
    }

    fn topic_with_notes(topic_id: &str, note_ids: &[&str]) -> TopicSnapshot {
        TopicSnapshot {
            topic: TopicConfig {
                id: topic_id.to_string(),
                display_name: topic_id.to_string(),
                path: PathBuf::from(format!("/tmp/{topic_id}")),
            },
            notes: note_ids
                .iter()
                .map(|note_id| NoteEntry {
                    id: (*note_id).to_string(),
                    topic_id: topic_id.to_string(),
                    title: (*note_id).to_string(),
                    folder_path: PathBuf::from(format!("/tmp/{topic_id}/{note_id}")),
                    index_html_path: PathBuf::from(format!("/tmp/{topic_id}/{note_id}/index.html")),
                    modified_at: SystemTime::UNIX_EPOCH,
                })
                .collect(),
            scan_warnings: Vec::new(),
            availability: TopicAvailability::Available,
        }
    }
}
