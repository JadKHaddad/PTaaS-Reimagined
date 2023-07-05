use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocustProject {
    pub name: String,
    pub installed: bool,
}
