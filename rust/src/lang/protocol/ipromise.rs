use super::{IDeref, IDerefTimeout};

pub trait IPromise<V, F>: IDeref<Output = V> + IDerefTimeout<V> + Sized {
    type State;
    type Error;

    fn state(&self) -> Self::State;
    fn value(&self) -> Result<V, Self::Error>;
    fn then(&self, function: F) -> Self;
    fn catch(&self, function: F) -> Self;
    fn r#finally(&self, function: F) -> Self;
    fn cancel(&self) -> bool;
}
