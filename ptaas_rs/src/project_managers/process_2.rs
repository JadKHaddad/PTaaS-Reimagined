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
    KilledByDroppingController,
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

/// Used `Process::run` to pass arguments, to improve readability.
#[derive(Debug)]
pub struct OsProcessArgs<I, S, P> {
    pub program: S,
    pub args: I,
    pub current_dir: P,
    pub stdout_sender: Option<mpsc::Sender<String>>,
    pub stderr_sender: Option<mpsc::Sender<String>>,
}

#[derive(Clone)]
struct StatusHolder {
    status: Arc<RwLock<Status>>,
}

impl StatusHolder {
    async fn overwrite(&self, status: Status) {
        *self.status.write().await = status;
    }

    async fn status(&self) -> Status {
        self.status.read().await.clone()
    }
}

pub struct ProcessController {
    status_holder: StatusHolder,
    /// Option so we can take it
    cancel_channel_sender: Option<oneshot::Sender<()>>,
    /// Option so we can take it
    cancel_status_channel_receiver: Option<oneshot::Receiver<Option<ProcessKillAndWaitError>>>,
}

impl ProcessController {
    /// Blocks until the corresponding `Process` is terminated.
    /// Will deadlock if the corresponding `Process` has not been started.
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
            .map_err(|_| CancellationError::ProcessTerminated)?;

        cancel_channel_receiver
            .await
            .map_err(|_| CancellationError::ProcessTerminated)
    }

    pub async fn status(&self) -> Status {
        self.status_holder.status().await
    }
}

pub struct Process {
    status_holder: StatusHolder,
    given_id: String,
    given_name: String,
    cancel_status_channel_sender: oneshot::Sender<Option<ProcessKillAndWaitError>>,
    cancel_channel_receiver: oneshot::Receiver<()>,
}

impl Process {
    #[must_use]
    pub fn new(given_id: String, given_name: String) -> (Self, ProcessController) {
        let status = Arc::new(RwLock::new(Status::Created));
        let status_holder = StatusHolder { status };

        let (cancel_status_channel_sender, cancel_status_channel_receiver) =
            tokio::sync::oneshot::channel();
        let (cancel_channel_sender, cancel_channel_receiver) = tokio::sync::oneshot::channel();

        let process = Self {
            status_holder: status_holder.clone(),
            given_id,
            given_name,
            cancel_status_channel_sender,
            cancel_channel_receiver,
        };

        let process_controller = ProcessController {
            status_holder,
            cancel_channel_sender: Some(cancel_channel_sender),
            cancel_status_channel_receiver: Some(cancel_status_channel_receiver),
        };

        (process, process_controller)
    }

    pub async fn run<I, S, P>(
        self,
        os_process_args: OsProcessArgs<I, S, P>,
    ) -> Result<Status, ProcessRunError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        P: AsRef<Path>,
    {
        let cancel_channel_sender = self.cancel_status_channel_sender;
        let cancel_channel_receiver = self.cancel_channel_receiver;

        let OsProcessArgs {
            program,
            args,
            current_dir,
            stdout_sender,
            stderr_sender,
        } = os_process_args;

        let mut child =
            Self::spawn_os_process(program, args, current_dir, &stdout_sender, &stderr_sender)
                .map_err(ProcessRunError::CouldNotSpawnOsProcess)?;

        self.status_holder.overwrite(Status::Running).await;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        Self::forward_ios_to_channels(
            stdout,
            stderr,
            stdout_sender,
            stderr_sender,
            self.given_id,
            self.given_name,
        );

        tokio::select! {
            result = cancel_channel_receiver => {
                if result.is_ok() {
                    // The process was explicitly cancelled by the controller
                    // Cancellation errors are sent to the controller and this function returns
                    match Self::check_if_still_running_and_kill_and_wait(child).await {
                        Ok((exit_status, child_killed_successfully)) => {
                            let new_status = Self::get_status_on_exit_status(exit_status, child_killed_successfully, false).await;
                            self.status_holder.overwrite(new_status).await;

                            cancel_channel_sender
                                .send(None).map_err(|_| ProcessRunError::ControllerDropped)?;
                        }
                        Err(e) => cancel_channel_sender.send(Some(e))
                            .map_err(|_| ProcessRunError::ControllerDropped)?
                    }
                }
                else {
                    // The controller was dropped, wich means we can't send the cancelation error, so we return it here
                    let (exit_status, child_killed_successfully) = Self::check_if_still_running_and_kill_and_wait(child).await?;
                    let new_status = Self::get_status_on_exit_status(exit_status, child_killed_successfully, true).await;
                    self.status_holder.overwrite(new_status).await;
                }
            }

            result_exit_status = child.wait() => {
                let exit_status = result_exit_status.map_err(ProcessRunError::CouldNotWaitForOsProcess)?;
                let new_status = Self::get_status_on_exit_status(exit_status, false, false).await;
                self.status_holder.overwrite(new_status).await;
            }
        }

        let status = self.status_holder.status().await;

        Ok(status)
    }

    fn spawn_os_process<I, S, P>(
        program: S,
        args: I,
        current_dir: P,
        stdout_sender: &Option<mpsc::Sender<String>>,
        stderr_sender: &Option<mpsc::Sender<String>>,
    ) -> Result<Child, IoError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        P: AsRef<Path>,
    {
        let stdout = Self::pipe_if_some_or_null(stdout_sender);
        let stderr = Self::pipe_if_some_or_null(stderr_sender);

        Command::new(program)
            .args(args)
            .current_dir(current_dir)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .kill_on_drop(true)
            .spawn()
    }

    fn pipe_if_some_or_null<T>(option: &Option<T>) -> Stdio {
        option
            .as_ref()
            .map(|_| Stdio::piped())
            .unwrap_or(Stdio::null())
    }

    async fn check_if_still_running_and_kill_and_wait(
        mut child: Child,
    ) -> Result<(ExitStatus, bool), ProcessKillAndWaitError> {
        let option_exit_status = child
            .try_wait()
            .map_err(ProcessKillAndWaitError::CouldNotCheckStatus)?;

        let (exit_status, killed) = match option_exit_status {
            Some(exit_status) => (exit_status, false),
            None => {
                child
                    .kill()
                    .await
                    .map_err(ProcessKillAndWaitError::CouldNotKillProcess)?;

                let exit_status = child
                    .wait()
                    .await
                    .map_err(ProcessKillAndWaitError::CouldNotWaitForProcess)?;

                (exit_status, true)
            }
        };

        Ok((exit_status, killed))
    }

    async fn get_status_on_exit_status(
        exit_status: ExitStatus,
        child_killed_successfuly: bool,
        controller_dropped: bool,
    ) -> Status {
        if exit_status.success() {
            return Status::Terminated(TerminationStatus::TerminatedSuccessfully);
        };

        match exit_status.code() {
            Some(code) => match code {
                1 if cfg!(target_os = "windows") && child_killed_successfuly => {
                    if controller_dropped {
                        return Status::Terminated(TerminationStatus::KilledByDroppingController);
                    }

                    Status::Terminated(TerminationStatus::Killed)
                }
                _ => Status::Terminated(TerminationStatus::TerminatedWithError(
                    TerminationWithErrorStatus::TerminatedWithErrorCode(code),
                )),
            },
            None if cfg!(target_os = "linux") && child_killed_successfuly => {
                if controller_dropped {
                    return Status::Terminated(TerminationStatus::KilledByDroppingController);
                }

                Status::Terminated(TerminationStatus::Killed)
            }
            _ => Status::Terminated(TerminationStatus::TerminatedWithError(
                TerminationWithErrorStatus::TerminatedWithUnknownErrorCode,
            )),
        }
    }

    fn forward_ios_to_channels(
        stdout: Option<ChildStdout>,
        stderr: Option<ChildStderr>,
        stdout_sender: Option<mpsc::Sender<String>>,
        stderr_sender: Option<mpsc::Sender<String>>,
        given_id: String,
        given_name: String,
    ) {
        if let Some(sender) = stdout_sender {
            if let Some(stdout) = stdout {
                Self::forward_io_to_channel(
                    stdout,
                    sender,
                    given_id.clone(),
                    given_name.clone(),
                    "stdout",
                );
            }
        }

        if let Some(sender) = stderr_sender {
            if let Some(stderr) = stderr {
                Self::forward_io_to_channel(stderr, sender, given_id, given_name, "stderr");
            }
        }
    }

    fn forward_io_to_channel<T: AsyncRead + Unpin + Send + 'static>(
        stdio: T,
        sender: mpsc::Sender<String>,
        given_id: String,
        given_name: String,
        io_name: &'static str,
    ) {
        let reader = io::BufReader::new(stdio);
        let mut lines = reader.lines();

        tokio::spawn(async move {
            tracing::debug!(given_id, io_name, given_name, "Starting to forward IO");
            while let Ok(Some(line)) = lines.next_line().await {
                if sender.send(line).await.is_err() {
                    break;
                }
            }
            tracing::debug!(given_id, io_name, given_name, "Finished forwarding IO");
        });
    }
}

#[derive(ThisError, Debug)]
pub enum ProcessRunError {
    #[error("Could not spawn os process: {0}")]
    CouldNotSpawnOsProcess(#[source] IoError),
    #[error("Could not wait for os process: {0}")]
    CouldNotWaitForOsProcess(#[source] IoError),
    #[error("Corresponding ProcessController was dropped after sending cancellation signal!. Should be infallible")]
    ControllerDropped,
    #[error("An error occured while killing and waiting for the process: {0}")]
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
    #[error("Cancellation signal can only be sent once")]
    AlreayTriedToCancel,
    #[error("Corresponding Process terminated already")]
    ProcessTerminated,
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
        stdout_sender: Option<mpsc::Sender<String>>,
        stderr_sender: Option<mpsc::Sender<String>>,
    ) -> OsProcessArgs<Vec<String>, String, String> {
        let path_str = path
            .to_str()
            .expect("Error converting path to string.")
            .to_owned();
        OsProcessArgs {
            program,
            args: vec![path_str],
            current_dir: ".".to_owned(),
            stdout_sender,
            stderr_sender,
        }
    }

    fn create_numbers_process() -> (Process, ProcessController) {
        Process::new("some_id".into(), "numbers_process".into())
    }

    fn create_number_process_run_args() -> OsProcessArgs<Vec<String>, String, String> {
        let path = get_numbers_script_path();
        create_process_args(program().to_owned(), path, None, None)
    }

    fn create_number_process_run_args_with_channels(
        stdout_sender: Option<mpsc::Sender<String>>,
        stderr_sender: Option<mpsc::Sender<String>>,
    ) -> OsProcessArgs<Vec<String>, String, String> {
        let path = get_numbers_script_path();
        create_process_args(program().to_owned(), path, stdout_sender, stderr_sender)
    }

    fn create_numbers_process_with_error_code() -> (Process, ProcessController) {
        Process::new("some_id".into(), "numbers_process_with_error_code".into())
    }

    fn create_number_process_with_error_code_run_args() -> OsProcessArgs<Vec<String>, String, String>
    {
        let path = get_numbers_script_with_error_code_path();
        create_process_args(program().to_owned(), path, None, None)
    }

    fn create_number_process_with_error_code_run_args_with_channels(
        stdout_sender: Option<mpsc::Sender<String>>,
        stderr_sender: Option<mpsc::Sender<String>>,
    ) -> OsProcessArgs<Vec<String>, String, String> {
        let path = get_numbers_script_with_error_code_path();
        create_process_args(program().to_owned(), path, stdout_sender, stderr_sender)
    }

    fn create_non_existing_process() -> (Process, ProcessController) {
        Process::new("some_id".into(), "non_existing_process".into())
    }

    fn create_non_existing_process_run_args() -> OsProcessArgs<Vec<String>, String, String> {
        let path = PathBuf::from("non_existing_process");
        create_process_args("non_existing_process".to_owned(), path, None, None)
    }

    fn assert_exit_with_error_code_1(result: Result<Status, ProcessRunError>) {
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

    fn assert_terminated_successfully(result: Result<Status, ProcessRunError>) {
        match result {
            Ok(Status::Terminated(TerminationStatus::TerminatedSuccessfully)) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn assert_killed(result: Result<Status, ProcessRunError>) {
        match result {
            Ok(Status::Terminated(TerminationStatus::Killed)) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn run_non_existing_process_and_expect_not_found() {
        let (process, _) = create_non_existing_process();
        let args = create_non_existing_process_run_args();

        let result = process.run(args).await;

        match result {
            Ok(_) => panic!("Process should not be created."),
            Err(error) => match error {
                ProcessRunError::CouldNotSpawnOsProcess(io_error) => match io_error.kind() {
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
        let (process, mut controller) = create_numbers_process();
        let args = create_number_process_run_args();

        let tast_handler = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            controller
                .cancel()
                .await
                .expect("Error cancelling process.");
        });

        let result = process.run(args).await;
        assert_killed(result);

        tast_handler.await.expect("Error waiting for handler.");
    }

    #[tokio::test]
    #[traced_test]
    async fn run_numbers_script_and_kill_before_start_and_expect_killed() {
        let (process, mut controller) = create_numbers_process();
        let args = create_number_process_run_args();

        let task_handler = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let result = process.run(args).await;

            assert_killed(result)
        });

        controller
            .cancel()
            .await
            .expect("Error cancelling process.");

        task_handler.await.expect("Error waiting for handler.");
    }

    #[tokio::test]
    #[traced_test]
    async fn run_numbers_script_and_kill_after_termination_and_expect_terminated_successfully_and_process_terminated(
    ) {
        let (process, mut controller) = create_numbers_process();
        let args = create_number_process_run_args();

        let task_handler = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let result = controller.cancel().await;
            match result {
                Err(CancellationError::ProcessTerminated) => {}
                _ => panic!("Unexpected result: {:?}", result),
            }
        });

        let result = process.run(args).await;
        assert_terminated_successfully(result);

        task_handler.await.expect("Error waiting for handler.");
    }

    #[tokio::test]
    #[traced_test]
    async fn run_numbers_script_with_error_code_and_expect_error_code_1() {
        let (process, _controller) = create_numbers_process_with_error_code();
        let args = create_number_process_with_error_code_run_args();

        let result = process.run(args).await;
        assert_exit_with_error_code_1(result);
    }

    #[tokio::test]
    #[traced_test]
    async fn cancel_a_dropped_process_and_expect_error() {
        let (process, mut controller) = create_numbers_process();

        drop(process);

        match controller.cancel().await {
            Err(CancellationError::ProcessTerminated) => {}
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn drop_controller_and_expect_killed_by_dropping_controller() {
        let (process, _) = create_numbers_process_with_error_code();
        let args = create_number_process_with_error_code_run_args();

        let result = process.run(args).await;

        match result {
            Ok(Status::Terminated(TerminationStatus::KilledByDroppingController)) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn cancel_a_process_twice_and_excpect_error() {
        let (process, mut controller) = create_numbers_process();
        let args = create_number_process_run_args();

        let task_handler = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            controller
                .cancel()
                .await
                .expect("Error cancelling process.");

            match controller.cancel().await {
                Err(CancellationError::AlreayTriedToCancel) => {}
                _ => panic!("Unexpected result"),
            }
        });

        process.run(args).await.expect("Error running process.");

        task_handler.await.expect("Error awaiting handler.");
    }

    #[tokio::test]
    #[traced_test]
    async fn pipe_stdout() {
        let (process, _controller) = create_numbers_process();
        let (stdout_sender, stdout_receiver) = mpsc::channel(10);

        let args = create_number_process_run_args_with_channels(Some(stdout_sender), None);

        let task_handler = tokio::spawn(async move {
            let mut lines: Vec<String> = Vec::new();
            let mut stdout = stdout_receiver;

            while let Some(line) = stdout.recv().await {
                lines.push(line);
            }

            let expected_lines: Vec<String> =
                vec!["1", "2", "3"].iter().map(|s| s.to_string()).collect();

            assert_eq!(lines, expected_lines);
        });

        let result = process.run(args).await;
        assert_terminated_successfully(result);

        task_handler.await.expect("Error awaiting handler.");
    }

    #[tokio::test]
    #[traced_test]

    async fn pipe_stderr() {
        let (process, _controller) = create_numbers_process_with_error_code();
        let (stderr_sender, stderr_receiver) = mpsc::channel(10);

        let args =
            create_number_process_with_error_code_run_args_with_channels(None, Some(stderr_sender));

        let task_handler = tokio::spawn(async move {
            let mut lines: Vec<String> = Vec::new();
            let mut stderr = stderr_receiver;

            while let Some(line) = stderr.recv().await {
                lines.push(line);
            }

            let expected_first_line = String::from("Error message");

            assert!(lines[0].contains(&expected_first_line));
        });

        let result = process.run(args).await;
        assert_exit_with_error_code_1(result);

        task_handler.await.expect("Error awaiting handler.");
    }
}
