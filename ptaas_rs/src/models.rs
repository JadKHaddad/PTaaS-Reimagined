use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocustProject {
    pub name: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocustScript {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralResponse {
    pub success: bool,
    pub data: Option<DataResponse>,
    pub error: Option<ErrorResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// We cannot use #[serde(untagged)] because serde-generate does not support it
pub enum DataResponse {
    LocustProjects(AllLocustProjects),
    LocustScripts(AllLocustScripts),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllLocustProjects {
    pub names: Vec<LocustProject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllLocustScripts {
    pub names: Vec<LocustScript>,
}
