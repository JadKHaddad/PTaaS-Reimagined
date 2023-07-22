use std::{io::Error as IoError, path::PathBuf, process::Stdio};

use crate::project_managers::{
    process::{NewProcessArgs, ProcessCreateError, Status},
    Process,
};
use thiserror::Error as ThisError;
use tokio::fs;

#[derive(Debug)]
pub struct NewLocalProjectInstallerArgs {
    pub uploaded_project_dir: PathBuf,
    pub installed_project_dir: PathBuf,
    pub project_env_dir: PathBuf,
}

pub struct LocalProjectInstaller {
    uploaded_project_dir: PathBuf,
    installed_project_dir: PathBuf,
    project_env_dir: PathBuf,
    process: Process,
}

impl LocalProjectInstaller {
    pub async fn new(
        new_local_project_installer_args: NewLocalProjectInstallerArgs,
    ) -> Result<Self, StartInstallError> {
        let process = Self::check_and_start_install(&new_local_project_installer_args).await?;

        Ok(Self {
            uploaded_project_dir: new_local_project_installer_args.uploaded_project_dir,
            installed_project_dir: new_local_project_installer_args.installed_project_dir,
            project_env_dir: new_local_project_installer_args.project_env_dir,
            process,
        })
    }

    pub fn process_status(&mut self) -> Result<&Status, IoError> {
        self.process.status()
    }

    async fn check_and_start_install(
        new_local_project_installer_args: &NewLocalProjectInstallerArgs,
    ) -> Result<Process, StartInstallError> {
        Self::check(new_local_project_installer_args).await?;

        let new_process_args = if cfg!(target_os = "windows") {
            let project_env_dir_str = new_local_project_installer_args
                .project_env_dir
                .to_str()
                .ok_or(StartInstallError::FailedToConvertPathBufToString(
                    new_local_project_installer_args.project_env_dir.clone(),
                ))?;

            NewProcessArgs {
                given_id: None,
                program: "cmd",
                args: vec!["/C", "python", "-m", "venv", project_env_dir_str],
                current_dir: ".",
                stdin: Stdio::inherit(),
                stdout: Stdio::inherit(),
                stderr: Stdio::inherit(),
                kill_on_drop: true,
            }
        } else {
            todo!();
        };

        Ok(Process::new(new_process_args)?)
    }

    async fn check(
        new_local_project_installer_args: &NewLocalProjectInstallerArgs,
    ) -> Result<(), ProjectCheckError> {
        Self::check_dir_exists_and_not_empty(
            &new_local_project_installer_args.uploaded_project_dir,
        )
        .await
        .map_err(|err| ProjectCheckError::ProjectDirError(err.into()))?;

        let requirements_file_path = new_local_project_installer_args
            .uploaded_project_dir
            .join("requirements.txt");

        Self::check_requirements_txt_exists_and_locust_in_requirements_txt(&requirements_file_path)
            .await?;

        let locust_dir_path = new_local_project_installer_args
            .uploaded_project_dir
            .join("locust");

        Self::check_dir_exists_and_not_empty(&locust_dir_path)
            .await
            .map_err(|err| ProjectCheckError::LocustDirError(err.into()))?;

        Ok(())
    }

    async fn check_dir_exists_and_not_empty(
        dir: &PathBuf,
    ) -> Result<(), DirExistsAndNotEmptyError> {
        if !fs::try_exists(dir)
            .await
            .map_err(DirExistsAndNotEmptyError::CouldNotCheckIfDirExists)?
        {
            return Err(DirExistsAndNotEmptyError::DirDoesNotExist);
        }

        let mut dir_content = fs::read_dir(dir)
            .await
            .map_err(DirExistsAndNotEmptyError::CouldNotCheckIfDirIsEmpty)?;

        if dir_content
            .next_entry()
            .await
            .map_err(DirExistsAndNotEmptyError::CouldNotCheckIfDirIsEmpty)?
            .is_none()
        {
            return Err(DirExistsAndNotEmptyError::DirIsEmpty);
        }

        Ok(())
    }

    async fn check_requirements_txt_exists_and_locust_in_requirements_txt(
        requirements_file_path: &PathBuf,
    ) -> Result<(), RequirementsError> {
        if !fs::try_exists(&requirements_file_path)
            .await
            .map_err(RequirementsError::CouldNotCheckIfRequirementsTxtExists)?
        {
            return Err(RequirementsError::RequirementsTxtDoesNotExist);
        }

        let requirements_file_content = fs::read_to_string(requirements_file_path)
            .await
            .map_err(RequirementsError::CouldNotReadRequirementsTxt)?;

        if !requirements_file_content.contains("locust") {
            return Err(RequirementsError::LocustIsNotInRequirementsTxt);
        }

        Ok(())
    }
}

#[derive(ThisError, Debug)]
pub enum ProjectCheckError {
    #[error("Project dir error: {0}")]
    ProjectDirError(
        #[source]
        #[from]
        ProjectDirError,
    ),
    #[error("Requirements error: {0}")]
    RequirementsError(
        #[source]
        #[from]
        RequirementsError,
    ),
    #[error("Locust dir error: {0}")]
    LocustDirError(
        #[source]
        #[from]
        LocustDirError,
    ),
}

#[derive(ThisError, Debug)]
pub enum ProjectDirError {
    #[error("Could not check if project dir exists: {0}")]
    CouldNotCheckIfProjectDirExists(#[source] IoError),
    #[error("Project dir does not exist")]
    ProjectDirDoesNotExist,
    #[error("Could not check if project dir is empty: {0}")]
    CouldNotCheckIfProjectDirIsEmpty(#[source] IoError),
    #[error("Project dir is empty")]
    ProjectDirIsEmpty,
}

#[derive(ThisError, Debug)]
pub enum RequirementsError {
    #[error("Could not check if requirements.txt exists: {0}")]
    CouldNotCheckIfRequirementsTxtExists(#[source] IoError),
    #[error("requirements.txt does not exist")]
    RequirementsTxtDoesNotExist,
    #[error("Could not read requirements.txt: {0}")]
    CouldNotReadRequirementsTxt(#[source] IoError),
    #[error("Locust is not in requirements.txt")]
    LocustIsNotInRequirementsTxt,
}

#[derive(ThisError, Debug)]
pub enum LocustDirError {
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
    #[error("Could not convert path buf to string: {0}")]
    FailedToConvertPathBufToString(PathBuf),
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

#[derive(ThisError, Debug)]
pub enum DirExistsAndNotEmptyError {
    #[error("Could not check if dir exists: {0}")]
    CouldNotCheckIfDirExists(#[source] IoError),
    #[error("Dir does not exist")]
    DirDoesNotExist,
    #[error("Could not check if dir is empty: {0}")]
    CouldNotCheckIfDirIsEmpty(#[source] IoError),
    #[error("Dir is empty")]
    DirIsEmpty,
}

impl From<DirExistsAndNotEmptyError> for ProjectDirError {
    fn from(dir_exists_and_not_empty_error: DirExistsAndNotEmptyError) -> Self {
        match dir_exists_and_not_empty_error {
            DirExistsAndNotEmptyError::CouldNotCheckIfDirExists(e) => {
                Self::CouldNotCheckIfProjectDirExists(e)
            }
            DirExistsAndNotEmptyError::DirDoesNotExist => Self::ProjectDirDoesNotExist,
            DirExistsAndNotEmptyError::CouldNotCheckIfDirIsEmpty(e) => {
                Self::CouldNotCheckIfProjectDirIsEmpty(e)
            }
            DirExistsAndNotEmptyError::DirIsEmpty => Self::ProjectDirIsEmpty,
        }
    }
}

impl From<DirExistsAndNotEmptyError> for LocustDirError {
    fn from(dir_exists_and_not_empty_error: DirExistsAndNotEmptyError) -> Self {
        match dir_exists_and_not_empty_error {
            DirExistsAndNotEmptyError::CouldNotCheckIfDirExists(e) => {
                Self::CouldNotCheckIfLocustDirExists(e)
            }
            DirExistsAndNotEmptyError::DirDoesNotExist => Self::LocustDirDoesNotExist,
            DirExistsAndNotEmptyError::CouldNotCheckIfDirIsEmpty(e) => {
                Self::CouldNotCheckIfLocustDirIsEmpty(e)
            }
            DirExistsAndNotEmptyError::DirIsEmpty => Self::LocustDirIsEmpty,
        }
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

    fn create_project_installer_default_args(
        uploaded_project_dir: PathBuf,
    ) -> NewLocalProjectInstallerArgs {
        NewLocalProjectInstallerArgs {
            uploaded_project_dir,
            installed_project_dir: PathBuf::from(""),
            project_env_dir: PathBuf::from(""),
        }
    }

    #[tokio::test]
    pub async fn project_dir_does_not_exist() {
        let installer_args = create_project_installer_default_args(
            get_uploaded_projects_dir().join("project_dir_does_not_exist"),
        );

        match LocalProjectInstaller::check(&installer_args).await {
            Err(ProjectCheckError::ProjectDirError(ProjectDirError::ProjectDirDoesNotExist)) => {}
            Err(err) => {
                panic!("Unexpected error: {}", err);
            }
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    pub async fn project_dir_is_empty() {
        let installer_args =
            create_project_installer_default_args(get_uploaded_projects_dir().join("empty"));

        match LocalProjectInstaller::check(&installer_args).await {
            Err(ProjectCheckError::ProjectDirError(ProjectDirError::ProjectDirIsEmpty)) => {}
            Err(err) => {
                panic!("Unexpected error: {}", err);
            }
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    pub async fn requirements_does_not_exist() {
        let installer_args = create_project_installer_default_args(
            get_uploaded_projects_dir().join("requirements_does_not_exist"),
        );

        match LocalProjectInstaller::check(&installer_args).await {
            Err(ProjectCheckError::RequirementsError(
                RequirementsError::RequirementsTxtDoesNotExist,
            )) => {}
            Err(err) => {
                panic!("Unexpected error: {}", err);
            }
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    pub async fn requirements_does_not_contain_locust() {
        let installer_args = create_project_installer_default_args(
            get_uploaded_projects_dir().join("requirements_does_not_contain_locust"),
        );

        match LocalProjectInstaller::check(&installer_args).await {
            Err(ProjectCheckError::RequirementsError(
                RequirementsError::LocustIsNotInRequirementsTxt,
            )) => {}
            Err(err) => {
                panic!("Unexpected error: {}", err);
            }
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    pub async fn locust_dir_does_not_exist() {
        let installer_args = create_project_installer_default_args(
            get_uploaded_projects_dir().join("locust_dir_does_not_exist"),
        );

        match LocalProjectInstaller::check(&installer_args).await {
            Err(ProjectCheckError::LocustDirError(LocustDirError::LocustDirDoesNotExist)) => {}
            Err(err) => {
                panic!("Unexpected error: {}", err);
            }
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    pub async fn locust_dir_is_empty() {
        let installer_args = create_project_installer_default_args(
            get_uploaded_projects_dir().join("locust_dir_is_empty"),
        );

        match LocalProjectInstaller::check(&installer_args).await {
            Err(ProjectCheckError::LocustDirError(LocustDirError::LocustDirIsEmpty)) => {}
            Err(err) => {
                panic!("Unexpected error: {}", err);
            }
            _ => panic!("Unexpected result"),
        }
    }
}
