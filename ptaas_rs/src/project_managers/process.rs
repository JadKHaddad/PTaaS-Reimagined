use std::{
    ffi::OsStr,
    path::Path,
    process::{ExitStatus, Stdio},
};

use thiserror::Error as ThisError;
use tokio::process::{Child, Command};

#[derive(Debug, Clone)]
pub enum Status {
    Running,
    TerminatedSuccessfully,
    TerminatedWithError(i32),
    TerminatedWithUnknownError,
}

#[derive(Debug)]
pub struct Process {
    child: Child,
    status: Status,
    child_awaited: bool,
}

#[derive(ThisError, Debug)]
pub enum ProcessCreateError {
    #[error("Could not create process: {0}")]
    CouldNotCreateProcess(#[source] std::io::Error),
}

#[derive(ThisError, Debug)]
pub enum ProcessKillAndWaitError {
    #[error("Could not kill process: {0}")]
    CouldNotKillProcess(#[source] std::io::Error),
    #[error("Could not wait for process: {0}")]
    CouldNotWaitForProcess(#[source] std::io::Error),
}

impl Process {
    pub async fn new<I, S, P, T>(
        program: S,
        args: I,
        current_dir: P,
        stdin: T,
        stdout: T,
        stderr: T,
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
            .spawn()
            .map_err(|e| ProcessCreateError::CouldNotCreateProcess(e))?;

        Ok(Self {
            child,
            status: Status::Running,
            child_awaited: false,
        })
    }

    pub async fn kill_and_wait(mut self) -> Result<(), ProcessKillAndWaitError> {
        self.child
            .kill()
            .await
            .map_err(|error| ProcessKillAndWaitError::CouldNotKillProcess(error))?;

        self.child
            .wait()
            .await
            .and_then(|ex_status| {
                self.set_status_on_ex_status(ex_status);
                Ok(())
            })
            .map_err(|error| ProcessKillAndWaitError::CouldNotWaitForProcess(error))
    }

    /// Maybe useful if 'kill_and_wait' fails with 'CouldNotKillProcess' error
    pub fn start_kill(&mut self) -> Result<(), std::io::Error> {
        tracing::warn!(
            id = self.id(),
            "Sending kill signal to process. Not awaited processes may cause zombie processes"
        );
        self.child.start_kill()
    }

    fn set_status_on_ex_status(&mut self, ex_status: ExitStatus) {
        if ex_status.success() {
            self.status = Status::TerminatedSuccessfully;
            tracing::debug!(id = self.id(), "Process terminated successfully");
        } else {
            match ex_status.code() {
                Some(code) => {
                    self.status = Status::TerminatedWithError(code);
                    tracing::debug!(id = self.id(), code, "Process terminated with error");
                }
                None => {
                    self.status = Status::TerminatedWithUnknownError;
                    tracing::debug!(id = self.id(), "Process terminated with unknown error");
                }
            }
        }
        self.child_awaited = true;
    }

    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    pub async fn status(&mut self) -> Result<&Status, std::io::Error> {
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
}

impl Drop for Process {
    fn drop(&mut self) {
        if !self.child_awaited {
            tracing::warn!(id = self.id(), "Process was dropped without being awaited. Not awaited processes may cause zombie processes");
        }
    }
}