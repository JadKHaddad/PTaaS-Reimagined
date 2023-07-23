use ptaas_rs::{
    init_tracing,
    project_managers::{process::dev::run_all as run_process_dev_examples, LocalProjectManager},
};

#[tokio::main]
async fn main() {
    init_tracing();

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

    run_process_dev_examples().await;
}
