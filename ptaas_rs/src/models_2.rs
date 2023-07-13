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
pub enum APIResponse {
    Processed(APIResponseProcessd),
    Failed(APIResponseFailed),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum APIResponseProcessd {
    AllProjects(AllProjectsResponse),
    AllScripts(AllScriptsResponse),
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

pub fn print_dummies() {
    let api_failed = APIResponse::Failed(APIResponseFailed::MissingToken);

    let all_proj = APIResponse::Processed(APIResponseProcessd::AllProjects(
        AllProjectsResponse::Processed(AllProjectsResponseProcessed {
            projects: vec![Project {
                id: "id".to_string(),
                installed: true,
                scripts: vec![Script {
                    id: "id".to_string(),
                }],
            }],
        }),
    ));

    let all_proj_failed = APIResponse::Processed(APIResponseProcessd::AllProjects(
        AllProjectsResponse::Failed(AllProjectsResponseFailed::AProjectIsMissing),
    ));

    let _all_scripts = APIResponse::Processed(APIResponseProcessd::AllScripts(
        AllScriptsResponse::Processed(AllScriptsResponseProcessed {
            scripts: vec![Script {
                id: "id".to_string(),
            }],
        }),
    ));

    // print them with serde_json

    println!(
        "api_failed:\n{}\n",
        serde_json::to_string(&api_failed).unwrap()
    );
    println!("all_proj:\n{}\n", serde_json::to_string(&all_proj).unwrap());
    println!(
        "all_proj_failed:\n{}\n",
        serde_json::to_string(&all_proj_failed).unwrap()
    );
}
