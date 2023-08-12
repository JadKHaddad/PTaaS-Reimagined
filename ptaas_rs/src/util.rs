use std::{io::Error as IoError, path::Path, time::Duration};
use thiserror::Error as ThisError;
use tokio::fs;

#[derive(ThisError, Debug)]
#[error("Max attempts exceeded")]
pub struct MaxAttemptsExceeded(Vec<IoError>);

pub async fn remove_dir_all_with_max_attempts_and_delay(
    max_attempts: u16,
    delay: Duration,
    path: &Path,
) -> Result<Vec<IoError>, MaxAttemptsExceeded> {
    let mut errors = Vec::new();

    for _ in 0..max_attempts {
        tracing::debug!(?path, "Attempting to delete dir");
        match fs::remove_dir_all(path).await {
            Ok(_) => return Ok(errors),
            Err(err) => {
                tracing::error!(%err, ?path, "Failed to delete dir");
                errors.push(err);
                tokio::time::sleep(delay).await;
            }
        }
    }

    Err(MaxAttemptsExceeded(errors))
}
