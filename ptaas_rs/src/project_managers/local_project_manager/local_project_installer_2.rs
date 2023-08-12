use crate::{
    project_managers::process_2::{
        KilledTerminationStatus, OsProcessArgs, Process, ProcessController,
        ProcessKillAndWaitError, ProcessRunError, SendingCancellationSignalToProcessError, Status,
        TerminationStatus, TerminationWithErrorStatus,
    },
    util::{remove_dir_all_with_max_attempts_and_delay, MaxAttemptsExceeded},
};
use std::{
    io::Error as IoError,
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error as ThisError;
use tokio::{
    fs::{self, File, ReadDir},
    io::AsyncWriteExt,
    sync::mpsc,
};

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

    fn path_to_str_mapped_error(path: &Path) -> Result<&str, InstallError> {
        path.to_str()
            .ok_or(InstallError::FailedToConvertPathBufToString(path.into()))
    }

    pub async fn install(&mut self) -> Result<(), InstallError> {
        let uploaded_project_dir_str = Self::path_to_str_mapped_error(&self.uploaded_project_dir)?;

        let project_env_dir_str = Self::path_to_str_mapped_error(&self.project_env_dir)?;

        let requirements_file_path = self.get_requirements_file_path();
        let requirements_file_path_str = Self::path_to_str_mapped_error(&requirements_file_path)?;

        let pip_path = self.create_os_specific_pip_path();
        let pip_path_str = Self::path_to_str_mapped_error(&pip_path)?;

        let IoFiles {
            venv_stdout_file,
            venv_stderr_file,
            req_stdout_file,
            req_stderr_file,
        } = self.create_io_files().await?;

        let IoChannels {
            venv_stdout_sender,
            venv_stdout_receiver,
            venv_stderr_sender,
            venv_stderr_receiver,
            req_stdout_sender,
            req_stdout_receiver,
            req_stderr_sender,
            req_stderr_receiver,
        } = Self::create_io_channels();

        Self::do_forward_ios_and_write_to_files(IoForwardArgs {
            stdout_sender: self.stdout_sender.clone(),
            stderr_sender: self.stderr_sender.clone(),
            stdout_receiver: venv_stdout_receiver,
            stdout_file: venv_stdout_file,
            stderr_receiver: venv_stderr_receiver,
            stderr_file: venv_stderr_file,
            stdout_name: "venv_stdout",
            stderr_name: "venv_stderr",
        });

        let venv_process_args = OsProcessArgs {
            program: "python3",
            args: vec!["-m", "venv", project_env_dir_str],
            current_dir: uploaded_project_dir_str,
            stdout_sender: Some(venv_stdout_sender),
            stderr_sender: Some(venv_stderr_sender),
        };

        let venv_process_result = self.venv_process.run(venv_process_args).await;
        let venv_process_run_result =
            generate_process_run_result!(venv_process_result, VenvInstallError);

        if let Err(error) = venv_process_run_result {
            return Err(self.clean_up_on_error_and_return_error(error).await);
        }

        Self::do_forward_ios_and_write_to_files(IoForwardArgs {
            stdout_sender: self.stdout_sender.clone(),
            stderr_sender: self.stderr_sender.clone(),
            stdout_receiver: req_stdout_receiver,
            stdout_file: req_stdout_file,
            stderr_receiver: req_stderr_receiver,
            stderr_file: req_stderr_file,
            stdout_name: "req_stdout",
            stderr_name: "req_stderr",
        });

        let req_process_args = OsProcessArgs {
            program: pip_path_str,
            args: vec!["install", "-r", requirements_file_path_str],
            current_dir: uploaded_project_dir_str,
            stdout_sender: Some(req_stdout_sender),
            stderr_sender: Some(req_stderr_sender),
        };

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

    fn do_forward_ios_and_write_to_files(args: IoForwardArgs) {
        Self::do_forward_io_and_write_to_file(
            args.stdout_sender,
            args.stdout_receiver,
            args.stdout_file,
            args.stdout_name,
        );

        Self::do_forward_io_and_write_to_file(
            args.stderr_sender,
            args.stderr_receiver,
            args.stderr_file,
            args.stderr_name,
        );
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

    async fn create_file(&self, path: &Path) -> Result<File, CreateFileError> {
        File::create(&path)
            .await
            .map_err(|e| CreateFileError::CouldNotCreateFile(e, path.into()))
    }

    async fn create_venv_file(&self, path: &Path) -> Result<File, InstallError> {
        self.create_file(path)
            .await
            .map_err(|e| InstallError::VenvStartError(SubStartInstallError::CreateFileError(e)))
    }

    async fn create_req_file(&self, path: &Path) -> Result<File, InstallError> {
        self.create_file(path).await.map_err(|e| {
            InstallError::RequirementsStartError(SubStartInstallError::CreateFileError(e))
        })
    }

    async fn create_venv_stdout_file(&self) -> Result<File, InstallError> {
        let venv_stdout_file_path = self.get_venv_out_file_path();
        self.create_venv_file(&venv_stdout_file_path).await
    }

    async fn create_venv_stderr_file(&self) -> Result<File, InstallError> {
        let venv_stderr_file_path = self.get_venv_err_file_path();
        self.create_venv_file(&venv_stderr_file_path).await
    }

    async fn create_req_stdout_file(&self) -> Result<File, InstallError> {
        let req_stdout_file_path = self.get_req_out_file_path();
        self.create_req_file(&req_stdout_file_path).await
    }

    async fn create_req_stderr_file(&self) -> Result<File, InstallError> {
        let req_stderr_file_path = self.get_req_err_file_path();
        self.create_req_file(&req_stderr_file_path).await
    }

    async fn create_io_files(&self) -> Result<IoFiles, InstallError> {
        let venv_stdout_file = self.create_venv_stdout_file().await?;
        let venv_stderr_file = self.create_venv_stderr_file().await?;
        let req_stdout_file = self.create_req_stdout_file().await?;
        let req_stderr_file = self.create_req_stderr_file().await?;

        Ok(IoFiles {
            venv_stdout_file,
            venv_stderr_file,
            req_stdout_file,
            req_stderr_file,
        })
    }

    fn create_io_channels() -> IoChannels {
        let (venv_stdout_sender, venv_stdout_receiver) = mpsc::channel::<String>(100);
        let (venv_stderr_sender, venv_stderr_receiver) = mpsc::channel::<String>(100);
        let (req_stdout_sender, req_stdout_receiver) = mpsc::channel::<String>(100);
        let (req_stderr_sender, req_stderr_receiver) = mpsc::channel::<String>(100);

        IoChannels {
            venv_stdout_sender,
            venv_stdout_receiver,
            venv_stderr_sender,
            venv_stderr_receiver,
            req_stdout_sender,
            req_stdout_receiver,
            req_stderr_sender,
            req_stderr_receiver,
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

struct IoFiles {
    venv_stdout_file: File,
    venv_stderr_file: File,
    req_stdout_file: File,
    req_stderr_file: File,
}

struct IoChannels {
    venv_stdout_sender: mpsc::Sender<String>,
    venv_stdout_receiver: mpsc::Receiver<String>,
    venv_stderr_sender: mpsc::Sender<String>,
    venv_stderr_receiver: mpsc::Receiver<String>,
    req_stdout_sender: mpsc::Sender<String>,
    req_stdout_receiver: mpsc::Receiver<String>,
    req_stderr_sender: mpsc::Sender<String>,
    req_stderr_receiver: mpsc::Receiver<String>,
}

struct IoForwardArgs {
    stdout_sender: Option<mpsc::Sender<String>>,
    stderr_sender: Option<mpsc::Sender<String>>,
    stdout_receiver: mpsc::Receiver<String>,
    stdout_file: File,
    stderr_receiver: mpsc::Receiver<String>,
    stderr_file: File,
    stdout_name: &'static str,
    stderr_name: &'static str,
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

    fn create_installer_and_process_from_project_path(
        project_id_and_dir: String,
    ) -> (LocalProjectInstaller, LocalProjectInstallerController) {
        let uploaded_project_dir = get_uploaded_projects_dir().join(&project_id_and_dir);
        let installed_project_dir = get_installed_projects_dir().join(&project_id_and_dir);
        let project_env_dir = get_environments_dir().join(&project_id_and_dir);

        LocalProjectInstaller::new(
            project_id_and_dir,
            uploaded_project_dir,
            installed_project_dir,
            project_env_dir,
            None,
            None,
        )
    }

    mod check_projects {
        use super::*;

        #[tokio::test]
        #[traced_test]
        pub async fn fail_on_project_dir_does_not_exist() {
            let project_id_and_dir = String::from("project_dir_does_not_exist");
            let (installer, _controller) =
                create_installer_and_process_from_project_path(project_id_and_dir);

            let result = installer.check().await;
            match result {
                Err(ProjectCheckError::ProjectDir(ProjectDirError::ProjectDirDoesNotExist)) => {}
                _ => panic!("Unexpected result: {:?}", result),
            }
        }

        #[tokio::test]
        #[traced_test]
        pub async fn fail_on_project_dir_is_empty() {
            let project_id_and_dir = String::from("empty");
            let (installer, _controller) =
                create_installer_and_process_from_project_path(project_id_and_dir.clone());

            delete_gitkeep(&get_uploaded_projects_dir().join(&project_id_and_dir)).await;

            let result = installer.check().await;
            let panic_msg = match result {
                Err(ProjectCheckError::ProjectDir(ProjectDirError::ProjectDirIsEmpty)) => None,
                _ => Some(format!("Unexpected result: {:?}", result)),
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
            let (installer, _controller) =
                create_installer_and_process_from_project_path(project_id_and_dir);

            let result = installer.check().await;
            match result {
                Err(ProjectCheckError::Requirements(
                    RequirementsError::RequirementsTxtDoesNotExist,
                )) => {}
                _ => panic!("Unexpected result: {:?}", result),
            }
        }

        #[tokio::test]
        #[traced_test]
        pub async fn fail_on_requirements_does_not_contain_locust() {
            let project_id_and_dir = String::from("requirements_does_not_contain_locust");
            let (installer, _controller) =
                create_installer_and_process_from_project_path(project_id_and_dir);

            let result = installer.check().await;
            match result {
                Err(ProjectCheckError::Requirements(
                    RequirementsError::LocustIsNotInRequirementsTxt,
                )) => {}
                _ => panic!("Unexpected result: {:?}", result),
            }
        }

        #[tokio::test]
        #[traced_test]
        pub async fn fail_on_locust_dir_does_not_exist() {
            let project_id_and_dir = String::from("locust_dir_does_not_exist");
            let (installer, _controller) =
                create_installer_and_process_from_project_path(project_id_and_dir);

            let result = installer.check().await;
            match result {
                Err(ProjectCheckError::LocustDir(LocustDirError::LocustDirDoesNotExist)) => {}
                _ => panic!("Unexpected result: {:?}", result),
            }
        }

        #[tokio::test]
        #[traced_test]
        pub async fn fail_on_locust_dir_is_empty() {
            let project_id_and_dir = String::from("locust_dir_is_empty");
            let (installer, _controller) =
                create_installer_and_process_from_project_path(project_id_and_dir);

            let locust_dir = installer.get_locust_dir_path();
            delete_gitkeep(&locust_dir).await;

            let result = installer.check().await;
            let panic_msg = match result {
                Err(ProjectCheckError::LocustDir(LocustDirError::LocustDirIsEmpty)) => None,
                _ => Some(format!("Unexpected result: {:?}", result)),
            };

            restore_gitkeep(&get_uploaded_projects_dir().join(&locust_dir)).await;

            if let Some(msg) = panic_msg {
                panic!("{}", msg);
            }
        }

        #[tokio::test]
        #[traced_test]
        pub async fn fail_on_locust_dir_contains_no_python_files() {
            let project_id_and_dir = String::from("locust_dir_is_contains_no_python_files");
            let (installer, _controller) =
                create_installer_and_process_from_project_path(project_id_and_dir);

            let result = installer.check().await;
            match result {
                Err(ProjectCheckError::LocustDir(LocustDirError::NoPythonFilesInLocustDir)) => {}
                _ => panic!("Unexpected result: {:?}", result),
            }
        }

        #[tokio::test]
        #[traced_test]
        pub async fn check_a_valid_project_and_expect_no_errors() {
            let project_id_and_dir = String::from("valid");
            let (installer, _controller) =
                create_installer_and_process_from_project_path(project_id_and_dir);

            let result = installer.check().await;
            match result {
                Ok(_) => {}
                _ => panic!("Unexpected result: {:?}", result),
            }
        }
    }

    mod install_projects {
        use super::*;

        #[tokio::test]
        #[traced_test]
        pub async fn fail_on_invalid_requirements_with_exit_code_1() {
            let project_id_and_dir = String::from("invalid_requirements");
            let (mut installer, _controller) =
                create_installer_and_process_from_project_path(project_id_and_dir);

            let result = installer.check_and_install().await;

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

            match result {
                Err(CheckAndInstallError::InstallError(
                    InstallError::ErrorThatTriggersCleanUp(
                        ErrorThatTriggersCleanUp::RequirementsInstallError(
                            SubInstallError::TerminatedWithError(
                                TerminationWithErrorStatus::TerminatedWithErrorCode(code),
                            ),
                        ),
                    ),
                )) => {
                    assert_eq!(code, 1);
                }
                _ => panic!("Unexpected result: {:?}", result),
            }
        }

        #[tokio::test]
        #[traced_test]
        pub async fn kill_installation_and_expect_killed() {
            let project_id_and_dir = String::from("valid");
            let (mut installer, mut controller) =
                create_installer_and_process_from_project_path(project_id_and_dir);

            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let cancel_result = controller.cancel().await;
                match cancel_result {
                    Ok(None) => {}
                    _ => panic!("Unexpected cancel result: {:?}", cancel_result),
                }
            });

            let result = installer.check_and_install().await;

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

            match result {
                Err(CheckAndInstallError::InstallError(
                    InstallError::ErrorThatTriggersCleanUp(
                        ErrorThatTriggersCleanUp::RequirementsInstallError(
                            SubInstallError::Killed(_),
                        ),
                    ),
                )) => {}
                Err(CheckAndInstallError::InstallError(
                    InstallError::ErrorThatTriggersCleanUp(
                        ErrorThatTriggersCleanUp::VenvInstallError(SubInstallError::Killed(_)),
                    ),
                )) => {}
                _ => panic!("Unexpected result: {:?}", result),
            }
        }

        #[tokio::test]
        #[traced_test]
        pub async fn valid() {
            let project_id_and_dir = String::from("valid");
            let (mut installer, _controller) =
                create_installer_and_process_from_project_path(project_id_and_dir);

            if let Err(e) = installer.check_and_install().await {
                panic!("Unexpected error: {:?}", e);
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
