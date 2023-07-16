use std::{
    ffi::OsStr,
    io::{Error as IoError, ErrorKind},
    path::Path,
    process::{ExitStatus, Stdio},
    time::Duration,
};

use thiserror::Error as ThisError;
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

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
    given_id: Option<String>,
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
    #[error("Could not check status of process: {0}")]
    CouldNotCheckStatus(#[source] IoError),
    #[error("Could not kill process: {0}")]
    CouldNotKillProcess(#[source] IoError),
    #[error("Could not wait for process: {0}")]
    CouldNotWaitForProcess(#[source] IoError),
}

/// Ensure calling `kill_and_wait_and_set_status` on the process before dropping it.
impl Process {
    pub fn new<I, S, P, T>(
        given_id: Option<String>,
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
            .map_err(ProcessCreateError::CouldNotCreateProcess)?;

        Ok(Self {
            child,
            given_id,
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
    pub async fn program_exists<S, T>(
        given_process_id: Option<String>,
        program: S,
        stdin: T,
        stdout: T,
        stderr: T,
    ) -> Result<bool, ProgramExistsError>
    where
        S: AsRef<OsStr>,
        T: Into<Stdio>,
    {
        let given_id = Some(given_process_id.unwrap_or_else(|| String::from("`program_exists`")));
        match Self::new(given_id, program, [], ".", stdin, stdout, stderr, true) {
            Ok(mut process) => {
                process.check_status_and_kill_and_wait().await?;
                Ok(true)
            }
            Err(p_error) => match p_error {
                ProcessCreateError::CouldNotCreateProcess(ref io_error) => match io_error.kind() {
                    ErrorKind::NotFound => Ok(false),
                    _ => Err(ProgramExistsError::CouldNotCreateProcess(p_error)),
                },
            },
        }
    }

    async fn check_status_and_kill_and_wait(&mut self) -> Result<(), ProcessKillAndWaitError> {
        self.check_status_and_kill().await?;
        self.wait()
            .await
            .map(|_| ())
            .map_err(ProcessKillAndWaitError::CouldNotWaitForProcess)
    }

    /// Will only attempt to kill the process if it is running.
    async fn check_status_and_kill(&mut self) -> Result<(), ProcessKillAndWaitError> {
        let status = self
            .status()
            .map_err(ProcessKillAndWaitError::CouldNotCheckStatus)?;

        match status {
            Status::Running => {
                self.kill()
                    .await
                    .map_err(ProcessKillAndWaitError::CouldNotKillProcess)?;
            }
            _ => {
                tracing::warn!(
                    id = self.id(),
                    given_id = self.given_id(),
                    "Trying to kill a process that is not running. Ignoring."
                );
            }
        }

        Ok(())
    }

    pub async fn check_status_and_kill_and_wait_and_set_status(
        &mut self,
    ) -> Result<(), ProcessKillAndWaitError> {
        self.check_status_and_kill().await?;
        self.wait_and_set_status()
            .await
            .map_err(ProcessKillAndWaitError::CouldNotWaitForProcess)
    }

    async fn kill(&mut self) -> Result<(), IoError> {
        self.child.kill().await.map(|_| {
            self.child_killed_successfuly = true;
        })
    }

    async fn wait(&mut self) -> Result<ExitStatus, IoError> {
        self.child.wait().await
    }

    async fn wait_and_set_status(&mut self) -> Result<(), IoError> {
        self.wait().await.map(|ex_status| {
            self.set_status_on_ex_status(ex_status);
        })
    }

    /// Maybe useful if 'kill_and_wait_and_set_status' fails with 'CouldNotKillProcess' error.
    pub fn start_kill(&mut self) -> Result<(), IoError> {
        tracing::warn!(
            id = self.id(),
            given_id = self.given_id(),
            "Sending kill signal to process."
        );
        self.child.start_kill()
    }

    fn set_status_on_ex_status(&mut self, ex_status: ExitStatus) -> &Status {
        if ex_status.success() {
            self.status = Status::TerminatedSuccessfully;
        } else {
            match ex_status.code() {
                Some(code) => {
                    self.status = Status::TerminatedWithError(code);
                }
                None => {
                    self.status = Status::TerminatedWithUnknownError;
                }
            }
        }
        self.child_terminated_and_awaited_successfuly = true;
        &self.status
    }

    /// Tries to wait for the process and sets the status.
    /// The status will not be updated otherwise.
    pub fn status(&mut self) -> Result<&Status, IoError> {
        self.child.try_wait().map(|option_ex_status| {
            match option_ex_status {
                Some(ex_status) => {
                    self.set_status_on_ex_status(ex_status);
                }
                None => {
                    self.status = Status::Running;
                }
            }
            &self.status
        })
    }

    /// After calling this funtion `stdout()` and `stderr()` will return `None`.
    /// If you want to use these values, use the returned `Output` instead.
    /// Depending on tokio's implementation of `select!`,
    /// it should not be possible to kill the process after it has terminated.
    pub async fn wait_with_timeout_and_output_and_set_status(
        &mut self,
        duration: Duration,
    ) -> Result<Output, ProcessKillAndWaitError> {
        tokio::select! {
            _ = tokio::time::sleep(duration) => {
                self.check_status_and_kill_and_wait_and_set_status().await?;
                tracing::warn!(id = self.id(), given_id = self.given_id(), "Process killed after timeout.");
            }
            _ = self.wait_and_set_status() => {
                tracing::debug!(id = self.id(), given_id = self.given_id(), "Process terminated before timeout.");
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

    pub fn given_id(&self) -> &Option<String> {
        &self.given_id
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
                tracing::warn!(id = self.id(), given_id = self.given_id(), "Process was not explicitly killed and the status was not or could not be checked. Process may still be running. Sending kill signal to process.");
            }
            tracing::warn!(id = self.id(), given_id = self.given_id(), "Process was dropped without being awaited. Not awaited processes may cause zombie processes.");
        }
    }
}

pub mod dev {
    use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};

    use super::*;
    pub async fn program_exists(program: &str) {
        match Process::program_exists(
            Some(format!("does {program} exist")),
            program,
            Stdio::null(),
            Stdio::null(),
            Stdio::null(),
        )
        .await
        {
            Ok(exists) => {
                if exists {
                    tracing::info!(program, "program exists.");
                } else {
                    tracing::info!(program, "program does not exist.");
                }
            }
            Err(error) => {
                tracing::error!(%error, program, "Error checking if program exists.");
            }
        }
    }

    fn create_numbers_process(stdout: Stdio) -> Result<Process, ProcessCreateError> {
        Process::new(
            Some("numbers.ps1".into()),
            "powershell.exe",
            vec!["./numbers.ps1"],
            ".",
            Stdio::inherit(),
            stdout,
            Stdio::inherit(),
            true,
        )
    }

    pub async fn run_numbers_script_and_kill_before_termination() {
        let mut p = match create_numbers_process(Stdio::inherit()) {
            Ok(p) => p,
            Err(error) => {
                tracing::error!(%error, "Error creating process.");
                return;
            }
        };
        tokio::time::sleep(Duration::from_secs(2)).await;
        match p.check_status_and_kill_and_wait_and_set_status().await {
            Ok(_) => {
                tracing::info!(
                    id = p.id(),
                    given_id = p.given_id(),
                    "Process killed and awaited."
                );
            }
            Err(error) => {
                tracing::error!(
                    %error,
                    id = p.id(),
                    given_id = p.given_id(),
                    "Error killing process."
                );
            }
        }
    }

    pub async fn run_numbers_script_and_kill_after_termination() {
        let mut p = match create_numbers_process(Stdio::inherit()) {
            Ok(p) => p,
            Err(error) => {
                tracing::error!(%error, "Error creating process.");
                return;
            }
        };
        tokio::time::sleep(Duration::from_secs(10)).await;
        match p.check_status_and_kill_and_wait_and_set_status().await {
            Ok(_) => {
                tracing::info!(
                    id = p.id(),
                    given_id = p.given_id(),
                    "Process killed and awaited."
                );
            }
            Err(error) => {
                tracing::error!(
                    %error,
                    id = p.id(),
                    given_id = p.given_id(),
                    "Error killing process."
                );
            }
        }
    }

    pub async fn run_numbers_script_with_less_timeout() {
        let mut p = match create_numbers_process(Stdio::inherit()) {
            Ok(p) => p,
            Err(error) => {
                tracing::error!(%error, "Error creating process.");
                return;
            }
        };
        match p
            .wait_with_timeout_and_output_and_set_status(Duration::from_secs(2))
            .await
        {
            Ok(output) => {
                tracing::info!(
                    id = p.id(),
                    given_id = p.given_id(),
                    ?output,
                    "Process terminated before timeout."
                );
            }
            Err(error) => {
                tracing::error!(
                    %error,
                    id = p.id(),
                    given_id = p.given_id(),
                    "Error waiting for process."
                );
            }
        }
    }

    pub async fn run_numbers_script_with_more_timeout() {
        let mut p = match create_numbers_process(Stdio::inherit()) {
            Ok(p) => p,
            Err(error) => {
                tracing::error!(%error, "Error creating process.");
                return;
            }
        };
        match p
            .wait_with_timeout_and_output_and_set_status(Duration::from_secs(10))
            .await
        {
            Ok(output) => {
                tracing::info!(
                    id = p.id(),
                    given_id = p.given_id(),
                    ?output,
                    "Process terminated before timeout."
                );
            }
            Err(error) => {
                tracing::error!(
                    %error,
                    id = p.id(),
                    given_id = p.given_id(),
                    "Error waiting for process."
                );
            }
        }
    }

    pub async fn run_numbers_script_and_pipe_output_to_file(file: &str) {
        let mut p = match create_numbers_process(Stdio::piped()) {
            Ok(p) => p,
            Err(error) => {
                tracing::error!(%error, "Error creating process.");
                return;
            }
        };
        let stdout = p.stdout();

        // Create a file to write the lines
        let mut file = match tokio::fs::File::create(file).await {
            Ok(file) => file,
            Err(error) => {
                tracing::error!(%error, "Error creating file.");
                return;
            }
        };

        tokio::spawn(async move {
            if let Some(stdout) = stdout {
                let reader = io::BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    file.write_all(line.as_bytes())
                        .await
                        .unwrap_or_else(|error| {
                            tracing::error!(%error, "Error writing to file.");
                        });
                    file.write_all(b"\n").await.unwrap_or_else(|error| {
                        tracing::error!(%error, "Error writing to file.");
                    });
                }
            }
        });

        let _ = p
            .wait_with_timeout_and_output_and_set_status(Duration::from_secs(10))
            .await;
    }

    pub async fn run_numbers_script_and_pipe_output_console() {
        let mut p = match create_numbers_process(Stdio::piped()) {
            Ok(p) => p,
            Err(error) => {
                tracing::error!(%error, "Error creating process.");
                return;
            }
        };
        let stdout = p.stdout();

        tokio::spawn(async move {
            if let Some(stdout) = stdout {
                let reader = io::BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    println!("{}", line);
                }
            }
        });

        let _ = p
            .wait_with_timeout_and_output_and_set_status(Duration::from_secs(10))
            .await;
    }

    pub async fn run_all() {
        tracing::info!("Running all examples.");
        tracing::info!("Running numbers script and killing before termination.");
        run_numbers_script_and_kill_before_termination().await;
        tracing::info!("Running numbers script and killing after termination.");
        run_numbers_script_and_kill_after_termination().await;
        tracing::info!("Running numbers script with less timeout.");
        run_numbers_script_with_less_timeout().await;
        tracing::info!("Running numbers script with more timeout.");
        run_numbers_script_with_more_timeout().await;
        tracing::info!("Running numbers script and piping output to file.");
        run_numbers_script_and_pipe_output_to_file("numbers.txt").await;
        tracing::info!("Running numbers script and piping output to console.");
        run_numbers_script_and_pipe_output_console().await;
        tracing::info!("checking if python.exe exists.");
        program_exists("python.exe").await;
        tracing::info!("checking if someprogram.exe exists.");
        program_exists("someprogram.exe").await;
    }
}
