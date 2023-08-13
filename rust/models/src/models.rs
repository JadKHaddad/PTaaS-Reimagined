use serde::{Deserialize, Serialize};

/// APIResponse is a generic struct that can be used to return data or an error
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct APIResponse<D, E> {
    pub success: bool,
    pub response_type: APIResponseType,
    /// If data is not None, then the request was successful
    pub data: Option<D>,
    /// If error is not None, then the request was unsuccessful
    pub error: Option<APIResponseError<E>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct APIResponseError<E> {
    /// Every endpoint should have its own error types
    pub error_type: E,
    pub error_message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum APIResponseType {
    /// GereralResponse is a generic response that indicates a failure before processing the request
    GerneralResponse,
    AllProjectsResponse,
    AllScriptsResponse,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum APIGerneralResponseErrorType {
    APIKeyIsMissing,
    APIKeyIsInvalid,
}

// -----------------

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub installed: bool,
    pub scripts: Vec<Script>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Script {
    pub id: String,
}

// -----------------

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AllProjectsResponseData {
    pub projects: Vec<Project>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum AllProjectsResponseErrorType {
    CantReadProjects,
    AProjectIsMissing,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AllScriptsResponseData {
    pub scripts: Vec<Script>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum AllScriptsResponseErrorType {
    CantReadScripts,
    AScriptIsMissing,
    CorrespondingProjectIsMissing,
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
