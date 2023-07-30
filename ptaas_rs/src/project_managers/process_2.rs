use std::{
    ffi::OsStr,
    io::Error as IoError,
    path::Path,
    process::{ExitStatus, Stdio},
    sync::Arc,
};

use thiserror::Error as ThisError;
use tokio::{
    process::{Child, ChildStderr, ChildStdout, Command},
    sync::{
        oneshot::{Receiver, Sender},
        RwLock,
    },
};
use tokio_util::sync::CancellationToken;

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
pub struct NewProcessArgs<I, S, P, T> {
    pub program: S,
    pub args: I,
    pub current_dir: P,
    pub stdin: T,
    pub stdout: T,
    pub stderr: T,
    pub kill_on_drop: bool,
}

pub struct Process {
    token: CancellationToken,
    status: Arc<RwLock<Status>>,
    given_id: String,
    child_killed_successfuly: bool,
    /// Option so we can take it
    cancel_channel_sender: Option<Sender<Option<ProcessKillAndWaitError>>>,
}

pub struct ProcessHandler {
    token: CancellationToken,
    status: Arc<RwLock<Status>>,
    /// Option so we can take it
    cancel_channel_receiver: Option<Receiver<Option<ProcessKillAndWaitError>>>,
}

impl ProcessHandler {
    /// Will deadlock if corresponding `Process` has not been started.
    pub async fn cancel(&mut self) -> Result<Option<ProcessKillAndWaitError>, CancellationError> {
        if self.token.is_cancelled() {
            return Err(CancellationError::AlreadyCancelled);
        }

        let cancel_channel_receiver = self
            .cancel_channel_receiver
            .take()
            .ok_or(CancellationError::AlreayTriedToCancel)?;

        self.token.cancel();

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
        let token = CancellationToken::new();
        let status = Arc::new(RwLock::new(Status::Created));
        let (cancel_channel_sender, cancel_channel_receiver) = tokio::sync::oneshot::channel();

        let process = Self {
            token: token.clone(),
            status: status.clone(),
            given_id,
            child_killed_successfuly: false,
            cancel_channel_sender: Some(cancel_channel_sender),
        };

        let process_handler = ProcessHandler {
            token,
            status,
            cancel_channel_receiver: Some(cancel_channel_receiver),
        };

        (process, process_handler)
    }

    pub async fn write_status(&self, status: Status) {
        *self.status.write().await = status;
    }

    pub async fn run<I, S, P, T>(
        &mut self,
        new_process_args: NewProcessArgs<I, S, P, T>,
    ) -> Result<Status, ProcessRunError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        P: AsRef<Path>,
        T: Into<Stdio>,
    {
        let cancel_channel_sender = self
            .cancel_channel_sender
            .take()
            .ok_or(ProcessRunError::AlreadyStarted)?;

        let mut child = Command::new(new_process_args.program)
            .args(new_process_args.args)
            .current_dir(new_process_args.current_dir)
            .stdin(new_process_args.stdin)
            .stdout(new_process_args.stdout)
            .stderr(new_process_args.stderr)
            .kill_on_drop(new_process_args.kill_on_drop)
            .spawn()
            .map_err(ProcessRunError::CouldNotCreateProcess)?;

        self.write_status(Status::Running).await;

        // do io piping

        tokio::select! {
            _ = self.token.cancelled() => {
                println!("##1");
                match self.check_if_still_running_and_kill_and_wait(child).await {
                    Ok(exit_status) => {
                        self.write_status_on_exit_status(exit_status).await;

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

            result_exit_status = child.wait() => {
                let exit_status = result_exit_status.map_err(ProcessRunError::CouldNotWaitForProcess)?;
                self.write_status_on_exit_status(exit_status).await;
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

    async fn get_status_on_exit_status(&self, exit_status: ExitStatus) -> Status {
        if exit_status.success() {
            return Status::Terminated(TerminationStatus::TerminatedSuccessfully);
        };

        match exit_status.code() {
            Some(code) => match code {
                1 if cfg!(target_os = "windows") && self.child_killed_successfuly => {
                    Status::Terminated(TerminationStatus::Killed)
                }
                _ => Status::Terminated(TerminationStatus::TerminatedWithError(
                    TerminationWithErrorStatus::TerminatedWithErrorCode(code),
                )),
            },
            None if cfg!(target_os = "linux") && self.child_killed_successfuly => {
                Status::Terminated(TerminationStatus::Killed)
            }
            _ => Status::Terminated(TerminationStatus::TerminatedWithError(
                TerminationWithErrorStatus::TerminatedWithUnknownErrorCode,
            )),
        }
    }

    async fn write_status_on_exit_status(&self, exit_status: ExitStatus) {
        let status = self.get_status_on_exit_status(exit_status).await;
        self.write_status(status).await;
    }

    pub async fn status(&self) -> Status {
        self.status.read().await.clone()
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
    #[error("Corresponding ProcessHandler was dropped")]
    HandlerDropped,
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
    ) -> NewProcessArgs<Vec<String>, String, String, Stdio> {
        let path_str = path
            .to_str()
            .expect("Error converting path to string.")
            .to_owned();
        NewProcessArgs {
            program,
            args: vec![path_str],
            current_dir: ".".to_owned(),
            stdin: Stdio::piped(),
            stdout: Stdio::inherit(),
            stderr: Stdio::inherit(),
            kill_on_drop: true,
        }
    }

    fn create_numbers_process() -> (Process, ProcessHandler) {
        Process::new("numbers_process".into())
    }

    fn create_number_process_run_args() -> NewProcessArgs<Vec<String>, String, String, Stdio> {
        let path = get_numbers_script_path();
        create_process_args(program().to_owned(), path)
    }

    fn create_numbers_process_with_error_code() -> (Process, ProcessHandler) {
        Process::new("numbers_process_with_error_code".into())
    }

    fn create_number_process_with_error_code_run_args(
    ) -> NewProcessArgs<Vec<String>, String, String, Stdio> {
        let path = get_numbers_script_with_error_code_path();
        create_process_args(program().to_owned(), path)
    }

    fn create_non_existing_process() -> (Process, ProcessHandler) {
        Process::new("non_existing_process".into())
    }

    fn create_non_existing_process_run_args() -> NewProcessArgs<Vec<String>, String, String, Stdio>
    {
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
        let (mut process, _) = create_numbers_process_with_error_code();
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
