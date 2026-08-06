pub trait IClose {
    type Error;

    fn close(&mut self) -> Result<(), Self::Error>;
}
