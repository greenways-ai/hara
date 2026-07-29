use super::IMutable;

pub trait IToPersistent: IMutable {
    type Persistent;

    fn to_persistent(&mut self) -> Self::Persistent;
}
