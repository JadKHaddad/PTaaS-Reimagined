use serde::{Deserialize, Serialize};

// Models

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

// Responses

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum APIResponse<T> {
    Processed(T),
    Failed(APIResponseFailed),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum APIResponseFailed {
    MissingToken,
    EmptyToken,
    NotLoggedIn,
    InternalServerError,
}

// Projects

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum AllProjectsResponse {
    Processed(AllProjectsResponseProcessed),
    Failed(AllProjectsResponseFailed),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AllProjectsResponseProcessed {
    pub projects: Vec<Project>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum AllProjectsResponseFailed {
    CantReadProjects,
    AProjectIsMissing,
}

// Scripts

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum AllScriptsResponse {
    Processed(AllScriptsResponseProcessed),
    Failed(AllScriptsResponseFailed),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AllScriptsResponseProcessed {
    pub scripts: Vec<Script>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum AllScriptsResponseFailed {
    CantReadScripts,
    AScriptIsMissing,
}

pub fn print_dummies() {}
