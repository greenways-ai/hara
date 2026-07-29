pub trait IEquality<Rhs: ?Sized = Self> {
    fn equality(&self, other: &Rhs) -> bool;
}
