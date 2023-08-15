use std::{collections::HashMap, io::Error as IoError, path::PathBuf, sync::Arc};
use thiserror::Error as ThisError;
use tokio::{
    fs,
    sync::{mpsc, RwLock},
};
use tracing::info_span;

use super::local_project_installer::LocalProjectInstallerController;

// TODO: Create Traits: ProjectManager, Database, Controller

pub struct LocalProjectManager {
    root_dir: PathBuf,
    // C: impl Controller: cancel...
    controllers: Arc<RwLock<HashMap</* id */ String, LocalProjectInstallerController>>>,
    // D: impl Database: save, remove, get...
}

#[derive(ThisError, Debug)]
pub enum LocalProjectManagerCreateError {
    #[error("Could not check if root dir exists: {0}")]
    CouldNotCheckIfRootDirExists(#[source] IoError),
    #[error("Could not create root dir: {0}")]
    CouldNotCreateRootDir(#[source] IoError),
}

impl LocalProjectManager {
    pub async fn new(root_dir: PathBuf) -> Result<Self, LocalProjectManagerCreateError> {
        let span = info_span!("LocalProjectManager::new");
        let _span_guard = span.enter();

        // TODO: replace with create_all_dirs_if_not_exist
        if !fs::try_exists(&root_dir)
            .await
            .map_err(LocalProjectManagerCreateError::CouldNotCheckIfRootDirExists)?
        {
            tracing::info!(?root_dir, "Root dir does not exist, creating it");
            fs::create_dir_all(&root_dir)
                .await
                .map_err(LocalProjectManagerCreateError::CouldNotCreateRootDir)?;
        }

        let controllers = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            root_dir,
            controllers,
        })
    }

    /// Creates all directories that are needed for the project manager to work.
    /// ```root_dir```, ```enviroments_dir``` and ```installed_projects_dir``` are created.
    async fn create_all_dirs_if_not_exist(&self) -> Result<(), ()> {
        // self.root_dir
        // self.get_enviroments_dir()
        // self.get_installed_projects_dir()
        todo!()
    }

    async fn create_dir_if_not_exists(dir: PathBuf) -> Result<(), IoError> {
        if !fs::try_exists(&dir).await? {
            fs::create_dir_all(&dir).await?;
        }

        Ok(())
    }

    fn get_installed_projects_dir(&self) -> PathBuf {
        self.root_dir.join("installed_projects")
    }

    fn get_enviroments_dir(&self) -> PathBuf {
        self.root_dir.join("enviroments")
    }

    fn get_project_installation_dir(&self, project_id: String) -> PathBuf {
        self.get_installed_projects_dir().join(project_id)
    }

    fn get_project_enviroment_dir(&self, project_id: String) -> PathBuf {
        self.get_enviroments_dir().join(project_id)
    }

    /// Checks if the project is valid.
    /// Saves the project in the database if it is valid.
    /// ```project_dir``` is the base directory, from which the project should be installed.
    /// The ```ProjectManager``` has no control over this directory.
    pub async fn add_new_project_to_database(
        &self,
        project_id: String,
        project_name: String,
        project_dir: PathBuf,
    ) -> Result<(), ()> {
        todo!()
    }

    async fn remove_project_from_database(&self, project_id: String) -> Result<(), ()> {
        todo!()
    }

    /// Starts the installation of a project in a new task.
    /// The given ```project_id``` must be a valid project id, that is saved in the database.
    /// Forwards the installation stdout and stderr to the given channels.
    pub fn do_install_project(
        &self,
        project_id: String,
        stdout_sender: Option<mpsc::Sender<String>>,
        stderr_sender: Option<mpsc::Sender<String>>,
    ) -> Result<(), ()> {
        todo!()
    }

    /// After a successful installation, the project is copied to the installation directory.
    async fn copy_installed_project_to_installation_dir(
        &self,
        project_id: String,
    ) -> Result<(), ()> {
        todo!()
    }

    pub async fn uninstall_project(&self, project_id: String) {
        todo!()
    }

    pub async fn delete_project(&self, project_id: String) {
        todo!()
    }

    pub async fn current_installation_count(&self) -> usize {
        self.controllers.read().await.len()
    }
}
