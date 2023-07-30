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
    pub given_id: Option<String>,
    pub program: S,
    pub args: I,
    pub current_dir: P,
    pub stdin: T,
    pub stdout: T,
    pub stderr: T,
    pub kill_on_drop: bool,
}

pub struct Process<I, S, P, T> {
    token: CancellationToken,
    status: Arc<RwLock<Status>>,
    given_id: Option<String>,
    /// Option so we can take it
    new_process_args: Option<NewProcessArgs<I, S, P, T>>,
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
    pub async fn cancel(&mut self) -> Result<Option<ProcessKillAndWaitError>, CancellationError> {
        if self.token.is_cancelled() {
            return Err(CancellationError::AlreadyCancelled);
        }

        let cancel_channel_receiver = self
            .cancel_channel_receiver
            .take()
            .ok_or(CancellationError::AlreadyCancelled)?;

        self.token.cancel();

        cancel_channel_receiver
            .await
            .map_err(|_| CancellationError::CouldNotReceiveFromChannel)
    }

    pub async fn status(&self) -> Status {
        self.status.read().await.clone()
    }
}

impl<I, S, P, T> Process<I, S, P, T>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    P: AsRef<Path>,
    T: Into<Stdio>,
{
    #[must_use]
    pub fn new(mut new_process_args: NewProcessArgs<I, S, P, T>) -> (Self, ProcessHandler) {
        let token = CancellationToken::new();
        let status = Arc::new(RwLock::new(Status::Created));
        let (cancel_channel_sender, cancel_channel_receiver) = tokio::sync::oneshot::channel();

        let process = Self {
            token: token.clone(),
            status: status.clone(),
            given_id: new_process_args.given_id.take(),
            new_process_args: Some(new_process_args),
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

    pub async fn run(&mut self) -> Result<Status, RunError> {
        let new_process_args = self
            .new_process_args
            .take()
            .ok_or(RunError::AlreadyRunning)?;

        let cancel_channel_sender = self
            .cancel_channel_sender
            .take()
            .ok_or(RunError::AlreadyRunning)?;

        let mut child = Command::new(new_process_args.program)
            .args(new_process_args.args)
            .current_dir(new_process_args.current_dir)
            .stdin(new_process_args.stdin)
            .stdout(new_process_args.stdout)
            .stderr(new_process_args.stderr)
            .kill_on_drop(new_process_args.kill_on_drop)
            .spawn()
            .map_err(RunError::CouldNotCreateProcess)?;

        self.write_status(Status::Running).await;

        // do io piping

        tokio::select! {
            _ = self.token.cancelled() => {
                match self.check_if_still_running_and_kill_and_wait(child).await {
                    Ok(exit_status) => {
                        self.write_status_on_exit_status(exit_status).await;

                        if cancel_channel_sender.send(None).is_err() {
                            return Err(RunError::CouldNotSendThroughChannel);
                        }
                    }
                    Err(e) => {
                        if cancel_channel_sender.send(Some(e)).is_err() {
                            return Err(RunError::CouldNotSendThroughChannel);
                        }
                    }
                }
            }

            result_exit_status = child.wait() => {
                let exit_status = result_exit_status.map_err(RunError::CouldNotWaitForProcess)?;
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
pub enum RunError {
    #[error("Process is already running")]
    AlreadyRunning,
    #[error("Could not create process: {0}")]
    CouldNotCreateProcess(#[source] IoError),
    #[error("Could not wait for process: {0}")]
    CouldNotWaitForProcess(#[source] IoError),
    #[error("Could not send cancellation result through channel")]
    CouldNotSendThroughChannel,
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
    #[error("Could not receive cancellation result from channel")]
    CouldNotReceiveFromChannel,
}
