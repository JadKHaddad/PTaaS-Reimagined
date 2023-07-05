use std::path::PathBuf;

use serde_generate::{Encoding, SourceInstaller};
use serde_reflection::{Tracer, TracerConfig};

use crate::models::{
    AllLocustProjects, AllLocustScripts, DataResponse, ErrorResponse, GeneralResponse,
    LocustProject, LocustScript,
};

pub fn export_models_to_dart(install_dir: PathBuf) {
    let mut tracer = Tracer::new(TracerConfig::default());

    if let Err(err) = tracer.trace_simple_type::<GeneralResponse>() {
        eprintln!("Failed to trace: {}", err);
        eprintln!("{}", err.explanation());
        return;
    }

    if let Err(err) = tracer.trace_simple_type::<DataResponse>() {
        eprintln!("Failed to trace: {}", err);
        eprintln!("{}", err.explanation());
        return;
    }

    // if let Err(err) = tracer.trace_simple_type::<AllLocustProjects>() {
    //     eprintln!("Failed to trace: {}", err);
    //     eprintln!("{}", err.explanation());
    //     return;
    // }

    // if let Err(err) = tracer.trace_simple_type::<AllLocustScripts>() {
    //     eprintln!("Failed to trace: {}", err);
    //     eprintln!("{}", err.explanation());
    //     return;
    // }

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
    // Create a dummy LocustProject
    let locust_project = LocustProject {
        name: "Project 1".to_string(),
        installed: true,
    };

    // Create a dummy LocustScript
    let locust_script = LocustScript {
        name: "Script 1".to_string(),
        content: "Script content".to_string(),
    };

    // Create a dummy AllLocustProjects
    let all_locust_projects = AllLocustProjects {
        names: vec![
            LocustProject {
                name: "Project 1".to_string(),
                installed: true,
            },
            LocustProject {
                name: "Project 2".to_string(),
                installed: false,
            },
        ],
    };

    // Create a dummy AllLocustScripts
    let all_locust_scripts = AllLocustScripts {
        names: vec![
            LocustScript {
                name: "Script 1".to_string(),
                content: "Script content 1".to_string(),
            },
            LocustScript {
                name: "Script 2".to_string(),
                content: "Script content 2".to_string(),
            },
        ],
    };

    // Create a dummy DataResponse containing AllLocustProjects
    let data_response_projects = DataResponse::LocustProjects(all_locust_projects.clone());

    // Create a dummy DataResponse containing AllLocustScripts
    let data_response_scripts = DataResponse::LocustScripts(all_locust_scripts.clone());

    // Create a dummy GeneralResponse with success and AllLocustProjects
    let general_response_projects = GeneralResponse {
        success: true,
        data: Some(data_response_projects.clone()),
        error: None,
    };

    // Create a dummy GeneralResponse with failure and an error response
    let error_response = ErrorResponse {
        code: "500".to_string(),
        message: "Internal Server Error".to_string(),
    };

    let general_response_error = GeneralResponse {
        success: false,
        data: None,
        error: Some(error_response),
    };

    // Print the dummy objects as json strings
    println!(
        "general_response_projects: {}",
        serde_json::to_string(&general_response_projects).unwrap()
    );
    println!(
        "general_response_error: {}",
        serde_json::to_string(&general_response_error).unwrap()
    );
}
