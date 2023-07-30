use std::{
    ffi::OsStr,
    io::Error as IoError,
    path::Path,
    process::{ExitStatus, Stdio},
    sync::Arc,
};

use thiserror::Error as ThisError;
use tokio::{
    process::{ChildStderr, ChildStdout, Command},
    sync::RwLock,
};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub enum Status {
    Created,
    Running,
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
    child_terminated_and_awaited_successfuly: bool,
    child_killed_successfuly: bool,
}

pub struct ProcessHandler {
    token: CancellationToken,
    status: Arc<RwLock<Status>>,
}

impl ProcessHandler {
    pub fn cancel(&self) {
        self.token.cancel();
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
    pub fn new(mut new_process_args: NewProcessArgs<I, S, P, T>) -> (Self, ProcessHandler) {
        let token = CancellationToken::new();
        let status = Arc::new(RwLock::new(Status::Created));

        let process = Self {
            token: token.clone(),
            status: status.clone(),
            given_id: new_process_args.given_id.take(),
            new_process_args: Some(new_process_args),
            child_terminated_and_awaited_successfuly: false,
            child_killed_successfuly: false,
        };
        let process_handler = ProcessHandler { token, status };
        (process, process_handler)
    }

    pub async fn write_status(&self, status: Status) {
        *self.status.write().await = status;
    }

    pub async fn run(&mut self) -> Result<(), RunError> {
        let new_process_args = self
            .new_process_args
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
                // check if child is running
                let try_wait_result = child.try_wait().map(|option_ex_status| async move{
                    match option_ex_status {
                        Some(exit_status) => {
                            self.write_status_on_ex_status(exit_status).await;
                        }
                        None => {
                            // child is still running
                            // kill child
                            let kill_result = child.kill().await;

                            let wait_result = child.wait().await;

                        }
                    }});
            }

            result_exit_status = child.wait() => {
                // set status
                match result_exit_status {
                    Ok(exit_status) => {
                        self.write_status_on_ex_status(exit_status).await;
                    }
                    Err(e) => {
                        // could not wait
                    }
                }

            }
        }

        Ok(())
    }

    async fn write_status_on_ex_status(&mut self, exit_status: ExitStatus) {
        if exit_status.success() {
            self.write_status(Status::TerminatedSuccessfully).await;
        } else {
            match exit_status.code() {
                Some(code) => match code {
                    1 if cfg!(target_os = "windows") && self.child_killed_successfuly => {
                        self.write_status(Status::Killed).await;
                    }
                    _ => {
                        self.write_status(Status::TerminatedWithError(
                            TerminationWithErrorStatus::TerminatedWithErrorCode(code),
                        ))
                        .await;
                    }
                },
                None if cfg!(target_os = "linux") && self.child_killed_successfuly => {
                    self.write_status(Status::Killed).await;
                }
                _ => {
                    self.write_status(Status::TerminatedWithError(
                        TerminationWithErrorStatus::TerminatedWithUnknownErrorCode,
                    ))
                    .await
                }
            }
        }
        self.child_terminated_and_awaited_successfuly = true;
    }
}

#[derive(ThisError, Debug)]
pub enum RunError {
    #[error("Process is already running")]
    AlreadyRunning,
    #[error("Could not create process: {0}")]
    CouldNotCreateProcess(#[source] IoError),
}

#[derive(ThisError, Debug)]
pub enum ChildWaitError {
    #[error("Child was not created")]
    ChildNotCreated,

    #[error("Could not wait for process: {0}")]
    CouldNotWaitForProcess(
        #[source]
        #[from]
        IoError,
    ),
}
