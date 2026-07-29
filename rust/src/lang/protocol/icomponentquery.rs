pub trait IComponentQuery<L> {
    type Info;
    type Health;

    fn started(&self) -> bool;
    fn stopped(&self) -> bool;
    fn info(&self, level: L) -> Self::Info;
    fn remote(&self) -> bool;
    fn health(&self) -> Self::Health;
}
