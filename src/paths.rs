use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use directories::ProjectDirs;

const QUALIFIER: &str = "dev";
const ORGANIZATION: &str = "openai";
const APPLICATION: &str = "web-notes";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub diagnostics_dir: PathBuf,
    pub diagnostics_log: PathBuf,
}

impl AppPaths {
    pub fn detect() -> Result<Self> {
        let project_dirs = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
            .ok_or_else(|| anyhow!("unable to determine application directories"))?;
        Ok(Self::from_dirs(
            project_dirs.config_dir(),
            project_dirs.data_local_dir(),
        ))
    }

    pub fn from_dirs(config_dir: &Path, data_dir: &Path) -> Self {
        let config_dir = config_dir.to_path_buf();
        let diagnostics_dir = data_dir.join("diagnostics");

        Self {
            config_file: config_dir.join("config.json"),
            diagnostics_log: diagnostics_dir.join("web-notes.log"),
            config_dir,
            diagnostics_dir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_config_and_diagnostics_paths() {
        let paths = AppPaths::from_dirs(
            Path::new("/tmp/support/config"),
            Path::new("/tmp/support/data"),
        );

        assert_eq!(paths.config_dir, PathBuf::from("/tmp/support/config"));
        assert_eq!(
            paths.config_file,
            PathBuf::from("/tmp/support/config/config.json")
        );
        assert_eq!(
            paths.diagnostics_dir,
            PathBuf::from("/tmp/support/data/diagnostics")
        );
        assert_eq!(
            paths.diagnostics_log,
            PathBuf::from("/tmp/support/data/diagnostics/web-notes.log")
        );
    }
}
