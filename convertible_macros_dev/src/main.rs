use convertible::{definitions::dart::DartFactory, macros::DartConvertible};

#[derive(DartConvertible)]
pub struct Project {
    pub id: String,
    pub installed: bool,
    pub scripts: Vec<Script>,
    pub optional_id: Option<Vec<String>>,
}

#[derive(DartConvertible)]
pub struct Script {
    pub id: String,
}

fn main() {
    let dart_code = DartFactory::new("models")
        .add::<Project>()
        .add::<Script>()
        .build();

    println!("{}", dart_code);
}
