use super::IClose;

pub trait ICoroutine<A>: IClose {
    type Status;
    type Output;

    fn status(&self) -> Self::Status;
    fn resume(&self, arguments: A) -> Result<Self::Output, Self::Error>;
}
