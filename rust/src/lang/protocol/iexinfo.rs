pub trait IExInfo {
    type Data;

    fn data(&self) -> Self::Data;
}
