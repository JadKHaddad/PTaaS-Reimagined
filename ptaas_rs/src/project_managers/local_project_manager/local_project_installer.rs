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
use tokio::fs::{self, File, ReadDir};

#[derive(Debug)]
pub struct NewLocalProjectInstallerArgs {
    pub id: String,
    pub uploaded_project_dir: PathBuf,
    pub installed_project_dir: PathBuf,
    pub project_env_dir: PathBuf,
}

pub struct LocalProjectInstaller {
    id: String,
    uploaded_project_dir: PathBuf,
    installed_project_dir: PathBuf,
    project_env_dir: PathBuf,
    process: Process,
}

struct FileAndStringPath {
    file: File,
    path: String,
}

struct OsSpecificArgs {
    program: &'static str,
    pip_path: PathBuf,
    first_arg: &'static str,
}

impl LocalProjectInstaller {
    pub async fn create_and_check_and_start_install(
        new_local_project_installer_args: NewLocalProjectInstallerArgs,
    ) -> Result<Self, CreateAndStartInstallError> {
        let process = Self::check_and_start_install(&new_local_project_installer_args).await?;

        let mut installer = Self {
            id: new_local_project_installer_args.id,
            uploaded_project_dir: new_local_project_installer_args.uploaded_project_dir,
            installed_project_dir: new_local_project_installer_args.installed_project_dir,
            project_env_dir: new_local_project_installer_args.project_env_dir,
            process,
        };

        if let Err(create_file_error) = installer
            .create_file_and_do_pipe_oi()
            .await
            .map_err(ErrorThatTriggersCleanUp::CreateFileError)
        {
            return Err(installer
                .clean_up_on_error_and_return_error(create_file_error)
                .await);
        }

        Ok(installer)
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

    #[cfg(test)]
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

        let OsSpecificArgs {
            program,
            pip_path,
            first_arg,
        } = Self::create_os_specific_args(project_env_dir);

        let pip_path_str =
            pip_path
                .to_str()
                .ok_or(StartInstallError::FailedToConvertPathBufToString(
                    pip_path.clone(),
                ))?;

        let install_cmd = Self::create_install_cmd(
            project_env_dir_str,
            pip_path_str,
            requirements_file_path_str,
        );

        let process_id = Self::create_process_id(&new_local_project_installer_args.id);

        let new_process_args = NewProcessArgs {
            given_id: Some(process_id),
            program,
            args: vec![first_arg, &install_cmd],
            current_dir: ".",
            stdin: Stdio::null(),
            stdout: Stdio::piped(),
            stderr: Stdio::piped(),
            kill_on_drop: true,
        };

        Ok(Process::create_and_run(new_process_args)?)
    }

    fn create_process_id(id: &str) -> String {
        format!("install_{}", id)
    }

    fn create_install_cmd(
        project_env_dir_str: &str,
        pip_path_str: &str,
        requirements_file_path_str: &str,
    ) -> String {
        format!(
            "python3 -m venv {} && {} install -r {}",
            project_env_dir_str, pip_path_str, requirements_file_path_str
        )
    }

    fn create_os_specific_args(project_env_dir: &Path) -> OsSpecificArgs {
        let (program, pip_path, first_arg) = if cfg!(target_os = "windows") {
            let program = "cmd";
            let pip_path = project_env_dir.join("Scripts").join("pip3");
            let first_first_arg = "/C";

            (program, pip_path, first_first_arg)
        } else {
            let program = "bash";
            let pip_path = project_env_dir.join("bin").join("pip3");
            let first_first_arg = "-c";

            (program, pip_path, first_first_arg)
        };

        OsSpecificArgs {
            program,
            pip_path,
            first_arg,
        }
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

    fn get_process_out_file_path(&self) -> PathBuf {
        self.uploaded_project_dir.join("out.txt")
    }

    fn get_process_err_file_path(&self) -> PathBuf {
        self.uploaded_project_dir.join("err.txt")
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

    async fn create_file_and_do_pipe_oi(&mut self) -> Result<(), CreateFileError> {
        self.create_file_and_do_pipe_stdout().await?;
        self.create_file_and_do_pipe_stderr().await
    }

    async fn create_file_string_path(
        file_path: &Path,
    ) -> Result<FileAndStringPath, CreateFileError> {
        let file_path_string = file_path
            .to_str()
            .ok_or_else(|| CreateFileError::FailedToConvertPathBufToString(file_path.to_owned()))?
            .to_owned();
        let file = File::create(file_path).await?;
        Ok(FileAndStringPath {
            file,
            path: file_path_string,
        })
    }

    async fn create_out_file_and_string_path(&self) -> Result<FileAndStringPath, CreateFileError> {
        let file_path = self.get_process_out_file_path();
        Self::create_file_string_path(&file_path).await
    }

    async fn create_err_file_and_string_path(&self) -> Result<FileAndStringPath, CreateFileError> {
        let file_path = self.get_process_err_file_path();
        Self::create_file_string_path(&file_path).await
    }

    async fn create_file_and_do_pipe_stdout(&mut self) -> Result<(), CreateFileError> {
        let FileAndStringPath { file, path } = self.create_out_file_and_string_path().await?;
        self.process.do_pipe_stdout_to_file(file, path).await;
        Ok(())
    }

    async fn create_file_and_do_pipe_stderr(&mut self) -> Result<(), CreateFileError> {
        let FileAndStringPath { file, path } = self.create_err_file_and_string_path().await?;
        self.process.do_pipe_stderr_to_file(file, path).await;
        Ok(())
    }

    async fn clean_up_on_error(&mut self) -> Result<(), CleanUpError> {
        self.stop().await?;
        self.delete_environment_dir_if_exists()
            .await
            .map_err(CleanUpError::CouldNotDeleteEnvironment)?;
        Ok(())
    }

    /// If an error occurs during the clean up, a `CleanUpError` is returned.
    /// If no error occurs during the clean up, the given error mapped to a `CreateAndStartInstallError` is returned.
    async fn clean_up_on_error_and_return_error(
        &mut self,
        error: ErrorThatTriggersCleanUp,
    ) -> CreateAndStartInstallError {
        match self.clean_up_on_error().await {
            Ok(_) => StartInstallError::ErrorThatTriggersCleanUp(error).into(),
            Err(clean_up_error) => CreateAndStartInstallError::CleanUpError(error, clean_up_error),
        }
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
pub enum CreateAndStartInstallError {
    #[error("Could not start install: {0}")]
    StartInstallError(
        #[from]
        #[source]
        StartInstallError,
    ),
    #[error("An error occurred: {0}, and could not clean up: {1}")]
    CleanUpError(ErrorThatTriggersCleanUp, #[source] CleanUpError),
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
    #[error("{0}")]
    ErrorThatTriggersCleanUp(
        #[from]
        #[source]
        ErrorThatTriggersCleanUp,
    ),
}

#[derive(ThisError, Debug)]
pub enum ErrorThatTriggersCleanUp {
    #[error("Could not create file: {0}")]
    CreateFileError(
        #[from]
        #[source]
        CreateFileError,
    ),
}

#[derive(ThisError, Debug)]
pub enum CleanUpError {
    #[error("Could not kill process: {0}")]
    CouldNotKillProcess(
        #[source]
        #[from]
        ProcessKillAndWaitError,
    ),
    #[error("Could not delete environment dir: {0}")]
    CouldNotDeleteEnvironment(#[source] IoError),
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

#[derive(ThisError, Debug)]
pub enum CreateFileError {
    #[error("Could not create file: {0}")]
    CouldNotCreateFile(
        #[source]
        #[from]
        IoError,
    ),
    #[error("Could not convert path buf to string: {0}")]
    FailedToConvertPathBufToString(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tracing_test::traced_test;

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

    async fn delete_gitkeep(dir: &Path) {
        tokio::fs::remove_file(dir.join(".gitkeep"))
            .await
            .expect("Could not delete .gitkeep");
    }

    async fn restore_gitkeep(dir: &Path) {
        tokio::fs::File::create(dir.join(".gitkeep"))
            .await
            .expect("Could not restore .gitkeep");
    }

    mod check_projects {
        use super::*;

        fn create_project_installer_default_args(
            uploaded_project_dir: PathBuf,
            id: String,
        ) -> NewLocalProjectInstallerArgs {
            NewLocalProjectInstallerArgs {
                id,
                uploaded_project_dir,
                installed_project_dir: PathBuf::from(""),
                project_env_dir: PathBuf::from(""),
            }
        }

        #[tokio::test]
        #[traced_test]
        pub async fn fail_on_project_dir_does_not_exist() {
            let project_id_and_dir = String::from("project_dir_does_not_exist");
            let installer_args = create_project_installer_default_args(
                get_uploaded_projects_dir().join(&project_id_and_dir),
                project_id_and_dir,
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
        #[traced_test]
        pub async fn fail_on_project_dir_is_empty() {
            let project_id_and_dir = String::from("empty");

            delete_gitkeep(&get_uploaded_projects_dir().join(&project_id_and_dir)).await;

            let installer_args = create_project_installer_default_args(
                get_uploaded_projects_dir().join(&project_id_and_dir),
                project_id_and_dir.clone(),
            );

            let panic_msg = match LocalProjectInstaller::check(&installer_args).await {
                Err(ProjectCheckError::ProjectDirError(ProjectDirError::ProjectDirIsEmpty)) => None,
                Err(err) => Some(format!("Unexpected error: {}", err)),
                _ => Some(String::from("Unexpected result")),
            };

            restore_gitkeep(&get_uploaded_projects_dir().join(&project_id_and_dir)).await;

            if let Some(msg) = panic_msg {
                panic!("{}", msg);
            }
        }

        #[tokio::test]
        #[traced_test]
        pub async fn fail_on_requirements_does_not_exist() {
            let project_id_and_dir = String::from("requirements_does_not_exist");
            let installer_args = create_project_installer_default_args(
                get_uploaded_projects_dir().join(&project_id_and_dir),
                project_id_and_dir,
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
        #[traced_test]
        pub async fn fail_on_requirements_does_not_contain_locust() {
            let project_id_and_dir = String::from("requirements_does_not_contain_locust");
            let installer_args = create_project_installer_default_args(
                get_uploaded_projects_dir().join(&project_id_and_dir),
                project_id_and_dir,
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
        #[traced_test]
        pub async fn fail_on_locust_dir_does_not_exist() {
            let project_id_and_dir = String::from("locust_dir_does_not_exist");
            let installer_args = create_project_installer_default_args(
                get_uploaded_projects_dir().join(&project_id_and_dir),
                project_id_and_dir,
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
        #[traced_test]
        pub async fn fail_on_locust_dir_is_empty() {
            let project_id_and_dir = String::from("locust_dir_is_empty");
            let locust_dir = get_uploaded_projects_dir()
                .join(&project_id_and_dir)
                .join("locust");

            delete_gitkeep(&locust_dir).await;

            let installer_args = create_project_installer_default_args(
                get_uploaded_projects_dir().join(&project_id_and_dir),
                project_id_and_dir,
            );

            let panic_msg = match LocalProjectInstaller::check(&installer_args).await {
                Err(ProjectCheckError::LocustDirError(LocustDirError::LocustDirIsEmpty)) => None,
                Err(err) => Some(format!("Unexpected error: {}", err)),
                _ => Some(String::from("Unexpected result")),
            };

            restore_gitkeep(&locust_dir).await;

            if let Some(msg) = panic_msg {
                panic!("{}", msg);
            }
        }

        #[tokio::test]
        #[traced_test]
        pub async fn fail_on_locust_dir_contains_no_python_files() {
            let project_id_and_dir = String::from("locust_dir_is_contains_no_python_files");
            let installer_args = create_project_installer_default_args(
                get_uploaded_projects_dir().join(&project_id_and_dir),
                project_id_and_dir,
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
        #[traced_test]
        pub async fn check_a_valid_project_and_expect_no_errors() {
            let project_id_and_dir = String::from("valid");
            let installer_args = create_project_installer_default_args(
                get_uploaded_projects_dir().join(&project_id_and_dir),
                project_id_and_dir,
            );

            if let Err(err) = LocalProjectInstaller::check(&installer_args).await {
                panic!("Unexpected error: {}", err);
            }
        }
    }

    mod install_projects {
        use crate::project_managers::process::TerminationWithErrorStatus;

        use super::*;

        #[tokio::test]
        #[traced_test]
        pub async fn fail_on_invalid_requirements_with_exit_code_1() {
            let project_id_and_dir = String::from("invalid_requirements");
            let uploaded_project_dir = get_uploaded_projects_dir().join(&project_id_and_dir);
            let installed_project_dir = get_installed_projects_dir().join(&project_id_and_dir);
            let project_env_dir = get_environments_dir().join(&project_id_and_dir);

            let installer_args = NewLocalProjectInstallerArgs {
                id: project_id_and_dir,
                uploaded_project_dir,
                installed_project_dir,
                project_env_dir,
            };

            let mut installer =
                LocalProjectInstaller::create_and_check_and_start_install(installer_args)
                    .await
                    .expect("Installation process failed to start");

            let output = installer.wait_process_with_output().await;

            installer
                .delete_environment_dir_if_exists()
                .await
                .expect("Could not delete environment dir");

            match output {
                Ok(output) => match output.status {
                    Status::TerminatedWithError(
                        TerminationWithErrorStatus::TerminatedWithErrorCode(code),
                    ) => {
                        assert_eq!(code, 1);
                    }
                    _ => panic!("Unexpected status: {:?}", output.status),
                },
                Err(err) => {
                    panic!("Could not wait for process: {}", err);
                }
            }
        }

        #[tokio::test]
        #[traced_test]
        pub async fn kill_installation_and_expect_killed() {
            let project_id = String::from("killed");
            let project_dir = String::from("valid");
            let uploaded_project_dir = get_uploaded_projects_dir().join(&project_dir);
            let installed_project_dir = get_installed_projects_dir().join(&project_dir);
            let project_env_dir = get_environments_dir().join(&project_dir);

            let installer_args = NewLocalProjectInstallerArgs {
                id: project_id,
                uploaded_project_dir,
                installed_project_dir,
                project_env_dir,
            };

            let mut installer =
                LocalProjectInstaller::create_and_check_and_start_install(installer_args)
                    .await
                    .expect("Installation process failed to start");

            let stop_result = installer.stop().await;

            let output_result = installer.wait_process_with_output().await;

            installer
                .delete_environment_dir_if_exists()
                .await
                .expect("Could not delete environment dir");

            if let Err(err) = stop_result {
                panic!("Could not stop process: {}", err);
            }

            let Ok(output) = output_result else {
                panic!("Could not wait for process");
            };

            match output.status {
                Status::Killed => {}
                _ => panic!("Unexpected status: {:?}", output.status),
            }
        }

        #[tokio::test]
        #[traced_test]
        pub async fn install_a_valid_project_and_expect_no_errors() {
            let project_id_and_dir = String::from("valid");
            let uploaded_project_dir = get_uploaded_projects_dir().join(&project_id_and_dir);
            let installed_project_dir = get_installed_projects_dir().join(&project_id_and_dir);
            let project_env_dir = get_environments_dir().join(&project_id_and_dir);

            let installer_args = NewLocalProjectInstallerArgs {
                id: project_id_and_dir,
                uploaded_project_dir,
                installed_project_dir,
                project_env_dir,
            };

            let mut installer =
                LocalProjectInstaller::create_and_check_and_start_install(installer_args)
                    .await
                    .expect("Installation process failed to start");

            let output_result = installer.wait_process_with_output().await;

            installer
                .delete_environment_dir_if_exists()
                .await
                .expect("Could not delete environment dir");

            let Ok(output) = output_result else {
                panic!("Could not wait for process");
            };

            match output.status {
                Status::TerminatedSuccessfully => {}
                _ => panic!("Unexpected status: {:?}", output.status),
            }
        }
    }
}
