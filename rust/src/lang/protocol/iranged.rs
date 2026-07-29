pub trait IRanged<T> {
    fn range_min(&self) -> T;
    fn range_max(&self) -> T;
}
