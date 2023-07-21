use std::{io::Error as IoError, path::PathBuf, process::Stdio};

use crate::project_managers::{
    process::{NewProcessArgs, ProcessCreateError},
    Process,
};
use thiserror::Error as ThisError;
use tokio::fs;

#[derive(ThisError, Debug)]
pub enum ProjectCheckError {
    #[error("Could not check if project dir exists: {0}")]
    CouldNotCheckIfProjectDirExists(#[source] IoError),
    #[error("Project dir does not exist")]
    ProjectDirDoesNotExist,
    #[error("Could not check if project dir is empty: {0}")]
    CouldNotCheckIfProjectDirIsEmpty(#[source] IoError),
    #[error("Project dir is empty")]
    ProjectDirIsEmpty,
    #[error("Could not check if requirements.txt exists: {0}")]
    CouldNotCheckIfRequirementsTxtExists(#[source] IoError),
    #[error("requirements.txt does not exist")]
    RequirementsTxtDoesNotExist,
    #[error("Could not read requirements.txt: {0}")]
    CouldNotReadRequirementsTxt(#[source] IoError),
    #[error("Locust is not in requirements.txt")]
    LocustIsNotInRequirementsTxt,
    #[error("Could not check if locust dir exists: {0}")]
    CouldNotCheckIfLocustDirExists(#[source] IoError),
    #[error("Locust dir does not exist")]
    LocustDirDoesNotExist,
    #[error("Could not check if locust dir is empty: {0}")]
    CouldNotCheckIfLocustDirIsEmpty(#[source] IoError),
    #[error("Locust dir is empty")]
    LocustDirIsEmpty,
}

#[derive(ThisError, Debug)]
pub enum StartInstallError {
    #[error("Project is not valid: {0}")]
    CheckFailed(
        #[from]
        #[source]
        ProjectCheckError,
    ),

    #[error("Could not create process: {0}")]
    ProcessCreateError(
        #[from]
        #[source]
        ProcessCreateError,
    ),
}

pub struct LocalProjectInstaller {
    uploaded_project_dir: PathBuf,
    installed_project_dir: PathBuf,
    project_env_dir: PathBuf,
    process: Option<Process>,
}

impl LocalProjectInstaller {
    pub fn new(
        uploaded_project_dir: PathBuf,
        installed_project_dir: PathBuf,
        project_env_dir: PathBuf,
    ) -> Self {
        Self {
            uploaded_project_dir,
            installed_project_dir,
            project_env_dir,
            process: None,
        }
    }

    pub async fn start_install(&mut self) -> Result<(), StartInstallError> {
        self.check().await?;

        let new_process_args = if cfg!(target_os = "windows") {
            NewProcessArgs {
                given_id: None,
                program: "cmd",
                args: vec![
                    "/C",
                    "python",
                    "-m",
                    "venv",
                    self.project_env_dir.to_str().unwrap(),
                ],
                current_dir: ".",
                stdin: Stdio::inherit(),
                stdout: Stdio::inherit(),
                stderr: Stdio::inherit(),
                kill_on_drop: true,
            }
        } else {
            todo!();
        };

        self.process = Some(Process::new(new_process_args)?);

        Ok(())
    }

    async fn check(&self) -> Result<(), ProjectCheckError> {
        // check if uploaded_project_dir exists
        if !fs::try_exists(&self.uploaded_project_dir)
            .await
            .map_err(ProjectCheckError::CouldNotCheckIfProjectDirExists)?
        {
            return Err(ProjectCheckError::ProjectDirDoesNotExist);
        }

        // check if uploaded_project_dir is empty
        let mut uploaded_project_dir_content = fs::read_dir(&self.uploaded_project_dir)
            .await
            .map_err(ProjectCheckError::CouldNotCheckIfProjectDirIsEmpty)?;

        if uploaded_project_dir_content
            .next_entry()
            .await
            .map_err(ProjectCheckError::CouldNotCheckIfProjectDirIsEmpty)?
            .is_none()
        {
            return Err(ProjectCheckError::ProjectDirIsEmpty);
        }

        // check if requirements.txt exists in uploaded_project_dir
        let requirements_file_path = self.uploaded_project_dir.join("requirements.txt");
        if !fs::try_exists(&requirements_file_path)
            .await
            .map_err(ProjectCheckError::CouldNotCheckIfRequirementsTxtExists)?
        {
            return Err(ProjectCheckError::RequirementsTxtDoesNotExist);
        }

        // check if locust is in requirements.txt
        let requirements_file_content = fs::read_to_string(&requirements_file_path)
            .await
            .map_err(ProjectCheckError::CouldNotReadRequirementsTxt)?;

        if !requirements_file_content.contains("locust") {
            return Err(ProjectCheckError::LocustIsNotInRequirementsTxt);
        }

        // check if locust dir exists in uploaded_project_dir
        let locust_dir_path = self.uploaded_project_dir.join("locust");
        if !fs::try_exists(&locust_dir_path)
            .await
            .map_err(ProjectCheckError::CouldNotCheckIfLocustDirExists)?
        {
            return Err(ProjectCheckError::LocustDirDoesNotExist);
        }

        // check if locust dir is empty
        let mut locust_dir_content = fs::read_dir(&locust_dir_path)
            .await
            .map_err(ProjectCheckError::CouldNotCheckIfLocustDirExists)?;

        if locust_dir_content
            .next_entry()
            .await
            .map_err(ProjectCheckError::CouldNotCheckIfLocustDirIsEmpty)?
            .is_none()
        {
            return Err(ProjectCheckError::LocustDirIsEmpty);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn get_uploaded_projects_dir() -> PathBuf {
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        Path::new(crate_dir)
            .join("tests_dir")
            .join("uploaded_projects")
    }

    fn create_project_installer_with_default_dirs(
        uploaded_project_dir: PathBuf,
    ) -> LocalProjectInstaller {
        LocalProjectInstaller::new(uploaded_project_dir, PathBuf::from(""), PathBuf::from(""))
    }

    #[tokio::test]
    pub async fn project_dir_does_not_exist() {
        let installer = create_project_installer_with_default_dirs(
            get_uploaded_projects_dir().join("project_dir_does_not_exist"),
        );

        match installer.check().await {
            Err(ProjectCheckError::ProjectDirDoesNotExist) => {}
            Err(err) => {
                panic!("Unexpected error: {}", err);
            }
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    pub async fn project_dir_is_empty() {
        let installer =
            create_project_installer_with_default_dirs(get_uploaded_projects_dir().join("empty"));

        match installer.check().await {
            Err(ProjectCheckError::ProjectDirIsEmpty) => {}
            Err(err) => {
                panic!("Unexpected error: {}", err);
            }
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    pub async fn requirements_does_not_exist() {
        let installer = create_project_installer_with_default_dirs(
            get_uploaded_projects_dir().join("requirements_does_not_exist"),
        );

        match installer.check().await {
            Err(ProjectCheckError::RequirementsTxtDoesNotExist) => {}
            Err(err) => {
                panic!("Unexpected error: {}", err);
            }
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    pub async fn requirements_does_not_contain_locust() {
        let installer = create_project_installer_with_default_dirs(
            get_uploaded_projects_dir().join("requirements_does_not_contain_locust"),
        );

        match installer.check().await {
            Err(ProjectCheckError::LocustIsNotInRequirementsTxt) => {}
            Err(err) => {
                panic!("Unexpected error: {}", err);
            }
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    pub async fn locust_dir_does_not_exist() {
        let installer = create_project_installer_with_default_dirs(
            get_uploaded_projects_dir().join("locust_dir_does_not_exist"),
        );

        match installer.check().await {
            Err(ProjectCheckError::LocustDirDoesNotExist) => {}
            Err(err) => {
                panic!("Unexpected error: {}", err);
            }
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    pub async fn locust_dir_is_empty() {
        let installer = create_project_installer_with_default_dirs(
            get_uploaded_projects_dir().join("locust_dir_is_empty"),
        );

        match installer.check().await {
            Err(ProjectCheckError::LocustDirIsEmpty) => {}
            Err(err) => {
                panic!("Unexpected error: {}", err);
            }
            _ => panic!("Unexpected result"),
        }
    }
}
