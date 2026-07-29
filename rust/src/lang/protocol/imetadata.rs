#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetaType {
    Object,
    Map,
    String,
}

pub trait IMetadata: Sized {
    type Metadata: Clone;

    fn meta(&self) -> Option<&Self::Metadata>;
    fn with_meta(&self, metadata: Option<Self::Metadata>) -> Self;

    fn metatype(&self) -> MetaType {
        MetaType::Object
    }
}
