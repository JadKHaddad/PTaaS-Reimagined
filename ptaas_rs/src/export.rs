use std::path::PathBuf;

use serde_generate::{Encoding, SourceInstaller};
use serde_reflection::{Tracer, TracerConfig};

use crate::models::{
    APIResponse, APIResponseData, APIResponseError, APIResponseErrorCode,
    AllLocustProjectsResponse, AllLocustProjectsResponseData, AllLocustProjectsResponseError,
    AllLocustProjectsResponseErrorCode, LocustProject,
};

pub fn export_models_to_dart(install_dir: PathBuf) {
    let mut tracer = Tracer::new(TracerConfig::default());

    if let Err(err) = tracer.trace_simple_type::<APIResponse>() {
        eprintln!("Failed to trace: {}", err);
        eprintln!("{}", err.explanation());
        return;
    }

    if let Err(err) = tracer.trace_simple_type::<APIResponseErrorCode>() {
        eprintln!("Failed to trace: {}", err);
        eprintln!("{}", err.explanation());
        return;
    }

    if let Err(err) = tracer.trace_simple_type::<AllLocustProjectsResponse>() {
        eprintln!("Failed to trace: {}", err);
        eprintln!("{}", err.explanation());
        return;
    }

    if let Err(err) = tracer.trace_simple_type::<AllLocustProjectsResponseErrorCode>() {
        eprintln!("Failed to trace: {}", err);
        eprintln!("{}", err.explanation());
        return;
    }

    let registry = match tracer.registry() {
        Ok(registry) => registry,
        Err(err) => {
            eprintln!("Failed to trace: {}", err);
            eprintln!("{}", err.explanation());
            return;
        }
    };

    let config = serde_generate::CodeGeneratorConfig::new("models".to_string())
        .with_encodings(vec![Encoding::Bincode, Encoding::Bcs])
        .with_serialization(true);

    let dart_generator = serde_generate::dart::CodeGenerator::new(&config);
    std::fs::create_dir_all(&install_dir).expect("Failed to create dart folder");
    dart_generator
        .output(install_dir.clone(), &registry)
        .expect("Failed to generate dart code");

    let dart_installer = serde_generate::dart::Installer::new(install_dir);
    dart_installer
        .install_module(&config, &registry)
        .expect("Failed to install dart code");
    dart_installer
        .install_serde_runtime()
        .expect("Failed to install dart serde runtime");
    dart_installer
        .install_bincode_runtime()
        .expect("Failed to install dart bincode runtime");
    dart_installer
        .install_bcs_runtime()
        .expect("Failed to install dart bcs runtime");
}

pub fn dummy() {
    // Create an APIResponse with data
    let api_response_data = APIResponseData {
        data: "Sample data".to_string(),
    };
    let api_response = APIResponse {
        data: Some(api_response_data),
        error: None,
    };

    println!(
        "APIResponse:\n{}",
        serde_json::to_string(&api_response).unwrap()
    );

    // Create an APIResponse with an error
    let api_error = APIResponseError {
        code: APIResponseErrorCode::InvalidAPIToken,
        message: "Invalid API token".to_string(),
    };
    let api_response_with_error = APIResponse {
        data: None,
        error: Some(api_error),
    };

    println!(
        "\n\nAPIResponse with error:\n{}",
        serde_json::to_string(&api_response_with_error).unwrap()
    );

    // Create an AllLocustProjectsResponse with data
    let locust_project1 = LocustProject {
        id: "1".to_string(),
        name: "Project 1".to_string(),
        description: "Sample project 1".to_string(),
        created_at: "2023-07-01".to_string(),
        updated_at: "2023-07-03".to_string(),
    };

    let locust_project2 = LocustProject {
        id: "2".to_string(),
        name: "Project 2".to_string(),
        description: "Sample project 2".to_string(),
        created_at: "2023-07-02".to_string(),
        updated_at: "2023-07-04".to_string(),
    };

    let all_locust_projects_data = AllLocustProjectsResponseData {
        data: vec![locust_project1, locust_project2],
    };

    let all_locust_projects_response = AllLocustProjectsResponse {
        data: Some(all_locust_projects_data),
        error: None,
    };

    println!(
        "\n\nAllLocustProjectsResponse:\n{}",
        serde_json::to_string(&all_locust_projects_response).unwrap()
    );

    // Create an AllLocustProjectsResponse with an error
    let all_locust_projects_error = AllLocustProjectsResponseError {
        code: AllLocustProjectsResponseErrorCode::CouldNotFindLocustProjects,
        message: "No locust projects found".to_string(),
    };

    let all_locust_projects_response_with_error = AllLocustProjectsResponse {
        data: None,
        error: Some(all_locust_projects_error),
    };

    println!(
        "\n\nAllLocustProjectsResponse with error:\n{}",
        serde_json::to_string(&all_locust_projects_response_with_error).unwrap()
    );
}
