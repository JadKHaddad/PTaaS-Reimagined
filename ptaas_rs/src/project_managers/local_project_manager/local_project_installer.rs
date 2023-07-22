use std::{
    io::Error as IoError,
    path::{Path, PathBuf},
    process::Stdio,
};

use crate::project_managers::{
    process::{NewProcessArgs, Output, ProcessCreateError, ProcessKillAndWaitError, Status},
    Process,
};
use thiserror::Error as ThisError;
use tokio::fs::{self, ReadDir};

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

    /// Returns the status of the underlying process, not the status of the installation.
    pub fn process_status(&mut self) -> Result<&Status, IoError> {
        self.process.status()
    }

    pub async fn stop(&mut self) -> Result<(), ProcessKillAndWaitError> {
        self.process
            .check_status_and_kill_and_wait_and_set_status()
            .await
    }

    async fn wait_process_with_output(&mut self) -> Result<Output, IoError> {
        self.process.wait_with_output_and_set_status().await
    }

    /// Checks if the project is valid and starts the installation process in the background.
    async fn check_and_start_install(
        new_local_project_installer_args: &NewLocalProjectInstallerArgs,
    ) -> Result<Process, StartInstallError> {
        Self::check(new_local_project_installer_args).await?;

        let uploaded_project_dir = &new_local_project_installer_args.uploaded_project_dir;

        let project_env_dir = &new_local_project_installer_args.project_env_dir;
        let project_env_dir_str =
            project_env_dir
                .to_str()
                .ok_or(StartInstallError::FailedToConvertPathBufToString(
                    new_local_project_installer_args.project_env_dir.clone(),
                ))?;

        let requirements_file_path = Self::get_requirements_file_path(uploaded_project_dir);
        let requirements_file_path_str = requirements_file_path.to_str().ok_or(
            StartInstallError::FailedToConvertPathBufToString(requirements_file_path.clone()),
        )?;

        let new_process = if cfg!(target_os = "windows") {
            let pip_path = project_env_dir.join("Scripts").join("pip3");
            let pip_path_str =
                pip_path
                    .to_str()
                    .ok_or(StartInstallError::FailedToConvertPathBufToString(
                        pip_path.clone(),
                    ))?;

            let new_process_args = NewProcessArgs {
                given_id: None,
                program: "cmd",
                args: vec![
                    "/C",
                    "python",
                    "-m",
                    "venv",
                    project_env_dir_str,
                    "&&",
                    pip_path_str,
                    "install",
                    "-r",
                    requirements_file_path_str,
                ],
                current_dir: ".",
                stdin: Stdio::inherit(),
                stdout: Stdio::inherit(),
                stderr: Stdio::inherit(),
                kill_on_drop: true,
            };

            Process::new(new_process_args)?
        } else {
            todo!();
        };

        Ok(new_process)
    }

    async fn delete_environment_dir_if_exists(&self) -> Result<(), IoError> {
        if fs::try_exists(&self.project_env_dir).await? {
            self.delete_environment_dir().await?;
        }

        Ok(())
    }

    async fn delete_environment_dir(&self) -> Result<(), IoError> {
        fs::remove_dir_all(&self.project_env_dir).await
    }

    fn get_requirements_file_path(uploaded_project_dir: &Path) -> PathBuf {
        uploaded_project_dir.join("requirements.txt")
    }

    fn get_locust_dir_path(uploaded_project_dir: &Path) -> PathBuf {
        uploaded_project_dir.join("locust")
    }

    /// A 'check' function fails if the project is not valid.
    /// Otherwise it returns Ok(()).
    async fn check(
        new_local_project_installer_args: &NewLocalProjectInstallerArgs,
    ) -> Result<(), ProjectCheckError> {
        let uploaded_project_dir = &new_local_project_installer_args.uploaded_project_dir;

        let _ = Self::check_dir_exists_and_not_empty(uploaded_project_dir)
            .await
            .map_err(|err| ProjectCheckError::ProjectDirError(err.into()))?;

        let requirements_file_path = Self::get_requirements_file_path(uploaded_project_dir);

        Self::check_requirements_txt_exists_and_locust_in_requirements_txt(&requirements_file_path)
            .await?;

        let locust_dir_path = Self::get_locust_dir_path(uploaded_project_dir);

        Self::check_locust_dir_exists_and_not_empty_and_contains_python_scripts(&locust_dir_path)
            .await
            .map_err(ProjectCheckError::LocustDirError)?;

        Ok(())
    }

    async fn check_dir_exists_and_not_empty(
        dir: &PathBuf,
    ) -> Result<ReadDir, DirExistsAndNotEmptyError> {
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

        Ok(dir_content)
    }

    async fn check_locust_dir_exists_and_not_empty_and_contains_python_scripts(
        dir: &PathBuf,
    ) -> Result<(), LocustDirError> {
        let mut dir_content = Self::check_dir_exists_and_not_empty(dir).await?;

        while let Some(entry) = dir_content
            .next_entry()
            .await
            .map_err(LocustDirError::CouldNotIterateOverLocustDir)?
        {
            if let Some("py") = entry.path().extension().and_then(|ext| ext.to_str()) {
                return Ok(());
            }
        }

        Err(LocustDirError::NoPythonFilesInLocustDir)
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
    #[error("Could not iterate over locust dir: {0}")]
    CouldNotIterateOverLocustDir(#[source] IoError),
    #[error("Locust dir does not contain any python files")]
    NoPythonFilesInLocustDir,
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
    use super::*;
    use std::path::Path;

    const CRATE_DIR: &str = env!("CARGO_MANIFEST_DIR");

    fn get_tests_dir() -> PathBuf {
        Path::new(CRATE_DIR).join("tests_dir")
    }

    fn get_uploaded_projects_dir() -> PathBuf {
        get_tests_dir().join("uploaded_projects")
    }

    fn get_installed_projects_dir() -> PathBuf {
        get_tests_dir().join("installed_projects")
    }

    fn get_environments_dir() -> PathBuf {
        get_tests_dir().join("environments")
    }

    mod check_projects {
        use super::*;

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
                Err(ProjectCheckError::ProjectDirError(
                    ProjectDirError::ProjectDirDoesNotExist,
                )) => {}
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

        #[tokio::test]
        pub async fn locust_dir_contains_no_python_files() {
            let installer_args = create_project_installer_default_args(
                get_uploaded_projects_dir().join("locust_dir_is_contains_no_python_files"),
            );

            match LocalProjectInstaller::check(&installer_args).await {
                Err(ProjectCheckError::LocustDirError(
                    LocustDirError::NoPythonFilesInLocustDir,
                )) => {}
                Err(err) => {
                    panic!("Unexpected error: {}", err);
                }
                _ => panic!("Unexpected result"),
            }
        }

        #[tokio::test]
        pub async fn valid() {
            let installer_args =
                create_project_installer_default_args(get_uploaded_projects_dir().join("valid"));

            match LocalProjectInstaller::check(&installer_args).await {
                Ok(_) => {}
                Err(err) => {
                    panic!("Unexpected error: {}", err);
                }
            }
        }
    }

    mod install_projects {
        use super::*;

        #[tokio::test]
        pub async fn invalid_requirements() {
            let uploaded_project_dir = get_uploaded_projects_dir().join("invalid_requirements");
            let installed_project_dir = get_installed_projects_dir().join("invalid_requirements");
            let project_env_dir = get_environments_dir().join("invalid_requirements");

            let installer_args = NewLocalProjectInstallerArgs {
                uploaded_project_dir,
                installed_project_dir,
                project_env_dir,
            };

            let mut installer = LocalProjectInstaller::new(installer_args)
                .await
                .expect("Installation process failed to start");

            let output = installer
                .wait_process_with_output()
                .await
                .expect("Wait failed");

            installer
                .delete_environment_dir_if_exists()
                .await
                .expect("Could not delete environment dir");

            match output.status {
                Status::TerminatedWithError(_) => {}
                _ => panic!("Unexpected status: {:?}", output.status),
            }
        }

        #[tokio::test]
        pub async fn killed() {
            let uploaded_project_dir = get_uploaded_projects_dir().join("valid");
            let installed_project_dir = get_installed_projects_dir().join("valid");
            let project_env_dir = get_environments_dir().join("valid");

            let installer_args = NewLocalProjectInstallerArgs {
                uploaded_project_dir,
                installed_project_dir,
                project_env_dir,
            };

            let mut installer = LocalProjectInstaller::new(installer_args)
                .await
                .expect("Installation process failed to start");

            installer
                .stop()
                .await
                .expect("Could not stop installation process");

            let output = installer
                .wait_process_with_output()
                .await
                .expect("Wait failed");

            installer
                .delete_environment_dir_if_exists()
                .await
                .expect("Could not delete environment dir");

            match output.status {
                Status::TerminatedWithUnknownError => if cfg!(target_os = "linux") {},
                Status::TerminatedWithError(_) => if cfg!(target_os = "windows") {},
                Status::TerminatedSuccessfully => panic!("Unexpected status: {:?}", output.status),
                _ => panic!("Uncovered case"),
            }
        }

        #[tokio::test]
        pub async fn valid() {
            let uploaded_project_dir = get_uploaded_projects_dir().join("valid");
            let installed_project_dir = get_installed_projects_dir().join("valid");
            let project_env_dir = get_environments_dir().join("valid");

            let installer_args = NewLocalProjectInstallerArgs {
                uploaded_project_dir,
                installed_project_dir,
                project_env_dir,
            };

            let mut installer = LocalProjectInstaller::new(installer_args)
                .await
                .expect("Installation process failed to start");

            let output = installer
                .wait_process_with_output()
                .await
                .expect("Wait failed");

            installer
                .delete_environment_dir_if_exists()
                .await
                .expect("Could not delete environment dir");

            match output.status {
                Status::TerminatedSuccessfully => {}
                _ => panic!("Unexpected status: {:?}", output.status),
            }
        }
    }
}
