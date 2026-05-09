use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing_appender::non_blocking::WorkerGuard;

pub struct Diagnostics {
    log_path: PathBuf,
    _guard: WorkerGuard,
}

impl Diagnostics {
    pub fn init(log_path: &Path) -> Result<Self> {
        ensure_log_file(log_path)?;

        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(log_path)
            .with_context(|| format!("failed to open diagnostics log {}", log_path.display()))?;
        let (writer, guard) = tracing_appender::non_blocking(file);
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(writer)
            .with_target(true)
            .without_time()
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);

        append_line(log_path, "diagnostics initialized")?;

        Ok(Self {
            log_path: log_path.to_path_buf(),
            _guard: guard,
        })
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }
}

pub fn ensure_log_file(log_path: &Path) -> Result<()> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create diagnostics directory {}",
                parent.display()
            )
        })?;
    }

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .with_context(|| format!("failed to create diagnostics log {}", log_path.display()))?;

    Ok(())
}

pub fn append_line(log_path: &Path, message: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_path)
        .with_context(|| format!("failed to open diagnostics log {}", log_path.display()))?;
    writeln!(file, "{message}")
        .with_context(|| format!("failed to write diagnostics log {}", log_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::tempdir;

    #[test]
    fn creates_and_writes_diagnostics_log() {
        let temp = tempdir().unwrap();
        let log_path = temp.path().join("diagnostics").join("web-notes.log");

        ensure_log_file(&log_path).unwrap();
        append_line(&log_path, "startup complete").unwrap();

        let contents = fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("startup complete"));
    }
}
