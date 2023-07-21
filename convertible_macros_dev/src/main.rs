use convertible::definitions::dart::DartConvertible as Trait;
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

#[derive(DartConvertible)]
pub enum MyEnum {
    WakaA,
    BcbData,
}

#[derive(DartConvertible)]
pub enum MyEnum2 {
    A(Script),
    B(Project),
}

#[derive(DartConvertible)]
pub enum MyEnum3 {
    A(Script),
    B(Script),
}

fn main() {
    let dart_code = DartFactory::new("models")
        .add::<Project>()
        .add::<Script>()
        .add::<MyEnum>()
        .add::<MyEnum2>()
        .add::<MyEnum3>()
        .build();

    println!("{}", dart_code);

    //println!("{}", MyEnum::to_dart());
    //println!("{}", MyEnum2::to_dart());
}
