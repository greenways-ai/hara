use super::IPersistent;

pub trait IToMutable: IPersistent {
    type Mutable;

    fn to_mutable(&self) -> Self::Mutable;
}
