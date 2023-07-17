use convertible::{
    definitions::DartConvertible, macros::DartConvertible as DartConvertibleDeriveMacro,
};

#[derive(DartConvertibleDeriveMacro)]
pub struct Project {
    pub id: String,
    pub installed: bool,
    pub scripts: Vec<Script>,
    pub optional_id: Option<String>,
}

#[derive(DartConvertibleDeriveMacro)]
pub struct Script {
    pub id: String,
}

fn main() {
    println!("{}", Project::to_dart());
    println!("{}", Script::to_dart());
}
