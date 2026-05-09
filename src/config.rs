use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::domain::TopicConfig;
use crate::paths::AppPaths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub topics: Vec<TopicConfig>,
}

impl AppConfig {
    pub fn empty() -> Self {
        Self { topics: Vec::new() }
    }

    pub fn load_or_create<F>(config_path: &Path, folder_picker: F) -> Result<Self>
    where
        F: FnOnce() -> Option<PathBuf>,
    {
        if config_path.exists() {
            let mut config = Self::load(config_path)?;
            if config.topics.is_empty() {
                config.add_initial_topic(config_path, folder_picker)?;
            }
            return Ok(config);
        }

        let Some(notes_folder) = folder_picker() else {
            return Ok(Self::empty());
        };
        let mut config = Self::empty();
        config.add_topic_folder(notes_folder);
        config.save(config_path)?;
        Ok(config)
    }

    pub fn load(config_path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;
        serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse {}", config_path.display()))
    }

    pub fn save(&self, config_path: &Path) -> Result<()> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let raw = serde_json::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(config_path, raw)
            .with_context(|| format!("failed to write {}", config_path.display()))
    }

    pub fn add_topic_folder(&mut self, path: PathBuf) -> TopicConfig {
        if let Some(existing) = self.topics.iter().find(|topic| topic.path == path) {
            return existing.clone();
        }

        let mut topic = TopicConfig::new(path);
        topic.id = unique_topic_id(&topic.id, self.topics.iter().map(|topic| topic.id.as_str()));
        self.topics.push(topic.clone());
        topic
    }

    pub fn replace_topic_path(&mut self, topic_id: &str, path: PathBuf) -> Result<()> {
        let topic = self
            .topics
            .iter_mut()
            .find(|topic| topic.id == topic_id)
            .ok_or_else(|| anyhow!("topic {topic_id} was not found"))?;
        let replacement = TopicConfig::new(path);

        topic.display_name = replacement.display_name;
        topic.path = replacement.path;
        Ok(())
    }

    fn add_initial_topic<F>(&mut self, config_path: &Path, folder_picker: F) -> Result<()>
    where
        F: FnOnce() -> Option<PathBuf>,
    {
        let Some(notes_folder) = folder_picker() else {
            return Ok(());
        };
        self.add_topic_folder(notes_folder);
        self.save(config_path)
    }
}

pub fn default_config_path() -> Result<PathBuf> {
    Ok(AppPaths::detect()?.config_file)
}

impl TopicConfig {
    pub fn new(path: PathBuf) -> Self {
        let display_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Imported Topic")
            .to_string();
        let id = slugify_topic_id(&display_name);

        Self {
            id,
            display_name,
            path,
        }
    }
}

fn slugify_topic_id(display_name: &str) -> String {
    let slug = display_name
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
        String::from("topic")
    } else {
        trimmed.to_string()
    }
}

fn unique_topic_id<'a>(base_id: &str, existing_ids: impl Iterator<Item = &'a str>) -> String {
    let existing_ids = existing_ids.collect::<Vec<_>>();
    if !existing_ids.contains(&base_id) {
        return base_id.to_string();
    }

    for suffix in 2.. {
        let candidate = format!("{base_id}-{suffix}");
        if !existing_ids.contains(&candidate.as_str()) {
            return candidate;
        }
    }

    unreachable!("unbounded suffix loop should always return")
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::tempdir;

    #[test]
    fn persists_selected_folder_on_first_run() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("config.json");
        let notes_dir = temp.path().join("notes");
        fs::create_dir_all(&notes_dir).unwrap();

        let config = AppConfig::load_or_create(&config_path, || Some(notes_dir.clone())).unwrap();
        let reloaded = AppConfig::load(&config_path).unwrap();

        assert_eq!(config, reloaded);
        assert_eq!(config.topics.len(), 1);
        assert_eq!(config.topics[0].path, notes_dir);
    }

    #[test]
    fn prompts_when_existing_config_has_no_topics() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("config.json");
        let notes_dir = temp.path().join("notes");
        fs::create_dir_all(&notes_dir).unwrap();
        AppConfig::empty().save(&config_path).unwrap();

        let config = AppConfig::load_or_create(&config_path, || Some(notes_dir.clone())).unwrap();

        assert_eq!(config.topics.len(), 1);
        assert_eq!(config.topics[0].path, notes_dir);
        assert_eq!(AppConfig::load(&config_path).unwrap(), config);
    }

    #[test]
    fn cancelled_first_run_returns_empty_config_without_creating_file() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("config.json");

        let config = AppConfig::load_or_create(&config_path, || None).unwrap();

        assert!(config.topics.is_empty());
        assert!(!config_path.exists());
    }

    #[test]
    fn cancelled_empty_config_setup_returns_empty_config_and_keeps_file_unchanged() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("config.json");
        AppConfig::empty().save(&config_path).unwrap();
        let original = fs::read_to_string(&config_path).unwrap();

        let config = AppConfig::load_or_create(&config_path, || None).unwrap();

        assert!(config.topics.is_empty());
        assert_eq!(fs::read_to_string(&config_path).unwrap(), original);
    }

    #[test]
    fn adds_topics_with_stable_unique_ids() {
        let mut config = AppConfig::empty();

        let first = config.add_topic_folder(PathBuf::from("/tmp/Notes"));
        let second = config.add_topic_folder(PathBuf::from("/var/Notes"));
        let duplicate = config.add_topic_folder(PathBuf::from("/tmp/Notes"));

        assert_eq!(first.id, "notes");
        assert_eq!(second.id, "notes-2");
        assert_eq!(duplicate.id, "notes");
        assert_eq!(config.topics.len(), 2);
    }

    #[test]
    fn replaces_topic_path_without_changing_topic_id() {
        let mut config = AppConfig::empty();
        let original = config.add_topic_folder(PathBuf::from("/tmp/Old Topic"));

        config
            .replace_topic_path(&original.id, PathBuf::from("/tmp/New Topic"))
            .unwrap();

        assert_eq!(config.topics[0].id, original.id);
        assert_eq!(config.topics[0].display_name, "New Topic");
        assert_eq!(config.topics[0].path, PathBuf::from("/tmp/New Topic"));
    }

    #[test]
    fn creates_topic_id_from_folder_name() {
        let topic = TopicConfig::new(PathBuf::from("/tmp/My Topic"));

        assert_eq!(topic.display_name, "My Topic");
        assert_eq!(topic.id, "my-topic");
    }
}
