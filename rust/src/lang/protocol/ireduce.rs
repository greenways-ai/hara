pub trait IReduce<A, F> {
    type Error;

    fn reduce(&self, function: F, initial: Option<A>) -> Result<A, Self::Error>;
}
