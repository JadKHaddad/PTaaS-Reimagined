use std::{
    ffi::OsStr,
    io::Error as IoError,
    path::Path,
    process::{ExitStatus, Stdio},
    sync::Arc,
};

use thiserror::Error as ThisError;
use tokio::{
    io::{self, AsyncBufReadExt, AsyncRead},
    process::{Child, ChildStderr, ChildStdout, Command},
    sync::{mpsc, oneshot, RwLock},
};

#[derive(Debug, Clone)]
pub enum Status {
    Created,
    Running,
    Terminated(TerminationStatus),
}

#[derive(Debug, Clone)]
pub enum TerminationStatus {
    /// Explicitly killed by this library.
    Killed,
    KilledByDroppingHandler,
    TerminatedSuccessfully,
    TerminatedWithError(TerminationWithErrorStatus),
}

#[derive(Debug, Clone)]
pub enum TerminationWithErrorStatus {
    /// On SIGTERM, the process will exit UnknownErrorCode.
    /// On windows, the process will exit with 1. This will be translated to `Killed` if `child_killed_successfuly` is true.
    /// On linux, the process will exit with UnknownErrorCode. This will be translated to `Killed` if `child_killed_successfuly` is true.
    /// Otherwise, it will not be translated.
    TerminatedWithUnknownErrorCode,
    TerminatedWithErrorCode(i32),
}

#[derive(Debug)]
pub struct Output {
    pub status: Status,
    pub stdout: Option<ChildStdout>,
    pub stderr: Option<ChildStderr>,
}

/// Used in the constructor of `Process` to pass arguments, to improve readability.
#[derive(Debug)]
pub struct NewProcessArgs<I, S, P> {
    pub program: S,
    pub args: I,
    pub current_dir: P,
    pub kill_on_drop: bool,
    pub stdout_sender: Option<mpsc::Sender<String>>,
    pub stderr_sender: Option<mpsc::Sender<String>>,
}

impl<I, S, P> TryFrom<NewProcessArgs<I, S, P>> for Child
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    P: AsRef<Path>,
{
    type Error = ProcessRunError;

    fn try_from(value: NewProcessArgs<I, S, P>) -> Result<Self, Self::Error> {
        let stdout = if value.stdout_sender.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        };

        let stderr = if value.stderr_sender.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        };

        Command::new(value.program)
            .args(value.args)
            .current_dir(value.current_dir)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .kill_on_drop(value.kill_on_drop)
            .spawn()
            .map_err(ProcessRunError::CouldNotCreateProcess)
    }
}

pub struct Process {
    status: Arc<RwLock<Status>>,
    given_id: String,
    child_killed_successfuly: bool,
    /// Option so we can take it
    cancel_status_channel_sender: Option<oneshot::Sender<Option<ProcessKillAndWaitError>>>,
    /// Option so we can take it
    cancel_channel_receiver: Option<oneshot::Receiver<()>>,
}

pub struct ProcessHandler {
    status: Arc<RwLock<Status>>,
    /// Option so we can take it
    cancel_channel_sender: Option<oneshot::Sender<()>>,
    /// Option so we can take it
    cancel_status_channel_receiver: Option<oneshot::Receiver<Option<ProcessKillAndWaitError>>>,
}

impl ProcessHandler {
    /// Will deadlock if corresponding `Process` has not been started.
    pub async fn cancel(&mut self) -> Result<Option<ProcessKillAndWaitError>, CancellationError> {
        let cancel_channel_sender = self
            .cancel_channel_sender
            .take()
            .ok_or(CancellationError::AlreayTriedToCancel)?;

        let cancel_channel_receiver = self
            .cancel_status_channel_receiver
            .take()
            .ok_or(CancellationError::AlreayTriedToCancel)?;

        cancel_channel_sender
            .send(())
            .map_err(|_| CancellationError::ProcessDropped)?;

        cancel_channel_receiver
            .await
            .map_err(|_| CancellationError::ProcessDropped)
    }

    pub async fn status(&self) -> Status {
        self.status.read().await.clone()
    }
}

impl Process {
    #[must_use]
    pub fn new(given_id: String) -> (Self, ProcessHandler) {
        let status = Arc::new(RwLock::new(Status::Created));

        let (cancel_status_channel_sender, cancel_status_channel_receiver) =
            tokio::sync::oneshot::channel();
        let (cancel_channel_sender, cancel_channel_receiver) = tokio::sync::oneshot::channel();

        let process = Self {
            status: status.clone(),
            given_id,
            child_killed_successfuly: false,
            cancel_status_channel_sender: Some(cancel_status_channel_sender),
            cancel_channel_receiver: Some(cancel_channel_receiver),
        };

        let process_handler = ProcessHandler {
            status,
            cancel_channel_sender: Some(cancel_channel_sender),
            cancel_status_channel_receiver: Some(cancel_status_channel_receiver),
        };

        (process, process_handler)
    }

    pub async fn write_status(&self, status: Status) {
        *self.status.write().await = status;
    }

    pub async fn run<I, S, P>(
        &mut self,
        mut new_process_args: NewProcessArgs<I, S, P>,
    ) -> Result<Status, ProcessRunError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        P: AsRef<Path>,
    {
        let cancel_channel_sender = self
            .cancel_status_channel_sender
            .take()
            .ok_or(ProcessRunError::AlreadyStarted)?;

        let cancel_channel_receiver = self
            .cancel_channel_receiver
            .take()
            .ok_or(ProcessRunError::AlreadyStarted)?;

        let stdout_sender = new_process_args.stdout_sender.take();
        let stderr_sender = new_process_args.stderr_sender.take();

        let mut child = Child::try_from(new_process_args)?;

        self.write_status(Status::Running).await;

        if let Some(sender) = stdout_sender {
            let stdout = child.stdout.take().expect("stdout was not piped");
            Self::forward_io_to_channel(stdout, sender);
        }

        if let Some(sender) = stderr_sender {
            let stderr = child.stderr.take().expect("stderr was not piped");
            Self::forward_io_to_channel(stderr, sender);
        }

        tokio::select! {
            result = cancel_channel_receiver => {
                if result.is_ok() {
                    // The process was explicitly cancelled by the handler
                    // Cancelation errors are sent to the handler and this function returns
                    match self.check_if_still_running_and_kill_and_wait(child).await {
                        Ok(exit_status) => {
                            self.write_status_on_exit_status(exit_status, false).await;

                            if cancel_channel_sender.send(None).is_err() {
                                return Err(ProcessRunError::HandlerDropped);
                            }
                        }
                        Err(e) => {
                            if cancel_channel_sender.send(Some(e)).is_err() {
                                return Err(ProcessRunError::HandlerDropped);
                            }
                        }
                    }
                }
                else {
                    // The handler was dropped, wich means we can't send the cancelation error, so we return it here
                    let exit_status = self.check_if_still_running_and_kill_and_wait(child).await?;
                    self.write_status_on_exit_status(exit_status, true).await;
                }
            }

            result_exit_status = child.wait() => {
                let exit_status = result_exit_status.map_err(ProcessRunError::CouldNotWaitForProcess)?;
                self.write_status_on_exit_status(exit_status, false).await;
            }
        }

        let status = self.status().await;

        Ok(status)
    }

    async fn check_if_still_running_and_kill_and_wait(
        &mut self,
        mut child: Child,
    ) -> Result<ExitStatus, ProcessKillAndWaitError> {
        let option_exit_status = child
            .try_wait()
            .map_err(ProcessKillAndWaitError::CouldNotCheckStatus)?;

        let exit_status = match option_exit_status {
            Some(exit_status) => exit_status,
            None => {
                child
                    .kill()
                    .await
                    .map_err(ProcessKillAndWaitError::CouldNotKillProcess)?;

                self.child_killed_successfuly = true;

                child
                    .wait()
                    .await
                    .map_err(ProcessKillAndWaitError::CouldNotWaitForProcess)?
            }
        };

        Ok(exit_status)
    }

    async fn get_status_on_exit_status(
        &self,
        exit_status: ExitStatus,
        handler_dropped: bool,
    ) -> Status {
        if exit_status.success() {
            return Status::Terminated(TerminationStatus::TerminatedSuccessfully);
        };

        match exit_status.code() {
            Some(code) => match code {
                1 if cfg!(target_os = "windows") && self.child_killed_successfuly => {
                    if handler_dropped {
                        return Status::Terminated(TerminationStatus::KilledByDroppingHandler);
                    }

                    Status::Terminated(TerminationStatus::Killed)
                }
                _ => Status::Terminated(TerminationStatus::TerminatedWithError(
                    TerminationWithErrorStatus::TerminatedWithErrorCode(code),
                )),
            },
            None if cfg!(target_os = "linux") && self.child_killed_successfuly => {
                if handler_dropped {
                    return Status::Terminated(TerminationStatus::KilledByDroppingHandler);
                }

                Status::Terminated(TerminationStatus::Killed)
            }
            _ => Status::Terminated(TerminationStatus::TerminatedWithError(
                TerminationWithErrorStatus::TerminatedWithUnknownErrorCode,
            )),
        }
    }

    async fn write_status_on_exit_status(&self, exit_status: ExitStatus, handler_dropped: bool) {
        let status = self
            .get_status_on_exit_status(exit_status, handler_dropped)
            .await;
        self.write_status(status).await;
    }

    pub async fn status(&self) -> Status {
        self.status.read().await.clone()
    }

    fn forward_io_to_channel<T: AsyncRead + Unpin + Send + 'static>(
        stdio: T,
        sender: mpsc::Sender<String>,
    ) {
        let reader = io::BufReader::new(stdio);
        let mut lines = reader.lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                if sender.send(line).await.is_err() {
                    break;
                }
            }
        });
    }
}

#[derive(ThisError, Debug)]
pub enum ProcessRunError {
    #[error("Process has already been started, can not start again")]
    AlreadyStarted,
    #[error("Could not create process: {0}")]
    CouldNotCreateProcess(#[source] IoError),
    #[error("Could not wait for process: {0}")]
    CouldNotWaitForProcess(#[source] IoError),
    #[error("Corresponding ProcessHandler was dropped. Should be infallible")]
    HandlerDropped,
    #[error("{0}")]
    ProcessKillAndWaitError(
        #[source]
        #[from]
        ProcessKillAndWaitError,
    ),
}

#[derive(ThisError, Debug)]
pub enum ProcessKillAndWaitError {
    #[error("Could not check status of process: {0}")]
    CouldNotCheckStatus(#[source] IoError),
    #[error("Could not kill process: {0}")]
    CouldNotKillProcess(#[source] IoError),
    #[error("Could not wait for process: {0}")]
    CouldNotWaitForProcess(#[source] IoError),
}

#[derive(ThisError, Debug)]
pub enum CancellationError {
    #[error("Cancellation is already requested")]
    AlreadyCancelled,
    #[error("Cancellation can only be requested once")]
    AlreayTriedToCancel,
    #[error("Corresponding Process was dropped")]
    ProcessDropped,
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, time::Duration};

    use super::*;
    use tracing_test::traced_test;

    const CRATE_DIR: &str = env!("CARGO_MANIFEST_DIR");

    fn get_tests_dir() -> PathBuf {
        Path::new(CRATE_DIR).join("tests_dir")
    }

    fn get_numbers_script_path() -> PathBuf {
        if cfg!(target_os = "linux") {
            return get_tests_dir().join("numbers.sh");
        } else if cfg!(target_os = "windows") {
            return get_tests_dir().join("numbers.ps1");
        }
        panic!("Uncovered target_os.");
    }

    fn get_numbers_script_with_error_code_path() -> PathBuf {
        if cfg!(target_os = "linux") {
            return get_tests_dir().join("numbers_with_error_code.sh");
        } else if cfg!(target_os = "windows") {
            return get_tests_dir().join("numbers_with_error_code.ps1");
        }
        panic!("Uncovered target_os.");
    }

    fn program() -> &'static str {
        if cfg!(target_os = "linux") {
            return "bash";
        } else if cfg!(target_os = "windows") {
            return "powershell.exe";
        }
        panic!("Uncovered target_os.");
    }

    fn create_process_args(
        program: String,
        path: PathBuf,
    ) -> NewProcessArgs<Vec<String>, String, String> {
        let path_str = path
            .to_str()
            .expect("Error converting path to string.")
            .to_owned();
        NewProcessArgs {
            program,
            args: vec![path_str],
            current_dir: ".".to_owned(),
            kill_on_drop: true,
            stdout_sender: None,
            stderr_sender: None,
        }
    }

    fn create_numbers_process() -> (Process, ProcessHandler) {
        Process::new("numbers_process".into())
    }

    fn create_number_process_run_args() -> NewProcessArgs<Vec<String>, String, String> {
        let path = get_numbers_script_path();
        create_process_args(program().to_owned(), path)
    }

    fn create_numbers_process_with_error_code() -> (Process, ProcessHandler) {
        Process::new("numbers_process_with_error_code".into())
    }

    fn create_number_process_with_error_code_run_args(
    ) -> NewProcessArgs<Vec<String>, String, String> {
        let path = get_numbers_script_with_error_code_path();
        create_process_args(program().to_owned(), path)
    }

    fn create_non_existing_process() -> (Process, ProcessHandler) {
        Process::new("non_existing_process".into())
    }

    fn create_non_existing_process_run_args() -> NewProcessArgs<Vec<String>, String, String> {
        let path = PathBuf::from("non_existing_process");
        create_process_args("non_existing_process".to_owned(), path)
    }

    #[tokio::test]
    #[traced_test]
    async fn run_non_existing_process_and_expect_not_found() {
        let (mut process, _) = create_non_existing_process();
        let args = create_non_existing_process_run_args();

        let result = process.run(args).await;

        match result {
            Ok(_) => panic!("Process should not be created."),
            Err(error) => match error {
                ProcessRunError::CouldNotCreateProcess(io_error) => match io_error.kind() {
                    std::io::ErrorKind::NotFound => {}
                    _ => panic!("Unexpected error kind: {:?}", io_error.kind()),
                },
                _ => panic!("Unexpected error: {:?}", error),
            },
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn run_numbers_script_and_kill_before_termination_and_expect_killed() {
        let (mut process, mut handler) = create_numbers_process();
        let args = create_number_process_run_args();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            handler.cancel().await.expect("Error cancelling process.");
        });

        let result = process.run(args).await;

        match result {
            Ok(Status::Terminated(TerminationStatus::Killed)) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn run_numbers_script_and_kill_before_start_and_expect_killed() {
        let (mut process, mut handler) = create_numbers_process();
        let args = create_number_process_run_args();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let result = process.run(args).await;

            match result {
                Ok(Status::Terminated(TerminationStatus::Killed)) => {}
                Err(e) => panic!("Unexpected error: {:?}", e),
                _ => panic!("Unexpected result: {:?}", result),
            }
        });

        handler.cancel().await.expect("Error cancelling process.");
    }

    #[tokio::test]
    #[traced_test]
    async fn run_numbers_script_and_kill_after_termination_and_expect_terminated_successfully() {
        let (mut process, mut handler) = create_numbers_process();
        let args = create_number_process_run_args();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            handler.cancel().await.expect("Error cancelling process.");
        });

        let result = process.run(args).await;

        match result {
            Ok(Status::Terminated(TerminationStatus::TerminatedSuccessfully)) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn run_numbers_script_with_error_code_and_expect_error_code_1() {
        let (mut process, _handler) = create_numbers_process_with_error_code();
        let args = create_number_process_with_error_code_run_args();

        let result = process.run(args).await;

        match result {
            Ok(Status::Terminated(TerminationStatus::TerminatedWithError(
                TerminationWithErrorStatus::TerminatedWithErrorCode(code),
            ))) => {
                assert_eq!(code, 1);
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn cancel_a_dropped_process_and_expect_error() {
        let (process, mut handler) = create_numbers_process();

        drop(process);

        match handler.cancel().await {
            Err(CancellationError::ProcessDropped) => {}
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn drop_handler_and_expect_killed_by_dropping_handler() {
        let (mut process, _) = create_numbers_process_with_error_code();
        let args = create_number_process_with_error_code_run_args();

        let result = process.run(args).await;

        match result {
            Ok(Status::Terminated(TerminationStatus::KilledByDroppingHandler)) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn cancel_a_process_twice_and_excpect_error() {
        let (mut process, mut handler) = create_numbers_process();
        let args = create_number_process_run_args();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            handler.cancel().await.expect("Error cancelling process.");

            match handler.cancel().await {
                Err(CancellationError::AlreadyCancelled) => {}
                _ => panic!("Unexpected result"),
            }
        });

        process.run(args).await.expect("Error running process.");
    }
}
