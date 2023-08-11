use crate::project_managers::process_2::{
    KilledTerminationStatus, OsProcessArgs, Process, ProcessController, ProcessKillAndWaitError,
    ProcessRunError, SendingCancellationSignalToProcessError, Status, TerminationStatus,
    TerminationWithErrorStatus,
};
use std::path::PathBuf;
use std::time::Duration;
use std::{io::Error as IoError, path::Path};
use thiserror::Error as ThisError;
use tokio::fs::{self, File, ReadDir};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

macro_rules! generate_process_run_result {
    ($process_run_result:ident, $error_that_triggers_cleanup_variant:ident) => {
        match $process_run_result {
            Ok(status) => match status {
                Status::Terminated(term_status) => match term_status {
                    TerminationStatus::TerminatedSuccessfully => Ok(()),
                    TerminationStatus::Killed(killed_term_status) => Err(
                        ErrorThatTriggersCleanUp::$error_that_triggers_cleanup_variant(
                            SubInstallError::Killed(killed_term_status),
                        ),
                    ),
                    TerminationStatus::TerminatedWithError(term_with_error_status) => Err(
                        ErrorThatTriggersCleanUp::$error_that_triggers_cleanup_variant(
                            SubInstallError::TerminatedWithError(term_with_error_status),
                        ),
                    ),
                },
                _ => Err(
                    ErrorThatTriggersCleanUp::$error_that_triggers_cleanup_variant(
                        SubInstallError::UnexpectedStatus(status),
                    ),
                ),
            },
            Err(error) => Err(
                ErrorThatTriggersCleanUp::$error_that_triggers_cleanup_variant(
                    SubInstallError::RunError(error),
                ),
            ),
        }
    };
}

#[derive(ThisError, Debug)]
pub enum InstallerKillAndWaitError {
    #[error("Failed to kill and wait for venv process: {0}")]
    VenvKillAndWaitError(#[source] ProcessKillAndWaitError),
    #[error("Failed to kill and wait for req process: {0}")]
    ReqKillAndWaitError(#[source] ProcessKillAndWaitError),
}

#[derive(ThisError, Debug)]
pub enum SendingCancellationSignalToInstallerError {
    #[error("Failed to cancel venv process: {0}")]
    VenvCancellationError(#[source] SendingCancellationSignalToProcessError),
    #[error("Failed to cancel req process: {0}")]
    ReqCancellationError(#[source] SendingCancellationSignalToProcessError),
}

pub struct LocalProjectInstallerController {
    venv_controller: ProcessController,
    req_controller: ProcessController,
}

impl LocalProjectInstallerController {
    pub async fn cancel(
        &mut self,
    ) -> Result<Option<InstallerKillAndWaitError>, SendingCancellationSignalToInstallerError> {
        match self.cancel_venv().await {
            Ok(option_kill_and_wait_error) => {
                Ok(option_kill_and_wait_error.map(InstallerKillAndWaitError::VenvKillAndWaitError))
            }
            Err(SendingCancellationSignalToProcessError::ProcessTerminated) => {
                self.cancel_req_mapped().await
            }
            Err(cancellation_error) => Err(
                SendingCancellationSignalToInstallerError::VenvCancellationError(
                    cancellation_error,
                ),
            ),
        }
    }

    async fn cancel_venv(
        &mut self,
    ) -> Result<Option<ProcessKillAndWaitError>, SendingCancellationSignalToProcessError> {
        self.venv_controller.cancel().await
    }

    async fn cancel_req(
        &mut self,
    ) -> Result<Option<ProcessKillAndWaitError>, SendingCancellationSignalToProcessError> {
        self.req_controller.cancel().await
    }

    async fn cancel_req_mapped(
        &mut self,
    ) -> Result<Option<InstallerKillAndWaitError>, SendingCancellationSignalToInstallerError> {
        Ok(self
            .cancel_req()
            .await
            .map_err(SendingCancellationSignalToInstallerError::ReqCancellationError)?
            .map(InstallerKillAndWaitError::ReqKillAndWaitError))
    }
}

pub struct LocalProjectInstaller {
    id: String,
    uploaded_project_dir: PathBuf,
    installed_project_dir: PathBuf,
    project_env_dir: PathBuf,
    venv_process: Process,
    req_process: Process,
    stdout_sender: Option<mpsc::Sender<String>>,
    stderr_sender: Option<mpsc::Sender<String>>,
}

impl LocalProjectInstaller {
    pub fn new(
        id: String,
        uploaded_project_dir: PathBuf,
        installed_project_dir: PathBuf,
        project_env_dir: PathBuf,
        stdout_sender: Option<mpsc::Sender<String>>,
        stderr_sender: Option<mpsc::Sender<String>>,
    ) -> (Self, LocalProjectInstallerController) {
        let (venv_process, venv_controller) = Process::new(
            String::from("venv_id"),
            String::from("install_venv_process"),
        );

        let (req_process, req_controller) =
            Process::new(String::from("req_id"), String::from("install_req_process"));

        (
            Self {
                id,
                uploaded_project_dir,
                installed_project_dir,
                project_env_dir,
                venv_process,
                req_process,
                stdout_sender,
                stderr_sender,
            },
            LocalProjectInstallerController {
                venv_controller,
                req_controller,
            },
        )
    }

    /// A 'check' function fails if the project is not valid.
    /// Otherwise it returns Ok(()).
    pub async fn check(&self) -> Result<(), ProjectCheckError> {
        let uploaded_project_dir = &self.uploaded_project_dir;

        let _ = Self::check_dir_exists_and_not_empty(uploaded_project_dir)
            .await
            .map_err(|err| ProjectCheckError::ProjectDir(err.into()))?;

        self.check_requirements_txt_exists_and_locust_in_requirements_txt()
            .await?;

        self.check_locust_dir_exists_and_not_empty_and_contains_python_scripts()
            .await
            .map_err(ProjectCheckError::LocustDir)?;

        Ok(())
    }

    pub async fn install(&mut self) -> Result<(), InstallError> {
        let uploaded_project_dir_str = self.uploaded_project_dir.to_str().ok_or(
            InstallError::FailedToConvertPathBufToString(self.uploaded_project_dir.clone()),
        )?;

        let project_env_dir = &self.project_env_dir;
        let project_env_dir_str =
            project_env_dir
                .to_str()
                .ok_or(InstallError::FailedToConvertPathBufToString(
                    self.project_env_dir.clone(),
                ))?;

        let requirements_file_path = self.get_requirements_file_path();
        let requirements_file_path_str =
            requirements_file_path
                .to_str()
                .ok_or(InstallError::FailedToConvertPathBufToString(
                    requirements_file_path.clone(),
                ))?;

        let pip_path = self.create_os_specific_pip_path();

        let pip_path_str =
            pip_path
                .to_str()
                .ok_or(InstallError::FailedToConvertPathBufToString(
                    pip_path.clone(),
                ))?;

        // create files and channels
        let venv_stdout_file_path = self.get_venv_out_file_path();
        let venv_stdout_file = File::create(&venv_stdout_file_path).await.map_err(|e| {
            InstallError::VenvStartError(SubStartInstallError::CreateFileError(
                CreateFileError::CouldNotCreateFile(e, venv_stdout_file_path),
            ))
        })?;

        let venv_stderr_file_path = self.get_venv_err_file_path();
        let venv_stderr_file = File::create(&venv_stderr_file_path).await.map_err(|e| {
            InstallError::VenvStartError(SubStartInstallError::CreateFileError(
                CreateFileError::CouldNotCreateFile(e, venv_stderr_file_path),
            ))
        })?;

        let req_stdout_file_path = self.get_req_out_file_path();
        let req_stdout_file = File::create(&req_stdout_file_path).await.map_err(|e| {
            InstallError::RequirementsStartError(SubStartInstallError::CreateFileError(
                CreateFileError::CouldNotCreateFile(e, req_stdout_file_path),
            ))
        })?;

        let req_stderr_file_path = self.get_req_err_file_path();
        let req_stderr_file = File::create(&req_stderr_file_path).await.map_err(|e| {
            InstallError::RequirementsStartError(SubStartInstallError::CreateFileError(
                CreateFileError::CouldNotCreateFile(e, req_stderr_file_path),
            ))
        })?;

        let (venv_stdout_sender, venv_stdout_receiver) = mpsc::channel::<String>(100);
        let (venv_stderr_sender, venv_stderr_receiver) = mpsc::channel::<String>(100);

        let (req_stdout_sender, req_stdout_receiver) = mpsc::channel::<String>(100);
        let (req_stderr_sender, req_stderr_receiver) = mpsc::channel::<String>(100);

        // Create processes args
        let venv_process_args = OsProcessArgs {
            program: "python3",
            args: vec!["-m", "venv", project_env_dir_str],
            current_dir: uploaded_project_dir_str,
            stdout_sender: Some(venv_stdout_sender),
            stderr_sender: Some(venv_stderr_sender),
        };

        let req_process_args = OsProcessArgs {
            program: pip_path_str,
            args: vec!["install", "-r", requirements_file_path_str],
            current_dir: uploaded_project_dir_str,
            stdout_sender: Some(req_stdout_sender),
            stderr_sender: Some(req_stderr_sender),
        };

        // do some channel magic for venv
        let stdout_sender = self.stdout_sender.clone();
        Self::do_forward_io_and_write_to_file(
            stdout_sender,
            venv_stdout_receiver,
            venv_stdout_file,
            "venv_stdout",
        );

        let stderr_sender = self.stderr_sender.clone();
        Self::do_forward_io_and_write_to_file(
            stderr_sender,
            venv_stderr_receiver,
            venv_stderr_file,
            "venv_sterr",
        );

        let venv_process_result = self.venv_process.run(venv_process_args).await;
        let venv_process_run_result =
            generate_process_run_result!(venv_process_result, VenvInstallError);

        if let Err(error) = venv_process_run_result {
            return Err(self.clean_up_on_error_and_return_error(error).await);
        }

        // do some channel magic for req
        let stdout_sender = self.stdout_sender.clone();
        Self::do_forward_io_and_write_to_file(
            stdout_sender,
            req_stdout_receiver,
            req_stdout_file,
            "req_stdout",
        );

        let stderr_sender = self.stderr_sender.clone();
        Self::do_forward_io_and_write_to_file(
            stderr_sender,
            req_stderr_receiver,
            req_stderr_file,
            "req_stderr",
        );

        let req_process_result = self.req_process.run(req_process_args).await;
        let req_process_run_result =
            generate_process_run_result!(req_process_result, RequirementsInstallError);

        if let Err(error) = req_process_run_result {
            return Err(self.clean_up_on_error_and_return_error(error).await);
        }

        Ok(())
    }

    pub async fn check_and_install(&mut self) -> Result<(), CheckAndInstallError> {
        self.check()
            .await
            .map_err(CheckAndInstallError::CheckError)?;

        self.install()
            .await
            .map_err(CheckAndInstallError::InstallError)?;

        Ok(())
    }

    fn do_forward_io_and_write_to_file(
        sender_to_forward_to: Option<mpsc::Sender<String>>,
        mut receiver: mpsc::Receiver<String>,
        mut file: File,
        io_name: &'static str,
    ) {
        tokio::spawn(async move {
            while let Some(mut line) = receiver.recv().await {
                line.push('\n');
                if let Err(err) = file.write_all(line.as_bytes()).await {
                    tracing::error!(%err, io_name, "Failed to write to file");
                    break;
                }
                if let Some(sender) = &sender_to_forward_to {
                    if let Err(err) = sender.send(line).await {
                        tracing::error!(%err, io_name, "Failed to send line to sender");
                    }
                }
            }
        });
    }

    async fn delete_environment_dir_if_exists(
        &self,
    ) -> Result<Vec<IoError>, DeleteEnvironmentDirError> {
        if fs::try_exists(&self.project_env_dir).await? {
            let errors = self.delete_environment_dir().await?;
            return Ok(errors);
        }

        Ok(Vec::new())
    }

    async fn delete_environment_dir(&self) -> Result<Vec<IoError>, MaxAttemptsExceeded> {
        remove_dir_all_with_max_attempts_and_delay(5, Duration::from_secs(2), &self.project_env_dir)
            .await
    }

    fn get_requirements_file_path(&self) -> PathBuf {
        self.uploaded_project_dir.join("requirements.txt")
    }

    fn get_locust_dir_path(&self) -> PathBuf {
        self.uploaded_project_dir.join("locust")
    }

    fn get_venv_out_file_path(&self) -> PathBuf {
        self.uploaded_project_dir.join("venv_out.txt")
    }

    fn get_venv_err_file_path(&self) -> PathBuf {
        self.uploaded_project_dir.join("venv_err.txt")
    }

    fn get_req_out_file_path(&self) -> PathBuf {
        self.uploaded_project_dir.join("req_out.txt")
    }

    fn get_req_err_file_path(&self) -> PathBuf {
        self.uploaded_project_dir.join("req_err.txt")
    }

    pub async fn get_venv_out_from_file(&self) -> Result<String, IoError> {
        fs::read_to_string(self.get_venv_out_file_path()).await
    }

    pub async fn get_venv_err_from_file(&self) -> Result<String, IoError> {
        fs::read_to_string(self.get_venv_err_file_path()).await
    }

    pub async fn get_req_out_from_file(&self) -> Result<String, IoError> {
        fs::read_to_string(self.get_req_out_file_path()).await
    }

    pub async fn get_req_err_from_file(&self) -> Result<String, IoError> {
        fs::read_to_string(self.get_req_err_file_path()).await
    }

    async fn check_dir_exists_and_not_empty(
        dir: &Path,
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
        &self,
    ) -> Result<(), LocustDirError> {
        let dir = self.get_locust_dir_path();
        let mut dir_content = Self::check_dir_exists_and_not_empty(&dir).await?;

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
        &self,
    ) -> Result<(), RequirementsError> {
        let requirements_file_path = self.get_requirements_file_path();
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

    fn create_os_specific_pip_path(&self) -> PathBuf {
        if cfg!(target_os = "windows") {
            self.project_env_dir.join("Scripts").join("pip3")
        } else if cfg!(target_os = "linux") {
            self.project_env_dir.join("bin").join("pip3")
        } else {
            tracing::warn!("Unknown OS, assuming linux");
            self.project_env_dir.join("bin").join("pip3")
        }
    }

    async fn clean_up_on_error(&mut self) -> Result<(), CleanUpError> {
        //TODO: what to do with errors vec?
        let io_errors_vector = self
            .delete_environment_dir_if_exists()
            .await
            .map_err(CleanUpError::CouldNotDeleteEnvironment)?;
        Ok(())
    }

    /// If an error occurs during the clean up, a `CleanUpError` is returned.
    /// If no error occurs during the clean up, the given error mapped to a `InstallError` is returned.
    async fn clean_up_on_error_and_return_error(
        &mut self,
        error: ErrorThatTriggersCleanUp,
    ) -> InstallError {
        match self.clean_up_on_error().await {
            Ok(_) => InstallError::ErrorThatTriggersCleanUp(error),
            Err(clean_up_error) => InstallError::CleanUpError(error, clean_up_error),
        }
    }
}

#[derive(ThisError, Debug)]
pub enum ProjectCheckError {
    #[error("Project dir error: {0}")]
    ProjectDir(
        #[source]
        #[from]
        ProjectDirError,
    ),
    #[error("Requirements error: {0}")]
    Requirements(
        #[source]
        #[from]
        RequirementsError,
    ),
    #[error("Locust dir error: {0}")]
    LocustDir(
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
pub enum SubStartInstallError {
    #[error("Error creating file: {0}")]
    CreateFileError(
        #[from]
        #[source]
        CreateFileError,
    ),
}

#[derive(ThisError, Debug)]
pub enum SubInstallError {
    #[error("Process failed to start: {0}")]
    RunError(
        #[from]
        #[source]
        ProcessRunError,
    ),
    #[error("Process killed")]
    Killed(KilledTerminationStatus),
    #[error("Process terminated with error")]
    TerminatedWithError(TerminationWithErrorStatus),
    #[error("Process had unexpected status")]
    UnexpectedStatus(Status),
}

#[derive(ThisError, Debug)]
pub enum CheckAndInstallError {
    #[error("Project is not valid: {0}")]
    CheckError(
        #[from]
        #[source]
        ProjectCheckError,
    ),
    #[error("Failed to install project: {0}")]
    InstallError(
        #[from]
        #[source]
        InstallError,
    ),
}

#[derive(ThisError, Debug)]
pub enum InstallError {
    #[error("Could not convert path buf to string: {0}")]
    FailedToConvertPathBufToString(PathBuf),
    #[error("Virtual environment installation can not be started: {0}")]
    VenvStartError(#[source] SubStartInstallError),
    #[error("Requirements installation can not be started: {0}")]
    RequirementsStartError(#[source] SubStartInstallError),
    #[error("{0}")]
    ErrorThatTriggersCleanUp(
        #[from]
        #[source]
        ErrorThatTriggersCleanUp,
    ),
    #[error("An error occurred: {0}, and could not clean up: {1}")]
    CleanUpError(ErrorThatTriggersCleanUp, #[source] CleanUpError),
}

#[derive(ThisError, Debug)]
pub enum ErrorThatTriggersCleanUp {
    #[error("Virtual environment installation failed: {0}")]
    VenvInstallError(#[source] SubInstallError),
    #[error("Requirements installation failed: {0}")]
    RequirementsInstallError(#[source] SubInstallError),
}

#[derive(ThisError, Debug)]
pub enum CleanUpError {
    #[error("Could not delete environment dir: {0}")]
    CouldNotDeleteEnvironment(#[source] DeleteEnvironmentDirError),
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
    #[error("Could not create file: {0} {1}")]
    CouldNotCreateFile(#[source] IoError, PathBuf),
    #[error("Could not convert path buf to string: {0}")]
    FailedToConvertPathBufToString(PathBuf),
}

#[derive(ThisError, Debug)]
pub enum DeleteEnvironmentDirError {
    #[error("Could not check if dir exists: {0}")]
    CouldNotCheckIfDirExists(
        #[source]
        #[from]
        IoError,
    ),
    #[error("{0}")]
    MaxAttemptsExceeded(
        #[source]
        #[from]
        MaxAttemptsExceeded,
    ),
}

#[derive(ThisError, Debug)]
#[error("Max attempts exceeded")]
pub struct MaxAttemptsExceeded(Vec<IoError>);

async fn remove_dir_all_with_max_attempts_and_delay(
    max_attempts: u16,
    delay: Duration,
    path: &Path,
) -> Result<Vec<IoError>, MaxAttemptsExceeded> {
    let mut errors = Vec::new();

    for _ in 0..max_attempts {
        tracing::debug!(?path, "Attempting to delete dir");
        match fs::remove_dir_all(path).await {
            Ok(_) => return Ok(errors),
            Err(err) => {
                tracing::error!(%err, ?path, "Failed to delete dir");
                errors.push(err);
                tokio::time::sleep(delay).await;
            }
        }
    }

    Err(MaxAttemptsExceeded(errors))
}

// #[cfg(test)]
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

    mod install_projects {
        use super::*;

        #[tokio::test]
        #[traced_test]
        #[ignore = "Failing on CI for some reason"]
        pub async fn valid() {
            let project_id_and_dir = String::from("valid");
            let uploaded_project_dir = get_uploaded_projects_dir().join(&project_id_and_dir);
            let installed_project_dir = get_installed_projects_dir().join(&project_id_and_dir);
            let project_env_dir = get_environments_dir().join(&project_id_and_dir);

            let (mut installer, _controller) = LocalProjectInstaller::new(
                project_id_and_dir,
                uploaded_project_dir,
                installed_project_dir,
                project_env_dir,
                None,
                None,
            );

            if let Err(e) = installer.check_and_install().await {
                panic!("Unexpected error: {}", e);
            }

            installer
                .delete_environment_dir_if_exists()
                .await
                .expect("Could not delete environment dir");

            let venv_err = installer
                .get_venv_err_from_file()
                .await
                .expect("Could not get venv err");
            println!("venv_err: {}", venv_err);

            let req_err = installer
                .get_req_err_from_file()
                .await
                .expect("Could not get req err");
            println!("req_err: {}", req_err);
        }
    }
}
