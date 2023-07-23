use std::{
    ffi::OsStr,
    io::Error as IoError,
    path::Path,
    process::{ExitStatus, Stdio},
    time::Duration,
};

use thiserror::Error as ThisError;
use tokio::{
    fs,
    io::{self, AsyncBufReadExt, AsyncRead, AsyncWriteExt},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
};

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

#[derive(ThisError, Debug)]
pub enum ProcessCreateError {
    #[error("Could not create process: {0}")]
    CouldNotCreateProcess(#[source] IoError),
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

/// Ensure killing the process before dropping it.
impl Process {
    pub fn new<I, S, P, T>(
        new_process_args: NewProcessArgs<I, S, P, T>,
    ) -> Result<Self, ProcessCreateError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        P: AsRef<Path>,
        T: Into<Stdio>,
    {
        let child = Command::new(new_process_args.program)
            .args(new_process_args.args)
            .current_dir(new_process_args.current_dir)
            .stdin(new_process_args.stdin)
            .stdout(new_process_args.stdout)
            .stderr(new_process_args.stderr)
            .kill_on_drop(new_process_args.kill_on_drop)
            .spawn()
            .map_err(ProcessCreateError::CouldNotCreateProcess)?;

        Ok(Self {
            child,
            given_id: new_process_args.given_id,
            status: Status::Running,
            child_terminated_and_awaited_successfuly: false,
            child_killed_successfuly: false,
            kill_on_drop: new_process_args.kill_on_drop,
        })
    }

    #[allow(dead_code)]
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

    /// Maybe useful if killing the process using `kill` failes.
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

    pub async fn wait_with_output_and_set_status(&mut self) -> Result<Output, IoError> {
        self.wait_and_set_status().await?;
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

    async fn do_pipe_io_to_file<T: AsyncRead + Unpin + Send + 'static>(
        given_id: Option<String>,
        file_path: &Path,
        io: Option<T>,
    ) -> Result<(), IoError> {
        let file_path_string = file_path
            .to_str()
            .unwrap_or_else(|| {
                tracing::error!(
                    given_id,
                    ?file_path,
                    "Error converting file path to string."
                );
                ""
            })
            .to_owned();

        let mut file = fs::File::create(file_path).await?;

        tokio::spawn(async move {
            tracing::debug!(given_id, file = file_path_string, "Stream opened.");

            if let Some(out) = io {
                let reader = io::BufReader::new(out);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    file.write_all(line.as_bytes())
                        .await
                        .unwrap_or_else(|error| {
                            tracing::error!(%error, given_id, file=file_path_string, "Error writing to file.");
                        });

                    file.write_all(b"\n").await.unwrap_or_else(|error| {
                        tracing::error!(%error, given_id, file=file_path_string, "Error writing to file.");
                    });
                }

                file.flush().await.unwrap_or_else(|error| {
                    tracing::error!(%error, given_id, file=file_path_string, "Error flushing file.");
                });

                tracing::debug!(given_id, file = file_path_string, "Stream closed.");
            }
        });

        Ok(())
    }

    pub async fn do_pipe_stdout_to_file(&mut self, file_path: &Path) -> Result<(), IoError> {
        Process::do_pipe_io_to_file(self.given_id.clone(), file_path, self.stdout()).await
    }

    pub async fn do_pipe_stderr_to_file(&mut self, file_path: &Path) -> Result<(), IoError> {
        Process::do_pipe_io_to_file(self.given_id.clone(), file_path, self.stderr()).await
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

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use tracing_test::traced_test;

    const CRATE_DIR: &str = env!("CARGO_MANIFEST_DIR");

    fn get_tests_dir() -> PathBuf {
        Path::new(CRATE_DIR).join("tests_dir")
    }

    fn get_numbers_script_path() -> PathBuf {
        get_tests_dir().join("numbers.ps1")
    }

    fn create_numbers_process(
        stdin: Stdio,
        stdout: Stdio,
        stderr: Stdio,
    ) -> Result<Process, ProcessCreateError> {
        let numbers_script_path = get_numbers_script_path();
        let numbers_script_path_str = numbers_script_path
            .to_str()
            .expect("Error converting path to string.");

        let args = NewProcessArgs {
            given_id: Some("numbers.ps1".into()),
            program: "powershell.exe",
            args: vec![numbers_script_path_str],
            current_dir: ".",
            stdin,
            stdout,
            stderr,
            kill_on_drop: true,
        };

        Process::new(args)
    }

    fn create_non_existing_process() -> Result<Process, ProcessCreateError> {
        let args = NewProcessArgs {
            given_id: Some("non_existing_process".into()),
            program: "non_existing_process",
            args: vec![],
            current_dir: ".",
            stdin: Stdio::null(),
            stdout: Stdio::null(),
            stderr: Stdio::null(),
            kill_on_drop: true,
        };

        Process::new(args)
    }

    fn create_numbers_process_with_panic() -> Process {
        create_numbers_process(Stdio::null(), Stdio::null(), Stdio::null())
            .expect("Error creating process.")
    }

    #[tokio::test]
    #[traced_test]
    async fn run_non_existing_process() {
        match create_non_existing_process() {
            Ok(_) => panic!("Process should not be created."),
            Err(error) => match error {
                ProcessCreateError::CouldNotCreateProcess(io_error) => match io_error.kind() {
                    std::io::ErrorKind::NotFound => {}
                    _ => panic!("Unexpected error kind: {:?}", io_error.kind()),
                },
            },
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn run_numbers_script_and_kill_before_termination() {
        let mut numbers_process = create_numbers_process_with_panic();

        tokio::time::sleep(Duration::from_secs(2)).await;

        numbers_process
            .check_status_and_kill_and_wait_and_set_status()
            .await
            .expect("Error killing process.");

        match numbers_process.status() {
            Ok(status) => match status {
                Status::TerminatedWithUnknownError => if cfg!(target_os = "linux") {},
                Status::TerminatedWithError(_) => if cfg!(target_os = "windows") {},
                Status::TerminatedSuccessfully => panic!("Unexpected status: {:?}", status),
                _ => panic!("Uncovered case"),
            },
            _ => panic!("Error getting status."),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn run_numbers_script_and_kill_after_termination() {
        let mut numbers_process = create_numbers_process_with_panic();

        tokio::time::sleep(Duration::from_secs(5)).await;

        numbers_process
            .check_status_and_kill_and_wait_and_set_status()
            .await
            .expect("Error killing process.");

        match numbers_process.status() {
            Ok(status) => match status {
                Status::TerminatedSuccessfully => {}
                _ => panic!("Unexpected status: {:?}", status),
            },
            _ => panic!("Error getting status."),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn run_numbers_script_with_less_timeout() {
        let mut numbers_process = create_numbers_process_with_panic();

        match numbers_process
            .wait_with_timeout_and_output_and_set_status(Duration::from_secs(2))
            .await
        {
            Ok(output) => match output.status {
                Status::TerminatedWithUnknownError => if cfg!(target_os = "linux") {},
                Status::TerminatedWithError(_) => if cfg!(target_os = "windows") {},
                Status::TerminatedSuccessfully => panic!("Unexpected status: {:?}", output.status),
                _ => panic!("Uncovered case"),
            },
            _ => panic!("Error waiting for process."),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn run_numbers_script_with_more_timeout() {
        let mut numbers_process = create_numbers_process_with_panic();

        match numbers_process
            .wait_with_timeout_and_output_and_set_status(Duration::from_secs(5))
            .await
        {
            Ok(output) => match output.status {
                Status::TerminatedSuccessfully => {}
                _ => panic!("unexpected status: {:?}", output.status),
            },
            _ => panic!("Error waiting for process."),
        }
    }
}
