use ptaas_rs::{
    models::{
        APIGerneralResponseErrorType, APIResponse, APIResponseError, APIResponseType,
        AllProjectsResponseData, AllProjectsResponseErrorType, Project, Script,
    },
    project_managers::LocalProjectManager,
};
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

fn create_dummy() {
    // Create fake data for an API response indicating success
    let projects = vec![
        Project {
            id: String::from("project1"),
            installed: true,
            scripts: vec![
                Script {
                    id: String::from("script1"),
                },
                Script {
                    id: String::from("script2"),
                },
            ],
        },
        Project {
            id: String::from("project2"),
            installed: false,
            scripts: vec![Script {
                id: String::from("script3"),
            }],
        },
    ];

    let all_projects_data = AllProjectsResponseData { projects };

    let api_response: APIResponse<_, ()> = APIResponse {
        success: true,
        response_type: APIResponseType::AllProjectsResponse,
        data: Some(all_projects_data),
        error: None,
    };

    println!(
        "API Response:\n{}\n",
        serde_json::to_string(&api_response).unwrap()
    );

    // Create fake data for an API response indicating an error
    let error_type = AllProjectsResponseErrorType::CantReadProjects;
    let error_message = String::from("Failed to read projects.");

    let error = APIResponseError {
        error_type,
        error_message,
    };

    let api_error_response: APIResponse<(), _> = APIResponse {
        success: false,
        response_type: APIResponseType::AllProjectsResponse,
        data: None,
        error: Some(error),
    };

    println!(
        "API Error Response:\n{}\n",
        serde_json::to_string(&api_error_response).unwrap()
    );

    // Create an unsuccessful APIGeneralResponse with an error
    let error_type = APIGerneralResponseErrorType::APIKeyIsMissing;
    let error_message = String::from("API key is missing.");

    let error = APIResponseError {
        error_type,
        error_message,
    };

    let api_error_response: APIResponse<(), _> = APIResponse {
        success: false,
        response_type: APIResponseType::GerneralResponse,
        data: None,
        error: Some(error),
    };

    println!(
        "API General Error Response: \n{}",
        serde_json::to_string(&api_error_response).unwrap()
    );
}
