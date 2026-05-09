use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use tao::dpi::LogicalSize;
use tao::event::{ElementState, Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::keyboard::{KeyCode, ModifiersState};
use tao::window::{Fullscreen, Window, WindowBuilder};
use wry::WebViewBuilder;
use wry::http::header::CONTENT_TYPE;
use wry::http::{Response, StatusCode};

use crate::config::{AppConfig, default_config_path};
use crate::diagnostics::Diagnostics;
use crate::domain::{NoteEntry, TopicAvailability, TopicConfig, TopicSnapshot};
use crate::notes::TopicScanner;
use crate::paths::AppPaths;

const INDEX_HTML: &str = include_str!("../ui/index.html");
const APP_JS: &str = include_str!("../ui/app.js");
const APP_CSS: &str = include_str!("../ui/styles.css");

pub fn run() -> Result<()> {
    install_app_menu();

    let paths = AppPaths::detect()?;
    let diagnostics = Diagnostics::init(&paths.diagnostics_log)?;
    tracing::info!(log_path = %diagnostics.log_path().display(), "starting web-notes");

    let config_path = default_config_path()?;
    let config = AppConfig::load_or_create(&config_path, || rfd::FileDialog::new().pick_folder())?;
    let topic_registry = Arc::new(TopicRegistry::new(
        config_path,
        config,
        TopicScanner::new(Some(diagnostics.log_path().to_path_buf())),
        diagnostics.log_path().to_path_buf(),
    ));
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Web Notes")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .build(&event_loop)
        .context("failed to create application window")?;

    window.set_title("Web Notes");

    let protocol_topics = Arc::clone(&topic_registry);
    let _webview = WebViewBuilder::new()
        .with_custom_protocol(String::from("app"), move |_id, request| {
            handle_request(request.uri().path(), &protocol_topics)
                .unwrap_or_else(|error| error_response(&error.to_string()))
        })
        .with_url("app://app/index.html")
        .build(&window)
        .context("failed to create webview")?;

    let mut modifiers = ModifiersState::default();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {}
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::ModifiersChanged(next_modifiers),
                ..
            } => modifiers = next_modifiers,
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event, .. },
                ..
            } if should_handle_command_shortcut(&event, modifiers) => match event.physical_key {
                KeyCode::KeyQ => *control_flow = ControlFlow::Exit,
                KeyCode::KeyX => *control_flow = ControlFlow::Exit,
                KeyCode::KeyF if modifiers.shift_key() => toggle_fullscreen(&window),
                _ => {}
            },
            _ => {}
        }
    })
}

#[cfg(target_os = "macos")]
fn install_app_menu() {
    use std::ffi::CString;

    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send, sel};

    const NS_EVENT_MODIFIER_FLAG_SHIFT: usize = 1 << 17;
    const NS_EVENT_MODIFIER_FLAG_COMMAND: usize = 1 << 20;

    unsafe fn ns_string(value: &str) -> *mut AnyObject {
        let value = CString::new(value).expect("menu strings must not contain nul bytes");
        unsafe { msg_send![class!(NSString), stringWithUTF8String: value.as_ptr()] }
    }

    unsafe fn menu_item(title: &str, action: objc2::runtime::Sel, key: &str) -> *mut AnyObject {
        unsafe {
            let item: *mut AnyObject = msg_send![class!(NSMenuItem), alloc];
            msg_send![
                item,
                initWithTitle: ns_string(title),
                action: action,
                keyEquivalent: ns_string(key)
            ]
        }
    }

    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let main_menu: *mut AnyObject = msg_send![class!(NSMenu), new];

        let app_menu_item: *mut AnyObject = msg_send![class!(NSMenuItem), new];
        let app_menu: *mut AnyObject = msg_send![class!(NSMenu), new];
        let quit_q = menu_item("Quit Web Notes", sel!(terminate:), "q");
        let quit_x = menu_item("Quit Web Notes", sel!(terminate:), "x");
        let command_mask = NS_EVENT_MODIFIER_FLAG_COMMAND;
        let _: () = msg_send![quit_q, setKeyEquivalentModifierMask: command_mask];
        let _: () = msg_send![quit_x, setKeyEquivalentModifierMask: command_mask];
        let _: () = msg_send![app_menu, addItem: quit_q];
        let _: () = msg_send![app_menu, addItem: quit_x];
        let _: () = msg_send![app_menu_item, setSubmenu: app_menu];
        let _: () = msg_send![main_menu, addItem: app_menu_item];

        let view_menu_item: *mut AnyObject = msg_send![class!(NSMenuItem), new];
        let view_menu: *mut AnyObject = msg_send![class!(NSMenu), new];
        let fullscreen = menu_item("Enter Full Screen", sel!(toggleFullScreen:), "f");
        let fullscreen_mask = NS_EVENT_MODIFIER_FLAG_COMMAND | NS_EVENT_MODIFIER_FLAG_SHIFT;
        let _: () = msg_send![fullscreen, setKeyEquivalentModifierMask: fullscreen_mask];
        let _: () = msg_send![view_menu, addItem: fullscreen];
        let _: () = msg_send![view_menu_item, setSubmenu: view_menu];
        let _: () = msg_send![main_menu, addItem: view_menu_item];

        let _: () = msg_send![app, setMainMenu: main_menu];
    }
}

#[cfg(not(target_os = "macos"))]
fn install_app_menu() {}

fn should_handle_command_shortcut(event: &tao::event::KeyEvent, modifiers: ModifiersState) -> bool {
    event.state == ElementState::Pressed && !event.repeat && modifiers.super_key()
}

fn toggle_fullscreen(window: &Window) {
    if window.fullscreen().is_some() {
        window.set_fullscreen(None);
    } else {
        window.set_fullscreen(Some(Fullscreen::Borderless(None)));
    }
}

#[derive(Debug)]
struct TopicRegistry {
    config_path: PathBuf,
    config: Mutex<AppConfig>,
    scanner: TopicScanner,
    diagnostics_log_path: PathBuf,
}

impl TopicRegistry {
    fn new(
        config_path: PathBuf,
        config: AppConfig,
        scanner: TopicScanner,
        diagnostics_log_path: PathBuf,
    ) -> Self {
        Self {
            config_path,
            config: Mutex::new(config),
            scanner,
            diagnostics_log_path,
        }
    }

    fn snapshots(&self) -> Result<Vec<TopicSnapshot>> {
        let topics = self.lock_config()?.topics.clone();
        Ok(self.scanner.scan_all(&topics))
    }

    fn snapshots_with_missing_recovery<F>(&self, mut folder_picker: F) -> Result<Vec<TopicSnapshot>>
    where
        F: FnMut(&TopicConfig) -> Option<PathBuf>,
    {
        let mut config = self.lock_config()?;
        let mut changed = false;
        let mut canceled_topic_ids = BTreeSet::new();

        let missing_topics = config
            .topics
            .iter()
            .filter(|topic| !topic.path.is_dir())
            .cloned()
            .collect::<Vec<_>>();

        for topic in missing_topics {
            tracing::warn!(
                topic_id = %topic.id,
                path = %topic.path.display(),
                "configured topic folder is missing"
            );
            let _ = crate::diagnostics::append_line(
                &self.diagnostics_log_path,
                &format!(
                    "configured topic folder is missing: {} ({})",
                    topic.display_name,
                    topic.path.display()
                ),
            );

            match folder_picker(&topic) {
                Some(replacement_path) => {
                    config.replace_topic_path(&topic.id, replacement_path)?;
                    changed = true;
                    tracing::info!(
                        topic_id = %topic.id,
                        "replaced missing topic folder"
                    );
                }
                None => {
                    canceled_topic_ids.insert(topic.id.clone());
                    tracing::warn!(topic_id = %topic.id, "missing topic replacement was canceled");
                }
            }
        }

        if changed {
            config.save(&self.config_path)?;
        }

        let topics = config.topics.clone();
        drop(config);

        let mut snapshots = self.scanner.scan_all(&topics);
        for snapshot in &mut snapshots {
            if canceled_topic_ids.contains(&snapshot.topic.id)
                && snapshot.availability == TopicAvailability::Missing
            {
                snapshot.availability = TopicAvailability::ReplacementCanceled;
            }
        }

        Ok(snapshots)
    }

    fn add_topic_with_picker<F>(&self, folder_picker: F) -> Result<Vec<TopicSnapshot>>
    where
        F: FnOnce() -> Option<PathBuf>,
    {
        let Some(folder) = folder_picker() else {
            return self.snapshots();
        };

        let mut config = self.lock_config()?;
        let topic = config.add_topic_folder(folder);
        tracing::info!(topic_id = %topic.id, path = %topic.path.display(), "configured topic folder");
        config.save(&self.config_path)?;
        let topics = config.topics.clone();
        drop(config);

        Ok(self.scanner.scan_all(&topics))
    }

    fn lock_config(&self) -> Result<std::sync::MutexGuard<'_, AppConfig>> {
        self.config
            .lock()
            .map_err(|_| anyhow!("topic configuration lock was poisoned"))
    }

    fn resolve_note_asset(&self, note_id: &str, asset_path: &str) -> Result<ResolvedNoteAsset> {
        match resolve_note_asset(&self.snapshots()?, note_id, asset_path) {
            Ok(asset) => Ok(asset),
            Err(error) => {
                self.log_render_failure(note_id, asset_path, &error);
                Err(error)
            }
        }
    }

    fn log_render_failure(&self, note_id: &str, asset_path: &str, error: &anyhow::Error) {
        tracing::warn!(note_id, asset_path, error = %error, "failed to render note asset");
        let _ = crate::diagnostics::append_line(
            &self.diagnostics_log_path,
            &format!("render failure for {note_id}/{asset_path}: {error}"),
        );
    }
}

fn handle_request(path: &str, topics: &TopicRegistry) -> Result<Response<Cow<'static, [u8]>>> {
    match path {
        "/index.html" | "/" => ok_html(INDEX_HTML),
        "/app.js" => ok_bytes(APP_JS.as_bytes(), "text/javascript"),
        "/styles.css" => ok_bytes(APP_CSS.as_bytes(), "text/css"),
        "/api/topics" => {
            let body = serde_json::to_vec(&topics.snapshots_with_missing_recovery(|_| {
                rfd::FileDialog::new()
                    .set_title("Select replacement topic folder")
                    .pick_folder()
            })?)
            .context("failed to serialize topic list")?;
            ok_bytes(&body, "application/json")
        }
        "/api/topics/snapshot" => {
            let body = serde_json::to_vec(&topics.snapshots()?)
                .context("failed to serialize topic list")?;
            ok_bytes(&body, "application/json")
        }
        "/api/topics/add" => {
            let body = serde_json::to_vec(
                &topics.add_topic_with_picker(|| rfd::FileDialog::new().pick_folder())?,
            )
            .context("failed to serialize topic list")?;
            ok_bytes(&body, "application/json")
        }
        _ if path.starts_with("/notes/") => note_response(path, topics),
        _ => response(StatusCode::NOT_FOUND, "text/plain", b"Not found"),
    }
}

fn note_response(path: &str, topics: &TopicRegistry) -> Result<Response<Cow<'static, [u8]>>> {
    let (note_id, asset_path) = parse_note_route(path)?;
    let asset = topics.resolve_note_asset(note_id, asset_path)?;
    let body = fs::read(&asset.path)
        .with_context(|| format!("failed to read note asset {}", asset.path.display()))?;

    ok_bytes(&body, content_type_for_path(&asset.path))
}

fn ok_html(body: &str) -> Result<Response<Cow<'static, [u8]>>> {
    ok_bytes(body.as_bytes(), "text/html")
}

fn ok_bytes(body: &[u8], content_type: &str) -> Result<Response<Cow<'static, [u8]>>> {
    response(StatusCode::OK, content_type, body)
}

fn response(
    status: StatusCode,
    content_type: &str,
    body: &[u8],
) -> Result<Response<Cow<'static, [u8]>>> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, content_type)
        .body(Cow::Owned(body.to_vec()))
        .context("failed to build HTTP response")
}

fn error_response(message: &str) -> Response<Cow<'static, [u8]>> {
    let html = format!(
        "<!doctype html><html><body><h1>Unable to render note</h1><p>{}</p></body></html>",
        escape_html(message)
    );

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(CONTENT_TYPE, "text/html")
        .body(Cow::Owned(html.into_bytes()))
        .expect("failed to build fallback error response")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedNoteAsset {
    path: PathBuf,
}

fn parse_note_route(path: &str) -> Result<(&str, &str)> {
    let tail = path
        .strip_prefix("/notes/")
        .ok_or_else(|| anyhow!("invalid note route"))?;
    let (note_id, asset_path) = tail.split_once('/').unwrap_or((tail, "index.html"));

    if note_id.is_empty() {
        return Err(anyhow!("missing note id"));
    }

    let asset_path = if asset_path.is_empty() {
        "index.html"
    } else {
        asset_path
    };

    Ok((note_id, asset_path))
}

fn resolve_note_asset(
    topics: &[TopicSnapshot],
    note_id: &str,
    asset_path: &str,
) -> Result<ResolvedNoteAsset> {
    let note = find_note(topics, note_id).ok_or_else(|| anyhow!("note {note_id} was not found"))?;
    let relative_path = safe_relative_asset_path(asset_path)?;
    let candidate = if relative_path == PathBuf::from("index.html") {
        note.index_html_path.clone()
    } else {
        note.folder_path.join(relative_path)
    };

    if !candidate.is_file() {
        return Err(anyhow!("note asset {} was not found", candidate.display()));
    }

    ensure_asset_is_inside_note_folder(note, &candidate)?;

    Ok(ResolvedNoteAsset { path: candidate })
}

fn find_note<'a>(
    topics: &'a [crate::domain::TopicSnapshot],
    note_id: &str,
) -> Option<&'a NoteEntry> {
    topics
        .iter()
        .flat_map(|topic| topic.notes.iter())
        .find(|note| note.id == note_id)
}

fn safe_relative_asset_path(asset_path: &str) -> Result<PathBuf> {
    let path = Path::new(asset_path);
    if path.is_absolute() {
        return Err(anyhow!("absolute note asset paths are not allowed"));
    }

    let mut safe_path = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => safe_path.push(segment),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(anyhow!("note asset path may not escape the note folder"));
            }
        }
    }

    if safe_path.as_os_str().is_empty() {
        Ok(PathBuf::from("index.html"))
    } else {
        Ok(safe_path)
    }
}

fn ensure_asset_is_inside_note_folder(note: &NoteEntry, asset_path: &Path) -> Result<()> {
    let note_folder = note.folder_path.canonicalize().with_context(|| {
        format!(
            "failed to resolve note folder {}",
            note.folder_path.display()
        )
    })?;
    let asset = asset_path
        .canonicalize()
        .with_context(|| format!("failed to resolve note asset {}", asset_path.display()))?;

    if asset.starts_with(&note_folder) {
        Ok(())
    } else {
        Err(anyhow!("note asset path may not escape the note folder"))
    }
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "text/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn escape_html(message: &str) -> String {
    message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::SystemTime;

    use crate::config::AppConfig;
    use crate::domain::{TopicConfig, TopicSnapshot};
    use tempfile::tempdir;

    #[test]
    fn parses_note_route_with_default_index_asset() {
        let (note_id, asset_path) = parse_note_route("/notes/topic-a:note-a").unwrap();

        assert_eq!(note_id, "topic-a:note-a");
        assert_eq!(asset_path, "index.html");
    }

    #[test]
    fn resolves_index_html_for_selected_note() {
        let temp = tempdir().unwrap();
        let topic_folder = temp.path().join("Topic");
        let note_path = topic_folder.join("note.html");
        fs::create_dir_all(&topic_folder).unwrap();
        fs::write(&note_path, "<html></html>").unwrap();

        let topics = vec![snapshot_for_note("topic:note", note_path.clone())];
        let resolved = resolve_note_asset(&topics, "topic:note", "index.html").unwrap();

        assert_eq!(resolved.path, note_path);
    }

    #[test]
    fn resolves_relative_note_assets_from_note_folder() {
        let temp = tempdir().unwrap();
        let topic_folder = temp.path().join("Topic");
        let note_path = topic_folder.join("note.html");
        fs::create_dir_all(topic_folder.join("assets")).unwrap();
        fs::write(&note_path, "<html></html>").unwrap();
        fs::write(topic_folder.join("assets").join("style.css"), "body {}").unwrap();

        let topics = vec![snapshot_for_note("topic:note", note_path.clone())];
        let resolved = resolve_note_asset(&topics, "topic:note", "assets/style.css").unwrap();

        assert_eq!(
            resolved.path,
            topic_folder.join("assets").join("style.css")
        );
    }

    #[test]
    fn rejects_note_asset_paths_that_escape_note_folder() {
        let temp = tempdir().unwrap();
        let topic_folder = temp.path().join("Topic");
        let note_path = topic_folder.join("note.html");
        fs::create_dir_all(&topic_folder).unwrap();
        fs::write(&note_path, "<html></html>").unwrap();

        let topics = vec![snapshot_for_note("topic:note", note_path)];
        let error = resolve_note_asset(&topics, "topic:note", "../other.html").unwrap_err();

        assert!(error.to_string().contains("may not escape"));
    }

    #[test]
    fn replaces_missing_topic_folder_before_scanning() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("config.json");
        let replacement = temp.path().join("Replacement Topic");
        fs::create_dir_all(&replacement).unwrap();
        fs::write(replacement.join("valid-note.html"), "<html></html>").unwrap();

        let mut config = AppConfig::empty();
        let topic = config.add_topic_folder(temp.path().join("Missing Topic"));
        config.save(&config_path).unwrap();
        let registry = TopicRegistry::new(
            config_path.clone(),
            config,
            TopicScanner::new(None),
            temp.path().join("web-notes.log"),
        );

        let snapshots = registry
            .snapshots_with_missing_recovery(|_| Some(replacement.clone()))
            .unwrap();

        assert_eq!(snapshots[0].topic.id, topic.id);
        assert_eq!(snapshots[0].topic.path, replacement);
        assert_eq!(snapshots[0].notes.len(), 1);
        assert_eq!(
            AppConfig::load(&config_path).unwrap().topics[0].id,
            topic.id
        );
    }

    #[test]
    fn marks_missing_topic_when_replacement_is_canceled() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("config.json");
        let missing = temp.path().join("Missing Topic");
        let mut config = AppConfig::empty();
        config.add_topic_folder(missing.clone());
        config.save(&config_path).unwrap();
        let registry = TopicRegistry::new(
            config_path.clone(),
            config,
            TopicScanner::new(None),
            temp.path().join("web-notes.log"),
        );

        let snapshots = registry.snapshots_with_missing_recovery(|_| None).unwrap();

        assert_eq!(
            snapshots[0].availability,
            TopicAvailability::ReplacementCanceled
        );
        assert_eq!(
            AppConfig::load(&config_path).unwrap().topics[0].path,
            missing
        );
    }

    fn snapshot_for_note(note_id: &str, note_path: PathBuf) -> TopicSnapshot {
        TopicSnapshot {
            topic: TopicConfig {
                id: String::from("topic"),
                display_name: String::from("Topic"),
                path: note_path.parent().unwrap().to_path_buf(),
            },
            notes: vec![NoteEntry {
                id: note_id.to_string(),
                topic_id: String::from("topic"),
                title: String::from("Note"),
                folder_path: note_path.parent().unwrap().to_path_buf(),
                index_html_path: note_path,
                modified_at: SystemTime::UNIX_EPOCH,
            }],
            scan_warnings: Vec::new(),
            availability: TopicAvailability::Available,
        }
    }
}
