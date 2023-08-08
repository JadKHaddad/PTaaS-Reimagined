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
use tracing::{debug_span, warn_span};

#[derive(Debug, Clone)]
pub enum Status {
    Created,
    Running,
    Terminated(TerminationStatus),
}

#[derive(Debug, Clone)]
pub enum TerminationStatus {
    Killed(KilledTerminationStatus),
    TerminatedSuccessfully,
    TerminatedWithError(TerminationWithErrorStatus),
}

#[derive(Debug, Clone)]
pub enum KilledTerminationStatus {
    /// Explicitly killed by this library.
    KilledByCancellationSignal,
    KilledByDroppingController,
}

#[derive(Debug, Clone)]
pub enum TerminationWithErrorStatus {
    /// On SIGTERM, the process will exit with UnknownErrorCode.
    /// On windows, the process will exit with 1. This will be translated to `Killed` if `child_killed_successfuly` is true.
    /// On linux, the process will exit with UnknownErrorCode. This will be translated to `Killed` if `child_killed_successfuly` is true.
    /// Otherwise, it will not be translated.
    TerminatedWithUnknownErrorCode,
    TerminatedWithErrorCode(i32),
}

/// Used in `Process::run` to pass arguments, to improve readability.
#[derive(Debug)]
pub struct OsProcessArgs<I, S, P> {
    pub program: S,
    pub args: I,
    pub current_dir: P,
    pub stdout_sender: Option<mpsc::Sender<String>>,
    pub stderr_sender: Option<mpsc::Sender<String>>,
}

/// Conveniently holding an `Arc<RwLock<Status>>` to hide **ugly** operations.
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
    given_id: String,
    /// Option so we can take it. Sends a cancellation signal to the process.
    cancel_channel_sender: Option<oneshot::Sender<()>>,
    /// Option so we can take it. Receives the cancellation result from the process.
    cancel_status_channel_receiver: Option<oneshot::Receiver<Option<ProcessKillAndWaitError>>>,
}

impl ProcessController {
    pub async fn cancel(&mut self) -> Result<Option<ProcessKillAndWaitError>, CancellationError> {
        let debug_span = debug_span!("ProcessController::cancel", given_id = self.given_id);
        let warn_span = warn_span!("ProcessController::cancel", given_id = self.given_id);

        let _debug_span_guard = debug_span.enter();
        let _warn_span_guard = warn_span.enter();

        match self.status_holder.status().await {
            Status::Created => {
                tracing::debug!("Process has not started yet");
                return Err(CancellationError::ProcessNotRunning);
            }
            Status::Terminated(_) => {
                tracing::debug!("Process is already terminated");
                return Err(CancellationError::ProcessTerminated);
            }
            Status::Running => {}
        }

        let cancel_channel_sender = self
            .cancel_channel_sender
            .take()
            .ok_or(CancellationError::AlreayTriedToCancel)?;

        let cancel_channel_receiver = self
            .cancel_status_channel_receiver
            .take()
            .ok_or(CancellationError::AlreayTriedToCancel)?;

        tracing::debug!("Sending cancellation signal to process");
        cancel_channel_sender.send(()).map_err(|_| {
            tracing::warn!("Failed to send cancellation signal to process");
            CancellationError::ProcessTerminated
        })?;

        tracing::debug!("Waiting for process to terminate");
        let cencel_result = cancel_channel_receiver.await.map_err(|_| {
            tracing::warn!("Failed to wait for process to terminate");
            CancellationError::ProcessTerminated
        })?;

        tracing::debug!("Process terminated");

        Ok(cencel_result)
    }

    pub async fn status(&self) -> Status {
        self.status_holder.status().await
    }
}

/// Wrapper around `tokio::process::Child` abstracting away the **ugly** details.
pub struct Process {
    status_holder: StatusHolder,
    given_id: String,
    given_name: String,
    child_killed_successfuly: bool,
    controller_dropped: bool,
    /// Option so we can take it. `None` if the process has not started yet.
    child: Option<Child>,
    /// Option so we can take it. `None` if the process has started. Receives the cancellation signal from the controller.
    cancel_status_channel_sender: Option<oneshot::Sender<Option<ProcessKillAndWaitError>>>,
    /// Option so we can take it. `None` if the process has started. Sends the cancellation result to the controller.
    cancel_channel_receiver: Option<oneshot::Receiver<()>>,
}

impl Drop for Process {
    fn drop(&mut self) {
        let child = self.child.take();

        let debug_span = debug_span!("Process::drop", given_id = self.given_id);

        {
            let _debug_span_guard = debug_span.enter();
            tracing::debug!("Dropping process");
        }

        let warn_span = warn_span!("Process::drop", given_id = self.given_id);

        if let Some(mut child) = child {
            if !self.child_killed_successfuly {
                tokio::spawn(async move {
                    let _debug_span_guard = debug_span.enter();
                    let _warn_span_guard = warn_span.enter();

                    tracing::warn!("Os process is being dropped without being killed first");

                    match child.kill().await {
                        Ok(_) => {
                            tracing::debug!("Killed os process");
                        }
                        Err(err) => {
                            tracing::warn!(%err, "Failed to kill os process");
                        }
                    }

                    match child.wait().await {
                        Ok(_) => {
                            tracing::debug!("Waited for os process to terminate");
                        }
                        Err(err) => {
                            tracing::warn!(%err, "Failed to wait for os process to terminate");
                        }
                    }

                    tracing::debug!("Dropping os process");
                });
            }
        }
    }
}

impl Process {
    #[must_use]
    pub fn new(given_id: String, given_name: String) -> (Self, ProcessController) {
        let status = Arc::new(RwLock::new(Status::Created));
        let status_holder = StatusHolder { status };

        let (cancel_status_channel_sender, cancel_status_channel_receiver) = oneshot::channel();
        let (cancel_channel_sender, cancel_channel_receiver) = oneshot::channel();

        let process = Self {
            status_holder: status_holder.clone(),
            given_id: given_id.clone(),
            given_name,
            child_killed_successfuly: false,
            controller_dropped: false,
            child: None,
            cancel_status_channel_sender: Some(cancel_status_channel_sender),
            cancel_channel_receiver: Some(cancel_channel_receiver),
        };

        let process_controller = ProcessController {
            status_holder,
            given_id,
            cancel_channel_sender: Some(cancel_channel_sender),
            cancel_status_channel_receiver: Some(cancel_status_channel_receiver),
        };

        (process, process_controller)
    }

    pub async fn run<I, S, P>(
        &mut self,
        os_process_args: OsProcessArgs<I, S, P>,
    ) -> Result<Status, ProcessRunError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        P: AsRef<Path>,
    {
        let debug_span = debug_span!(
            "Process::run",
            given_id = self.given_id,
            given_name = self.given_name
        );
        let _span_guard = debug_span.enter();

        let cancel_channel_sender = self
            .cancel_status_channel_sender
            .take()
            .ok_or(ProcessRunError::AlreayTriedToRun)?;

        let cancel_channel_receiver = self
            .cancel_channel_receiver
            .take()
            .ok_or(ProcessRunError::AlreayTriedToRun)?;

        self.spawn_os_process_and_forward_ios_to_channels(os_process_args)
            .await
            .map_err(ProcessRunError::CouldNotSpawnOsProcess)?;

        self.wait_for_signal_or_termination(cancel_channel_receiver, cancel_channel_sender)
            .await?;

        let status = self.status_holder.status().await;

        Ok(status)
    }

    async fn wait_for_signal_or_termination(
        &mut self,
        cancel_channel_receiver: oneshot::Receiver<()>,
        cancel_channel_sender: oneshot::Sender<Option<ProcessKillAndWaitError>>,
    ) -> Result<(), ProcessRunError> {
        let child = self
            .child
            .as_mut()
            .ok_or(ProcessRunError::OOPS(ChildNotSet {}))?;

        tracing::debug!("Waiting for termination or cancellation signal");
        tokio::select! {
            result = cancel_channel_receiver => {
                if result.is_ok() {
                    tracing::debug!(
                        "Os process was cancelled by the controller"
                    );

                    // The process was explicitly cancelled by the controller
                    // Cancellation errors are sent to the controller and this function returns
                    match self.check_if_still_running_and_kill_and_wait().await {
                        Ok(exit_status) => {
                            self.set_status_on_exit_status(exit_status).await;

                            cancel_channel_sender
                                .send(None).map_err(|_| ProcessRunError::ControllerDropped)?;
                        }
                        Err(e) => cancel_channel_sender.send(Some(e))
                            .map_err(|_| ProcessRunError::ControllerDropped)?
                    }
                }
                else {
                    self.controller_dropped = true;
                    tracing::debug!(
                        "Os process was cancelled by dropping the controller"
                    );

                    // The controller was dropped, wich means we can't send the cancelation error, so we return it here
                    let exit_status = self.check_if_still_running_and_kill_and_wait().await?;
                    self.set_status_on_exit_status(exit_status).await;
                }
            }

            result_exit_status = child.wait() => {
                tracing::debug!(
                    "Os process terminated by itself"
                );

                let exit_status = result_exit_status.map_err(ProcessRunError::CouldNotWaitForOsProcess)?;
                self.set_status_on_exit_status(exit_status).await;
            }
        }

        Ok(())
    }

    async fn spawn_os_process_and_forward_ios_to_channels<I, S, P>(
        &mut self,
        os_process_args: OsProcessArgs<I, S, P>,
    ) -> Result<(), IoError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        P: AsRef<Path>,
    {
        let OsProcessArgs {
            program,
            args,
            current_dir,
            stdout_sender,
            stderr_sender,
        } = os_process_args;

        let stdout = Self::pipe_if_some_else_null(&stdout_sender);
        let stderr = Self::pipe_if_some_else_null(&stderr_sender);

        let mut child = Command::new(program)
            .args(args)
            .current_dir(current_dir)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .kill_on_drop(true)
            .spawn()?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        Self::forward_ios_to_channels(
            stdout,
            stderr,
            stdout_sender,
            stderr_sender,
            self.given_id.clone(),
            self.given_name.clone(),
        );

        self.status_holder.overwrite(Status::Running).await;

        self.child = Some(child);

        Ok(())
    }

    async fn check_if_still_running_and_kill_and_wait(
        &mut self,
    ) -> Result<ExitStatus, ProcessKillAndWaitError> {
        let child = self
            .child
            .as_mut()
            .ok_or(ProcessKillAndWaitError::OOPS(ChildNotSet {}))?;

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

    async fn get_termination_status_on_exit_status(
        &self,
        exit_status: ExitStatus,
    ) -> TerminationStatus {
        if exit_status.success() {
            return TerminationStatus::TerminatedSuccessfully;
        };

        match exit_status.code() {
            Some(code) => match code {
                1 if cfg!(target_os = "windows") && self.child_killed_successfuly => {
                    if self.controller_dropped {
                        return TerminationStatus::Killed(
                            KilledTerminationStatus::KilledByDroppingController,
                        );
                    }

                    TerminationStatus::Killed(KilledTerminationStatus::KilledByCancellationSignal)
                }
                _ => TerminationStatus::TerminatedWithError(
                    TerminationWithErrorStatus::TerminatedWithErrorCode(code),
                ),
            },
            None if cfg!(target_os = "linux") && self.child_killed_successfuly => {
                if self.controller_dropped {
                    return TerminationStatus::Killed(
                        KilledTerminationStatus::KilledByDroppingController,
                    );
                }

                TerminationStatus::Killed(KilledTerminationStatus::KilledByCancellationSignal)
            }
            _ => TerminationStatus::TerminatedWithError(
                TerminationWithErrorStatus::TerminatedWithUnknownErrorCode,
            ),
        }
    }

    async fn set_status_on_exit_status(&self, exit_status: ExitStatus) {
        let termination_status = self
            .get_termination_status_on_exit_status(exit_status)
            .await;
        let new_status = Status::Terminated(termination_status);
        self.status_holder.overwrite(new_status).await;
    }

    fn pipe_if_some_else_null<T>(option: &Option<T>) -> Stdio {
        option
            .as_ref()
            .map(|_| Stdio::piped())
            .unwrap_or(Stdio::null())
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
                Self::forward_io(
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
                Self::forward_io(stderr, sender, given_id, given_name, "stderr");
            }
        }
    }

    fn forward_io<T: AsyncRead + Unpin + Send + 'static>(
        stdio: T,
        sender: mpsc::Sender<String>,
        given_id: String,
        given_name: String,
        io_name: &'static str,
    ) {
        let reader = io::BufReader::new(stdio);
        let mut lines = reader.lines();

        tokio::spawn(async move {
            let debug_span = tracing::debug_span!(
                "Process::Forwarding_IO",
                given_id = given_id,
                io_name = io_name,
                given_name = given_name
            );
            {
                let _span_guard = debug_span.enter();
                tracing::debug!("Starting to forward IO");
            }

            while let Ok(Some(line)) = lines.next_line().await {
                if sender.send(line).await.is_err() {
                    break;
                }
            }

            let _span_guard = debug_span.enter();
            tracing::debug!("Finished forwarding IO");
        });
    }

    pub async fn status(&self) -> Status {
        self.status_holder.status().await
    }
}

/// Getting a `ChildNotSet` error, which is extremely weird, requires you to drop the process in order to kill and wait for the child.
/// Long story short: this is a bug in the code. investigate it.
#[derive(ThisError, Debug)]
#[error("Some one should have set the child :D")]
pub struct ChildNotSet {}

#[derive(ThisError, Debug)]
pub enum ProcessRunError {
    #[error("Process was already run!")]
    AlreayTriedToRun,
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
    #[error("OOPS: {0}")]
    OOPS(ChildNotSet),
}

#[derive(ThisError, Debug)]
pub enum ProcessKillAndWaitError {
    #[error("Could not check status of process: {0}")]
    CouldNotCheckStatus(#[source] IoError),
    #[error("Could not kill process: {0}")]
    CouldNotKillProcess(#[source] IoError),
    #[error("Could not wait for process: {0}")]
    CouldNotWaitForProcess(#[source] IoError),
    #[error("OOPS: {0}")]
    OOPS(ChildNotSet),
}

#[derive(ThisError, Debug)]
pub enum CancellationError {
    #[error("Process is not running")]
    ProcessNotRunning,
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

    fn get_non_stop_numbers_script_path() -> PathBuf {
        if cfg!(target_os = "linux") {
            return get_tests_dir().join("non_stop_numbers.sh");
        } else if cfg!(target_os = "windows") {
            return get_tests_dir().join("non_stop_numbers.ps1");
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

    fn create_non_stop_number_process_run_args_with_channels(
        stdout_sender: Option<mpsc::Sender<String>>,
        stderr_sender: Option<mpsc::Sender<String>>,
    ) -> OsProcessArgs<Vec<String>, String, String> {
        let path = get_non_stop_numbers_script_path();
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
            Ok(Status::Terminated(TerminationStatus::Killed(
                KilledTerminationStatus::KilledByCancellationSignal,
            ))) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
            _ => panic!("Unexpected result: {:?}", result),
        }
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
        let (mut process, mut controller) = create_numbers_process();
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
    async fn run_numbers_script_and_kill_after_termination_and_expect_terminated_successfully_and_process_terminated(
    ) {
        let (mut process, mut controller) = create_numbers_process();
        let args = create_number_process_run_args();

        let task_handler = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            match controller.cancel().await {
                Err(CancellationError::ProcessTerminated) => {}
                result => panic!("Unexpected result: {:?}", result),
            }
        });

        let result = process.run(args).await;
        assert_terminated_successfully(result);

        task_handler.await.expect("Error waiting for handler.");
    }

    #[tokio::test]
    #[traced_test]
    async fn run_numbers_script_with_error_code_and_expect_error_code_1() {
        let (mut process, _controller) = create_numbers_process_with_error_code();
        let args = create_number_process_with_error_code_run_args();

        let result = process.run(args).await;
        assert_exit_with_error_code_1(result);
    }

    #[tokio::test]
    #[traced_test]
    async fn drop_controller_and_expect_killed_by_dropping_controller() {
        let (mut process, _) = create_numbers_process_with_error_code();
        let args = create_number_process_with_error_code_run_args();

        let result = process.run(args).await;

        match result {
            Ok(Status::Terminated(TerminationStatus::Killed(
                KilledTerminationStatus::KilledByDroppingController,
            ))) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn cancel_process_before_start_and_expect_process_not_running_error() {
        let (_process, mut controller) = create_numbers_process();

        match controller.cancel().await {
            Err(CancellationError::ProcessNotRunning) => {}
            result => panic!("Unexpected result {:?}", result),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn cancel_process_after_termination_and_expect_process_terminated_error() {
        let (mut process, mut controller) = create_numbers_process();
        let args = create_number_process_run_args();

        process.run(args).await.expect("Error running process.");

        match controller.cancel().await {
            Err(CancellationError::ProcessTerminated) => {}
            result => panic!("Unexpected result {:?}", result),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn cancel_a_dropped_process_before_running_and_expect_process_not_running_error() {
        let (process, mut controller) = create_numbers_process();

        drop(process);

        match controller.cancel().await {
            Err(CancellationError::ProcessNotRunning) => {}
            result => panic!("Unexpected result {:?}", result),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn cancel_a_dropped_process_after_running_and_expect_process_terminated_error() {
        let (mut process, mut controller) = create_numbers_process();
        let args = create_number_process_run_args();

        process.run(args).await.expect("Error running process.");

        drop(process);

        match controller.cancel().await {
            Err(CancellationError::ProcessTerminated) => {}
            result => panic!("Unexpected result {:?}", result),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn cancel_a_process_twice_and_expect_process_terminated_error() {
        let (mut process, mut controller) = create_numbers_process();
        let args = create_number_process_run_args();

        let task_handler = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            controller
                .cancel()
                .await
                .expect("Error cancelling process.");

            match controller.cancel().await {
                Err(CancellationError::ProcessTerminated) => {}
                result => panic!("Unexpected result {:?}", result),
            }
        });

        process.run(args).await.expect("Error running process.");

        task_handler.await.expect("Error awaiting handler.");
    }

    #[tokio::test]
    #[traced_test]
    async fn pipe_stdout() {
        let (mut process, _controller) = create_numbers_process();
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
        let (mut process, _controller) = create_numbers_process_with_error_code();
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

    #[tokio::test]
    #[traced_test]
    #[ignore]
    async fn observe_process_drop_for_humans() {
        let (mut process, controller) = create_numbers_process();
        let (stdout_sender, stdout_receiver) = mpsc::channel(10);

        let args = create_non_stop_number_process_run_args_with_channels(Some(stdout_sender), None);

        let task_handler = tokio::spawn(async move {
            let mut stdout = stdout_receiver;

            while let Some(line) = stdout.recv().await {
                println!("Received line: {}", line);
            }
        });

        // ensuring a drop after a select!
        tokio::spawn(async move {
            let _move_controller = controller;
            tokio::select! {
                _ = process.run(args) => {

                }
                _ = tokio::time::sleep(Duration::from_secs(2)) => {
                    println!("Process timed out.");
                }
            }
        });

        task_handler.await.expect("Error awaiting handler.");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
