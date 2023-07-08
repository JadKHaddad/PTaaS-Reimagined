use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct APIResponse {
    pub data: Option<APIResponseData>,
    pub error: Option<APIResponseError>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct APIResponseData {
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct APIResponseError {
    pub code: APIResponseErrorCode,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum APIResponseErrorCode {
    APITokenMissing,
    InvalidAPIToken,
}

// ------------------------------

#[derive(Serialize, Deserialize, Debug)]
pub struct AllLocustProjectsResponse {
    pub data: Option<AllLocustProjectsResponseData>,
    pub error: Option<AllLocustProjectsResponseError>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AllLocustProjectsResponseData {
    pub data: Vec<LocustProject>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AllLocustProjectsResponseError {
    pub code: AllLocustProjectsResponseErrorCode,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AllLocustProjectsResponseErrorCode {
    CouldNotFindLocustProjects,
    TimeOut,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LocustProject {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}
