use std::{process::Stdio, time::Duration};

use ptaas_rs::{
    models_2::print_dummies,
    project_managers::{process::Status, LocalProjectManager, Process},
};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
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

    let program = "python.exe";
    match Process::program_exists(
        Some("does python.exe exists".into()),
        program,
        Stdio::null(),
        Stdio::null(),
        Stdio::null(),
    )
    .await
    {
        Ok(exists) => {
            if exists {
                println!("{program} exists");
            } else {
                println!("{program} does not exist");
            }
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    }

    let mut p = Process::new(
        Some("numbers.ps1".into()),
        "powershell.exe",
        vec!["./numbers.ps1"],
        ".",
        Stdio::inherit(),
        Stdio::inherit(),
        Stdio::inherit(),
        true,
    )
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_secs(6)).await;
    println!("{:?}", p.kill_and_wait_and_set_status().await);
    println!("{:?}", p.status().unwrap());
    println!("{:?}", p.status().unwrap());

    // match p.wait_with_timeout_and_output(Duration::from_secs(6)).await {
    //     Ok(_) => {}
    //     Err(error) => {
    //         println!("Error: {}", error);
    //     }
    // }

    std::process::exit(0);
    // let stdout = p.stdout();

    // // Create a file to write the lines
    // let mut file = tokio::fs::File::create("output.txt").await.unwrap();

    // tokio::spawn(async move {
    //     if let Some(stdout) = stdout {
    //         let reader = io::BufReader::new(stdout);
    //         let mut lines = reader.lines();
    //         while let Ok(Some(line)) = lines.next_line().await {
    //             println!("{}", line);
    //             //file.write_all(line.as_bytes()).await.unwrap();
    //             //file.write_all(b"\n").await.unwrap();
    //         }
    //     }
    // });

    // match p.status().unwrap() {
    //     Status::Running => {
    //         println!("Process is still running");
    //         // p.kill_and_wait().await.unwrap();
    //     }
    //     _ => {
    //         println!("Process is not running");
    //     }
    // }

    print_dummies();

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
