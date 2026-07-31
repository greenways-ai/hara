use crate::lang::data::{List, Tuple};
use crate::lang::hash::JavaHash;
use crate::lang::protocol::{
    HashType, IColl, IConj, ICons, ICount, IDisplay, IEmpty, IEquality, IHash, IMetadata, INth,
    IObjType, IPeekFirst, IPersistent, IPopFirst, IPushFirst, ObjType,
};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Cons<E, M = List<E>> {
    metadata: Option<Rc<crate::lang::data::Metadata>>,
    first: E,
    more: M,
}

impl<E: Clone + PartialEq, M: Clone + PartialEq> PartialEq for Cons<E, M> {
    fn eq(&self, other: &Self) -> bool {
        self.first == other.first && self.more == other.more
    }
}
impl<E: Clone, M: Clone + IntoIterator<Item = E>> Cons<E, M> {
    pub fn new(first: E, more: M) -> Self {
        Self {
            metadata: None,
            first,
            more,
        }
    }
    pub fn peek_first(&self) -> &E {
        &self.first
    }
    pub fn iter(&self) -> impl Iterator<Item = E> {
        std::iter::once(self.first.clone()).chain(self.more.clone())
    }
    pub fn to_list(&self) -> List<E> {
        self.iter().collect()
    }
}
impl<E: Clone, M: Clone + IntoIterator<Item = E>> IntoIterator for Cons<E, M> {
    type Item = E;
    type IntoIter = std::vec::IntoIter<E>;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.first)
            .chain(self.more)
            .collect::<Vec<_>>()
            .into_iter()
    }
}
impl<E: Clone, M: Clone + IntoIterator<Item = E>> ICount for Cons<E, M> {
    fn count(&self) -> usize {
        1 + self.more.clone().into_iter().count()
    }
}
impl<E: Clone> INth<E> for Cons<E> {
    fn nth(&self, index: usize) -> Option<&E> {
        if index == 0 {
            Some(&self.first)
        } else {
            self.more.get(index - 1)
        }
    }
}
impl<E: Clone, M: Clone> IPeekFirst<E> for Cons<E, M> {
    fn peek_first(&self) -> Option<E> {
        Some(self.first.clone())
    }
}
impl<E: Clone, M: Clone> IPushFirst<E> for Cons<E, M> {
    type Output = Cons<E, Self>;
    fn push_first(&self, value: E) -> Self::Output {
        Cons {
            metadata: self.metadata.clone(),
            first: value,
            more: self.clone(),
        }
    }
}
impl<E: Clone, M: Clone> IConj<E> for Cons<E, M> {
    type Output = Cons<E, Self>;
    fn conj(&self, value: E) -> Self::Output {
        self.push_first(value)
    }
}
impl<E: Clone, M: Clone> ICons<E> for Cons<E, M> {
    type Output = Cons<E, Self>;
    fn cons(&self, value: E) -> Self::Output {
        Cons {
            metadata: None,
            first: value,
            more: self.clone(),
        }
    }
}
impl<E: Clone, M: Clone> IPopFirst for Cons<E, M> {
    type Output = M;
    fn pop_first(&self) -> Self::Output {
        self.more.clone()
    }
}
impl<E: Clone, M: Clone> IEmpty for Cons<E, M> {
    type Output = Tuple<E>;
    fn empty(&self) -> Self::Output {
        Tuple::Tup0.with_meta(self.metadata.clone())
    }
}
impl<E: Clone, M: Clone> IMetadata for Cons<E, M> {
    type Metadata = Rc<crate::lang::data::Metadata>;
    fn meta(&self) -> Option<&Self::Metadata> {
        self.metadata.as_ref()
    }
    fn with_meta(&self, metadata: Option<Self::Metadata>) -> Self {
        Self {
            metadata,
            ..self.clone()
        }
    }
}
impl<E: Clone, M: Clone> IPersistent for Cons<E, M> {}
impl<E: Clone + PartialEq, M: Clone + IntoIterator<Item = E>> IEquality for Cons<E, M> {
    fn equality(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}
impl<E: Clone + std::fmt::Debug, M: Clone + IntoIterator<Item = E>> IDisplay for Cons<E, M> {
    fn display(&self) -> String {
        format!(
            "({})",
            self.iter()
                .map(|v| format!("{v:?}"))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}
impl<E: Clone + std::hash::Hash + JavaHash, M: Clone + IntoIterator<Item = E>> IHash
    for Cons<E, M>
{
    fn hash_calc(&self, hash_type: HashType) -> u64 {
        // Java Cons is sequential: ordered composition, "::SEQUENTIAL" seed.
        crate::lang::hash::compose_ordered(
            "SEQUENTIAL",
            self.iter().map(|value| value.java_hash(hash_type)),
        ) as u64
    }
}
impl<E: Clone + std::fmt::Debug, M: Clone + IntoIterator<Item = E>> IObjType for Cons<E, M> {
    fn obj_type(&self) -> ObjType {
        ObjType::Sequential
    }
}
impl<E, M> IColl<E> for Cons<E, M>
where
    E: Clone + PartialEq + std::fmt::Debug + std::hash::Hash + JavaHash,
    M: Clone + IntoIterator<Item = E>,
{
    fn start_string(&self) -> &'static str {
        "("
    }
    fn end_string(&self) -> &'static str {
        ")"
    }
}

#[cfg(test)]
mod tests {
    use super::Cons;
    use crate::lang::data::List;
    use crate::lang::protocol::{
        IConj, ICons, ICount, IEmpty, IMetadata, INth, IPopFirst, IPushFirst,
    };
    #[test]
    fn linked_navigation() {
        let c = Cons::new(1, vec![2, 3].into_iter().collect::<List<_>>());
        assert_eq!(c.count(), 3);
        assert_eq!(c.nth(2), Some(&3));
        assert_eq!(c.pop_first(), List::from(vec![2, 3]));
        assert!(Cons::new(1, List::new()).pop_first().is_empty());

        let documented = c.with_meta(Some(crate::lang::data::Metadata::document("doc")));
        assert_eq!(
            documented.push_first(0).meta().map(|m| m.doc().unwrap()),
            Some("doc")
        );
        let pushed = documented.push_first(0);
        assert_eq!(pushed.peek_first(), &0);
        assert_eq!(pushed.pop_first(), documented);
        let conjoined = documented.conj(0);
        assert_eq!(conjoined.pop_first(), documented);
        assert_eq!(conjoined.meta().map(|m| m.doc().unwrap()), Some("doc"));
        let consed = documented.cons(0);
        assert_eq!(consed.meta(), None);
        assert_eq!(consed.pop_first(), documented);
        assert!(documented.empty().is_empty());
        assert_eq!(
            documented.empty().meta().map(|m| m.doc().unwrap()),
            Some("doc")
        );
    }
}
