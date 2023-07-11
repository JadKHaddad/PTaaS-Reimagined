use serde::{Deserialize, Serialize};

/// APIResponse is a generic struct that can be used to return data or an error
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct APIResponse<D, E> {
    pub success: bool,
    pub response_type: APIResponseType,
    /// If data is not None, then the request was successful
    pub data: Option<D>,
    /// If error is not None, then the request was unsuccessful
    pub error: Option<APIResponseError<E>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct APIResponseError<E> {
    /// Every endpoint should have its own error types
    pub error_type: E,
    pub error_message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum APIResponseType {
    /// GereralResponse is a generic response that indicates a failure before processing the request
    GerneralResponse,
    AllProjectsResponse,
    AllScriptsResponse,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum APIGerneralResponseErrorType {
    APIKeyIsMissing,
    APIKeyIsInvalid,
}

// -----------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub id: String,
    pub installed: bool,
    pub scripts: Vec<Script>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Script {
    pub id: String,
}

// -----------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AllProjectsResponseData {
    pub projects: Vec<Project>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AllProjectsResponseErrorType {
    CantReadProjects,
    AProjectIsMissing,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AllScriptsResponseData {
    pub scripts: Vec<Script>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AllScriptsResponseErrorType {
    CantReadScripts,
    AScriptIsMissing,
    CorrespondingProjectIsMissing,
}
