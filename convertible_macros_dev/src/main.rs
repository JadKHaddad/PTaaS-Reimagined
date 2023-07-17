use convertible::macros::DartConvertible;
use std::collections::HashMap;
//#[derive(DartConvertible)]
pub struct Project {
    pub id: String,
    pub installed: bool,
    pub scripts: Vec<Script>,
    pub optional_id: Option<String>,
    pub optional_scripts: Option<Vec<Script>>,
}

pub struct Script {
    pub id: String,
}

fn main() {}
