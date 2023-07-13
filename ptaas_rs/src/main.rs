use std::{process::Stdio, time::Duration};

use ptaas_rs::{
    models_2::print_dummies,
    project_managers::{LocalProjectManager, Process},
};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let mut p = Process::new(
        "powershell.exe",
        vec!["./numbers.ps1"],
        ".",
        Stdio::inherit(),
        Stdio::piped(),
        Stdio::inherit(),
    )
    .await
    .unwrap();

    let stdout = p.stdout();

    // Create a file to write the lines
    let mut file = tokio::fs::File::create("output.txt").await.unwrap();

    tokio::spawn(async move {
        if let Some(stdout) = stdout {
            let reader = io::BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                file.write_all(line.as_bytes()).await.unwrap();
                file.write_all(b"\n").await.unwrap();
            }
        }
    });

    tokio::time::sleep(Duration::from_secs(5)).await;
    p.status().await.unwrap();
    p.kill_and_wait().await.unwrap();

    print_dummies();
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "ptaas_rs=trace,tower_http=off,hyper=off");
    }

    tracing_subscriber::fmt()
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
        .with_level(true)
        .with_ansi(true)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let basic_auth_username = std::env::var("BASIC_AUTH_USERNAME").unwrap_or_else(|_| {
        tracing::warn!("BASIC_AUTH_USERNAME not set, using default value");
        String::from("admin")
    });
    let basic_auth_password = std::env::var("BASIC_AUTH_PASSWORD").unwrap_or_else(|_| {
        tracing::warn!("BASIC_AUTH_PASSWORD not set, using default value");
        String::from("admin")
    });

    let root_dir = "./projects";
    let manager = match LocalProjectManager::new(root_dir.into()).await {
        Ok(manager) => manager,
        Err(error) => {
            tracing::error!(%error, "Failed to create LocalProjectManager");
            std::process::exit(1);
        }
    };
}
