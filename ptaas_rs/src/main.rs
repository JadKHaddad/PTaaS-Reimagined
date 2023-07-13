use ptaas_rs::{models_2::print_dummies, project_managers::LocalProjectManager};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
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
