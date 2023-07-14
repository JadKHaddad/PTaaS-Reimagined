pub trait DartConvertible {
    fn to_dart(&self) -> &'static str;
}
