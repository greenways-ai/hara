pub trait IValidate<V> {
    type Error;
    fn validate(&self, value: &V) -> Result<(), Self::Error>;
}
