use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use scraper::{Html, Selector};

use crate::diagnostics;
use crate::domain::{NoteEntry, ScanWarning, TopicAvailability, TopicConfig, TopicSnapshot};

#[derive(Debug, Clone)]
pub struct TopicScanner {
    diagnostics_log_path: Option<PathBuf>,
}

impl TopicScanner {
    pub fn new(diagnostics_log_path: Option<PathBuf>) -> Self {
        Self {
            diagnostics_log_path,
        }
    }

    pub fn scan_all(&self, topics: &[TopicConfig]) -> Vec<TopicSnapshot> {
        topics.iter().map(|topic| self.scan_topic(topic)).collect()
    }

    pub fn scan_topic(&self, topic: &TopicConfig) -> TopicSnapshot {
        let mut notes = Vec::new();
        let mut scan_warnings = Vec::new();
        let mut availability = TopicAvailability::Available;

        if !topic.path.is_dir() {
            availability = TopicAvailability::Missing;
            self.log_warning(
                &mut scan_warnings,
                topic.path.clone(),
                String::from("topic folder is missing"),
            );
            return TopicSnapshot {
                topic: topic.clone(),
                notes,
                scan_warnings,
                availability,
            };
        }

        match fs::read_dir(&topic.path) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            if !is_html_file(&path) {
                                continue;
                            }

                            match build_note_entry(topic, &path) {
                                Ok(note) => notes.push(note),
                                Err(message) => {
                                    self.log_warning(
                                        &mut scan_warnings,
                                        path,
                                        format!("invalid note file: {message}"),
                                    );
                                }
                            }
                        }
                        Err(error) => {
                            self.log_warning(
                                &mut scan_warnings,
                                topic.path.clone(),
                                format!("failed to read topic entry: {error}"),
                            );
                        }
                    }
                }
            }
            Err(error) => {
                self.log_warning(
                    &mut scan_warnings,
                    topic.path.clone(),
                    format!("failed to scan topic folder: {error}"),
                );
            }
        }

        sort_notes_newest_first(&mut notes);

        TopicSnapshot {
            topic: topic.clone(),
            notes,
            scan_warnings,
            availability,
        }
    }

    fn log_warning(&self, warnings: &mut Vec<ScanWarning>, path: PathBuf, message: String) {
        tracing::warn!(path = %path.display(), "{message}");

        if let Some(log_path) = &self.diagnostics_log_path {
            let _ = diagnostics::append_line(log_path, &format!("{}: {message}", path.display()));
        }

        warnings.push(ScanWarning { path, message });
    }
}

fn build_note_entry(
    topic: &TopicConfig,
    html_path: &Path,
) -> std::result::Result<NoteEntry, String> {
    if !is_html_file(html_path) {
        return Err(String::from("not an HTML file"));
    }

    let source = fs::read_to_string(html_path).map_err(|error| error.to_string())?;
    let metadata = fs::metadata(html_path).map_err(|error| error.to_string())?;
    let modified_at = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let title = note_title_from_html(html_path, &source);
    let id = note_id(&topic.id, html_path);

    Ok(NoteEntry {
        id,
        topic_id: topic.id.clone(),
        title,
        folder_path: html_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf(),
        index_html_path: html_path.to_path_buf(),
        modified_at,
    })
}

fn is_html_file(path: &Path) -> bool {
    path.is_file()
        && matches!(
            path.extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("html" | "htm")
        )
}

fn note_title_from_html(path: &Path, source: &str) -> String {
    let document = Html::parse_document(source);

    first_non_empty_text(&document, "title")
        .or_else(|| first_non_empty_text(&document, "h1"))
        .unwrap_or_else(|| note_title_from_file(path))
}

fn first_non_empty_text(document: &Html, selector: &str) -> Option<String> {
    let selector = Selector::parse(selector).ok()?;
    document
        .select(&selector)
        .map(|element| element.text().collect::<String>())
        .map(|text| text.trim().to_string())
        .find(|text| !text.is_empty())
}

fn note_title_from_file(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Untitled note")
        .to_string()
}

fn note_id(topic_id: &str, html_path: &Path) -> String {
    let file_stem = html_path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("note");

    format!(
        "{topic_id}:{}",
        slugify_note_id(file_stem)
    )
}

fn slugify_note_id(title: &str) -> String {
    let slug = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = slug.trim_matches('-');

    if trimmed.is_empty() {
        String::from("note")
    } else {
        trimmed.to_string()
    }
}

fn sort_notes_newest_first(notes: &mut [NoteEntry]) {
    notes.sort_by(|a, b| {
        b.modified_at
            .cmp(&a.modified_at)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
            .then_with(|| a.folder_path.cmp(&b.folder_path))
    });
}

#[allow(dead_code)]
pub fn read_rendered_note(note: &NoteEntry) -> Result<String> {
    let source = fs::read_to_string(&note.index_html_path)
        .with_context(|| format!("failed to read note {}", note.index_html_path.display()))?;

    Ok(inject_base_href(&source, &note.index_html_path))
}

fn inject_base_href(source: &str, note_path: &Path) -> String {
    let base_dir = note_path.parent().unwrap_or_else(|| Path::new("."));
    let base_href = format!(
        r#"<base href="{}">"#,
        url::Url::from_directory_path(base_dir)
            .expect("note directory should always convert to a file URL")
    );

    if let Some(head_index) = source.find("<head>") {
        let insertion = head_index + "<head>".len();
        let mut rendered = source.to_owned();
        rendered.insert_str(insertion, &base_href);
        return rendered;
    }

    format!("<head>{base_href}</head>{source}")
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::tempdir;

    #[test]
    fn derives_note_title_from_html_title() {
        let title = note_title_from_html(
            Path::new("/tmp/topic/fallback.html"),
            "<html><head><title>Document Title</title></head><body><h1>Heading</h1></body></html>",
        );

        assert_eq!(title, "Document Title");
    }

    #[test]
    fn derives_note_title_from_first_h1_when_title_is_missing() {
        let title = note_title_from_html(
            Path::new("/tmp/topic/fallback.html"),
            "<html><body><h1>First Heading</h1><h1>Second Heading</h1></body></html>",
        );

        assert_eq!(title, "First Heading");
    }

    #[test]
    fn derives_note_title_from_file_name_when_html_has_no_title_or_h1() {
        let title = note_title_from_html(Path::new("/tmp/topic/fallback.html"), "<html></html>");

        assert_eq!(title, "fallback");
    }

    #[test]
    fn validates_note_html_file() {
        let temp = tempdir().unwrap();
        let topic_path = temp.path().join("Topic");
        let topic = TopicConfig::new(topic_path.clone());
        let note = topic_path.join("valid-note.html");
        fs::create_dir_all(&topic_path).unwrap();
        fs::write(
            &note,
            "<html><head><title>Valid Note</title></head><body></body></html>",
        )
        .unwrap();

        let entry = build_note_entry(&topic, &note).unwrap();

        assert_eq!(entry.title, "Valid Note");
        assert_eq!(entry.folder_path, topic_path);
        assert_eq!(entry.index_html_path, note);
    }

    #[test]
    fn rejects_non_html_note_files() {
        let temp = tempdir().unwrap();
        let topic = TopicConfig::new(temp.path().join("Topic"));
        let note = temp.path().join("Topic").join("invalid.txt");
        fs::create_dir_all(note.parent().unwrap()).unwrap();
        fs::write(&note, "not html").unwrap();

        let error = build_note_entry(&topic, &note).unwrap_err();

        assert_eq!(error, "not an HTML file");
    }

    #[test]
    fn sorts_notes_by_folder_modified_time_descending() {
        let mut notes = vec![
            note_with_time("old", "Old", SystemTime::UNIX_EPOCH),
            note_with_time(
                "new",
                "New",
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(10),
            ),
        ];

        sort_notes_newest_first(&mut notes);

        assert_eq!(notes[0].id, "new");
        assert_eq!(notes[1].id, "old");
    }

    #[test]
    fn scans_direct_html_files_and_ignores_directories() {
        let temp = tempdir().unwrap();
        let topic_path = temp.path().join("Topic");
        let valid_path = topic_path.join("valid-note.html");
        let ignored_folder = topic_path.join("Ignored Folder");
        let log_path = temp.path().join("diagnostics.log");
        fs::create_dir_all(&ignored_folder).unwrap();
        fs::write(&valid_path, "<html><body><h1>Valid Note</h1></body></html>").unwrap();
        fs::write(topic_path.join("ignored.txt"), "not html").unwrap();

        let scanner = TopicScanner::new(Some(log_path.clone()));
        let snapshot = scanner.scan_topic(&TopicConfig::new(topic_path.clone()));

        assert_eq!(snapshot.notes.len(), 1);
        assert_eq!(snapshot.notes[0].title, "Valid Note");
        assert!(snapshot.scan_warnings.is_empty());
        assert!(!log_path.exists());
    }

    #[test]
    fn scans_multiple_topics_without_failing_on_missing_topic() {
        let temp = tempdir().unwrap();
        let topic_path = temp.path().join("Topic");
        let valid_path = topic_path.join("valid-note.html");
        fs::create_dir_all(&topic_path).unwrap();
        fs::write(&valid_path, "<html></html>").unwrap();

        let topics = vec![
            TopicConfig::new(topic_path),
            TopicConfig::new(temp.path().join("Missing Topic")),
        ];

        let snapshots = TopicScanner::new(None).scan_all(&topics);

        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].notes.len(), 1);
        assert_eq!(snapshots[1].notes.len(), 0);
        assert_eq!(snapshots[1].scan_warnings.len(), 1);
        assert_eq!(snapshots[1].availability, TopicAvailability::Missing);
    }

    #[test]
    fn repeated_scans_reflect_filesystem_mutations() {
        let temp = tempdir().unwrap();
        let topic_path = temp.path().join("Topic");
        let first_note_path = topic_path.join("first-note.html");
        let second_note_path = topic_path.join("second-note.html");
        fs::create_dir_all(&topic_path).unwrap();
        fs::write(&first_note_path, "<html><title>First Note</title></html>").unwrap();

        let topic = TopicConfig::new(topic_path);
        let scanner = TopicScanner::new(None);

        let first_scan = scanner.scan_all(std::slice::from_ref(&topic));
        assert_eq!(
            note_titles(&first_scan[0]),
            vec![String::from("First Note")]
        );

        fs::write(&second_note_path, "<html><title>Second Note</title></html>").unwrap();
        fs::remove_file(&first_note_path).unwrap();

        let second_scan = scanner.scan_all(&[topic]);
        assert_eq!(
            note_titles(&second_scan[0]),
            vec![String::from("Second Note")]
        );
        assert!(second_scan[0].scan_warnings.is_empty());
    }

    #[test]
    fn injects_base_href_for_relative_assets() {
        let note_path = PathBuf::from("/tmp/example/note/index.html");
        let rendered = inject_base_href("<html><head></head><body></body></html>", &note_path);

        assert!(rendered.contains("<base href=\"file:///tmp/example/note/\">"));
    }

    fn note_with_time(id: &str, title: &str, modified_at: SystemTime) -> NoteEntry {
        NoteEntry {
            id: id.to_string(),
            topic_id: String::from("topic"),
            title: title.to_string(),
            folder_path: PathBuf::from("/tmp"),
            index_html_path: PathBuf::from(format!("/tmp/{id}.html")),
            modified_at,
        }
    }

    fn note_titles(snapshot: &TopicSnapshot) -> Vec<String> {
        snapshot
            .notes
            .iter()
            .map(|note| note.title.clone())
            .collect()
    }
}
