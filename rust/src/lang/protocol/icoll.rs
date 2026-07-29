use super::{IConj, ICount, IDisplay, IEmpty, IEquality, IHash};

pub trait IColl<E>:
    IntoIterator<Item = E> + IEquality + IConj<E> + IEmpty + ICount + IHash + IDisplay
{
    fn start_string(&self) -> &'static str;
    fn end_string(&self) -> &'static str;

    fn separator(&self) -> &'static str {
        " "
    }
}
