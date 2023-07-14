use std::{
    ffi::OsStr,
    io::{Error as IoError, ErrorKind},
    path::Path,
    process::{ExitStatus, Stdio},
    time::Duration,
};

use thiserror::Error as ThisError;
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

// TODO: id is lost when process is no longer running!. use a given id in the constructor.

#[derive(Debug, Clone)]
pub enum Status {
    Running,
    TerminatedSuccessfully,
    TerminatedWithError(i32),
    TerminatedWithUnknownError,
}

#[derive(Debug)]
pub struct Output {
    pub status: Status,
    pub stdout: Option<ChildStdout>,
    pub stderr: Option<ChildStderr>,
}

#[derive(Debug)]
pub struct Process {
    child: Child,
    status: Status,
    child_terminated_and_awaited_successfuly: bool,
    child_killed_successfuly: bool,
    kill_on_drop: bool,
}

#[derive(ThisError, Debug)]
pub enum ProcessCreateError {
    #[error("Could not create process: {0}")]
    CouldNotCreateProcess(#[source] IoError),
}
#[derive(ThisError, Debug)]
pub enum ProgramExistsError {
    #[error("Could not create process: {0}")]
    CouldNotCreateProcess(
        #[source]
        #[from]
        ProcessCreateError,
    ),

    #[error("Could not kill and wait for process: {0}")]
    ProcessKillAndWaitError(
        #[source]
        #[from]
        ProcessKillAndWaitError,
    ),
}

#[derive(ThisError, Debug)]
pub enum ProcessKillAndWaitError {
    #[error("Could not kill process: {0}")]
    CouldNotKillProcess(#[source] IoError),
    #[error("Could not wait for process: {0}")]
    CouldNotWaitForProcess(#[source] IoError),
}

/// Ensure calling `kill_and_wait_and_set_status` on the process before dropping it.
impl Process {
    pub async fn new<I, S, P, T>(
        program: S,
        args: I,
        current_dir: P,
        stdin: T,
        stdout: T,
        stderr: T,
        kill_on_drop: bool,
    ) -> Result<Self, ProcessCreateError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        P: AsRef<Path>,
        T: Into<Stdio>,
    {
        let child = Command::new(program)
            .args(args)
            .current_dir(current_dir)
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr)
            .kill_on_drop(kill_on_drop)
            .spawn()
            .map_err(|e| ProcessCreateError::CouldNotCreateProcess(e))?;

        Ok(Self {
            child,
            status: Status::Running,
            child_terminated_and_awaited_successfuly: false,
            child_killed_successfuly: false,
            kill_on_drop,
        })
    }

    /// Spawns a process and kills it immediately after a successful spawn returning `true`.
    /// Returns `false`, if `ErrorKind::NotFound` is returned.
    /// Otherwise returns an error.
    /// An error does not necessarily mean that the program does not exist.
    /// `Ok(true)` means that the program exists.
    /// Filtering out `ErrorKind::PermissionDenied` on `kill_and_wait_and_set_status`,
    /// because the program could exit immediately after spawning.
    pub async fn program_exists<S, T>(
        program: S,
        stdin: T,
        stdout: T,
        stderr: T,
    ) -> Result<bool, ProgramExistsError>
    where
        S: AsRef<OsStr>,
        T: Into<Stdio>,
    {
        match Self::new(program, [], ".", stdin, stdout, stderr, true).await {
            Ok(mut process) => {
                process
                    .kill_and_wait_and_set_status()
                    .await
                    .or_else(|error| {
                        if let ProcessKillAndWaitError::CouldNotKillProcess(ref io_error) = error {
                            if let ErrorKind::PermissionDenied = io_error.kind() {
                                return Ok(());
                            }
                        }
                        return Err(error);
                    })?;

                Ok(true)
            }
            Err(p_error) => match p_error {
                ProcessCreateError::CouldNotCreateProcess(ref io_error) => match io_error.kind() {
                    ErrorKind::NotFound => return Ok(false),
                    _ => return Err(ProgramExistsError::CouldNotCreateProcess(p_error)),
                },
            },
        }
    }

    /// Kill may fail if the process has already exited.
    pub async fn kill_and_wait_and_set_status(&mut self) -> Result<(), ProcessKillAndWaitError> {
        self.kill()
            .await
            .map_err(|error| ProcessKillAndWaitError::CouldNotKillProcess(error))?;

        self.wait_and_set_status()
            .await
            .map_err(|error| ProcessKillAndWaitError::CouldNotWaitForProcess(error))
    }

    async fn kill(&mut self) -> Result<(), IoError> {
        self.child.kill().await.and_then(|_| {
            self.child_killed_successfuly = true;
            Ok(())
        })
    }

    async fn wait_and_set_status(&mut self) -> Result<(), IoError> {
        self.child.wait().await.and_then(|ex_status| {
            self.set_status_on_ex_status(ex_status);
            Ok(())
        })
    }

    /// Maybe useful if 'kill_and_wait_and_set_status' fails with 'CouldNotKillProcess' error.
    pub fn start_kill(&mut self) -> Result<(), IoError> {
        tracing::warn!(id = self.id(), "Sending kill signal to process.");
        self.child.start_kill()
    }

    fn set_status_on_ex_status(&mut self, ex_status: ExitStatus) -> &Status {
        if ex_status.success() {
            self.status = Status::TerminatedSuccessfully;
            tracing::debug!(id = self.id(), "Process terminated successfully.");
        } else {
            match ex_status.code() {
                Some(code) => {
                    self.status = Status::TerminatedWithError(code);
                    tracing::debug!(id = self.id(), code, "Process terminated with error.");
                }
                None => {
                    self.status = Status::TerminatedWithUnknownError;
                    tracing::debug!(id = self.id(), "Process terminated with unknown error.");
                }
            }
        }
        self.child_terminated_and_awaited_successfuly = true;
        &self.status
    }

    pub fn status(&mut self) -> Result<&Status, IoError> {
        self.child.try_wait().and_then(|option_ex_status| {
            match option_ex_status {
                Some(ex_status) => {
                    self.set_status_on_ex_status(ex_status);
                }
                None => {
                    self.status = Status::Running;
                }
            }
            Ok(&self.status)
        })
    }

    /// After calling this funtion `stdout()` and `stderr()` will return `None`.
    /// If you want to use these values, use the returned `Output` instead.
    pub async fn wait_with_timeout_and_output(
        &mut self,
        duration: Duration,
    ) -> Result<Output, ProcessKillAndWaitError> {
        tokio::select! {
            _ = tokio::time::sleep(duration) => {
                self.kill_and_wait_and_set_status().await?;
                tracing::warn!(id = self.id(), "Process killed after timeout.");
            }
            _ = self.wait_and_set_status() => {
                tracing::debug!(id = self.id(), "Process terminated before timeout.");
            }
        }

        Ok(Output {
            status: self.status.clone(),
            stdout: self.stdout(),
            stderr: self.stderr(),
        })
    }

    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    pub fn stdin(&mut self) -> Option<ChildStdin> {
        self.child.stdin.take()
    }

    pub fn stdout(&mut self) -> Option<ChildStdout> {
        self.child.stdout.take()
    }

    pub fn stderr(&mut self) -> Option<ChildStderr> {
        self.child.stderr.take()
    }
}

impl Drop for Process {
    /// Can not kill and wait for termination here, because these are async functions.
    fn drop(&mut self) {
        if !self.child_terminated_and_awaited_successfuly {
            if !self.child_killed_successfuly && self.kill_on_drop {
                tracing::warn!(id = self.id(), "Process was not explicitly killed and the status was not or could not be checked. Process may still be running. Sending kill signal to process.");
            }
            tracing::warn!(id = self.id(), "Process was dropped without being awaited. Not awaited processes may cause zombie processes.");
        }
    }
}
