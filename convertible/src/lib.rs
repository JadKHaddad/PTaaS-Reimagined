#[cfg(feature = "derive")]
pub mod macros;

pub trait DartConvertible {
    fn to_dart(&self) -> &'static str;
}
