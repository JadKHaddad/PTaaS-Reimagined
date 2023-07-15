use std::path::PathBuf;
use thiserror::Error as ThisError;
use tokio::fs;
pub struct LocalProjectManager {
    root_dir: PathBuf,
}

#[derive(ThisError, Debug)]
pub enum LocalProjectManagerCreateError {
    #[error("Could not check if root dir exists: {0}")]
    CouldNotCheckIfRootDirExists(#[source] std::io::Error),
    #[error("Could not create root dir: {0}")]
    CouldNotCreateRootDir(#[source] std::io::Error),
}

impl LocalProjectManager {
    pub async fn new(root_dir: PathBuf) -> Result<Self, LocalProjectManagerCreateError> {
        if !fs::try_exists(&root_dir)
            .await
            .map_err(LocalProjectManagerCreateError::CouldNotCheckIfRootDirExists)?
        {
            tracing::info!(?root_dir, "Root dir does not exist, creating it");
            fs::create_dir_all(&root_dir)
                .await
                .map_err(LocalProjectManagerCreateError::CouldNotCreateRootDir)?;
        }

        Ok(Self { root_dir })
    }
}
