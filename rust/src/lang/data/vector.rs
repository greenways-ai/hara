use std::cell::Cell;
use std::rc::Rc;

use crate::lang::protocol::ihash::HashType;
use crate::lang::protocol::{
    IAssoc, IColl, IConj, ICount, IDisplay, IEmpty, IEquality, IHash, IMetadata, IMutable, INth,
    IObjType, IPeekFirst, IPeekLast, IPersistent, IPopLast, IPushLast, IToMutable, IToPersistent,
    ObjType,
};

const NODE_SHIFT: usize = 5;
const NODE_WIDTH: usize = 1 << NODE_SHIFT;
const NODE_MASK: usize = NODE_WIDTH - 1;

#[derive(Debug, Clone)]
enum Node<E> {
    Branch(Rc<Vec<Option<Rc<Node<E>>>>>),
    Leaf(Rc<Vec<E>>),
}

impl<E> Node<E> {
    fn empty() -> Rc<Self> {
        Rc::new(Self::Branch(Rc::new(vec![None; NODE_WIDTH])))
    }
}

#[derive(Debug, Clone)]
pub struct Standard<E> {
    metadata: Option<Rc<crate::lang::data::Metadata>>,
    size: usize,
    shift: usize,
    root: Rc<Node<E>>,
    tail: Rc<Vec<E>>,
}

impl<E: Clone> Default for Standard<E> {
    fn default() -> Self {
        Self {
            metadata: None,
            size: 0,
            shift: NODE_SHIFT,
            root: Node::empty(),
            tail: Rc::new(Vec::new()),
        }
    }
}

impl<E: Clone> Standard<E> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_iter(values: impl IntoIterator<Item = E>) -> Self {
        values
            .into_iter()
            .fold(Self::new(), |vector, value| vector.push_last(value))
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    fn tail_offset(&self) -> usize {
        if self.size < NODE_WIDTH {
            0
        } else {
            ((self.size - 1) >> NODE_SHIFT) << NODE_SHIFT
        }
    }

    pub fn get(&self, index: usize) -> Option<&E> {
        if index >= self.size {
            return None;
        }
        if index >= self.tail_offset() {
            return self.tail.get(index & NODE_MASK);
        }

        let mut node = self.root.as_ref();
        let mut level = self.shift;
        while level > 0 {
            let Node::Branch(children) = node else {
                return None;
            };
            node = children[(index >> level) & NODE_MASK].as_deref()?;
            level -= NODE_SHIFT;
        }
        match node {
            Node::Leaf(values) => values.get(index & NODE_MASK),
            Node::Branch(_) => None,
        }
    }

    pub fn assoc_value(&self, index: usize, value: E) -> Option<Self> {
        if index == self.size {
            return Some(self.push_last(value));
        }
        if index >= self.size {
            return None;
        }
        if index >= self.tail_offset() {
            let mut tail = (*self.tail).clone();
            tail[index & NODE_MASK] = value;
            return Some(Self {
                tail: Rc::new(tail),
                ..self.clone()
            });
        }
        Some(Self {
            root: assoc_node(&self.root, self.shift, index, value),
            ..self.clone()
        })
    }

    pub fn push_last(&self, value: E) -> Self {
        if self.tail.len() < NODE_WIDTH {
            let mut tail = (*self.tail).clone();
            tail.push(value);
            return Self {
                size: self.size + 1,
                tail: Rc::new(tail),
                ..self.clone()
            };
        }

        let tail_node = Rc::new(Node::Leaf(self.tail.clone()));
        let overflow = (self.size >> NODE_SHIFT) > (1usize << self.shift);
        let (root, shift) = if overflow {
            let mut children = vec![None; NODE_WIDTH];
            children[0] = Some(self.root.clone());
            children[1] = Some(new_path(self.shift, tail_node));
            (
                Rc::new(Node::Branch(Rc::new(children))),
                self.shift + NODE_SHIFT,
            )
        } else {
            (
                push_tail(&self.root, self.shift, self.size, tail_node),
                self.shift,
            )
        };

        Self {
            metadata: self.metadata.clone(),
            size: self.size + 1,
            shift,
            root,
            tail: Rc::new(vec![value]),
        }
    }

    pub fn pop_last_value(&self) -> Option<Self> {
        if self.size == 0 {
            return None;
        }
        if self.size == 1 {
            return Some(Self {
                metadata: self.metadata.clone(),
                ..Self::new()
            });
        }
        if self.size - self.tail_offset() > 1 {
            let mut tail = (*self.tail).clone();
            tail.pop();
            return Some(Self {
                size: self.size - 1,
                tail: Rc::new(tail),
                ..self.clone()
            });
        }

        let new_tail = self
            .array_for(self.size - 2)
            .expect("previous vector leaf")
            .clone();
        let mut root = pop_tail(&self.root, self.shift, self.size).unwrap_or_else(Node::empty);
        let mut shift = self.shift;
        if shift > NODE_SHIFT {
            if let Node::Branch(children) = root.as_ref() {
                if children[1].is_none() {
                    root = children[0].clone().expect("collapsed vector root");
                    shift -= NODE_SHIFT;
                }
            }
        }
        Some(Self {
            metadata: self.metadata.clone(),
            size: self.size - 1,
            shift,
            root,
            tail: Rc::new(new_tail),
        })
    }

    fn array_for(&self, index: usize) -> Option<&Vec<E>> {
        if index >= self.size {
            return None;
        }
        if index >= self.tail_offset() {
            return Some(&self.tail);
        }
        let mut node = self.root.as_ref();
        let mut level = self.shift;
        while level > 0 {
            let Node::Branch(children) = node else {
                return None;
            };
            node = children[(index >> level) & NODE_MASK].as_deref()?;
            level -= NODE_SHIFT;
        }
        match node {
            Node::Leaf(values) => Some(values),
            Node::Branch(_) => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, E> {
        Iter {
            vector: self,
            index: 0,
        }
    }

    pub fn subview(&self, start: usize, end: usize) -> Option<SubView<E>> {
        if start > end || end > self.size {
            return None;
        }
        Some(SubView {
            vector: self.clone(),
            start,
            end,
        })
    }

    #[cfg(test)]
    fn shares_root_with(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.root, &other.root)
    }
}

fn new_path<E: Clone>(level: usize, node: Rc<Node<E>>) -> Rc<Node<E>> {
    if level == 0 {
        return node;
    }
    let mut children = vec![None; NODE_WIDTH];
    children[0] = Some(new_path(level - NODE_SHIFT, node));
    Rc::new(Node::Branch(Rc::new(children)))
}

fn push_tail<E: Clone>(
    parent: &Rc<Node<E>>,
    level: usize,
    size: usize,
    tail: Rc<Node<E>>,
) -> Rc<Node<E>> {
    let Node::Branch(existing) = parent.as_ref() else {
        unreachable!("vector tree parent must be a branch")
    };
    let mut children = (**existing).clone();
    let index = ((size - 1) >> level) & NODE_MASK;
    children[index] = Some(if level == NODE_SHIFT {
        tail
    } else if let Some(child) = &children[index] {
        push_tail(child, level - NODE_SHIFT, size, tail)
    } else {
        new_path(level - NODE_SHIFT, tail)
    });
    Rc::new(Node::Branch(Rc::new(children)))
}

fn assoc_node<E: Clone>(node: &Rc<Node<E>>, level: usize, index: usize, value: E) -> Rc<Node<E>> {
    if level == 0 {
        let Node::Leaf(existing) = node.as_ref() else {
            unreachable!("vector terminal node must be a leaf")
        };
        let mut values = (**existing).clone();
        values[index & NODE_MASK] = value;
        return Rc::new(Node::Leaf(Rc::new(values)));
    }
    let Node::Branch(existing) = node.as_ref() else {
        unreachable!("vector path must contain branches")
    };
    let mut children = (**existing).clone();
    let child_index = (index >> level) & NODE_MASK;
    children[child_index] = Some(assoc_node(
        children[child_index]
            .as_ref()
            .expect("existing vector path"),
        level - NODE_SHIFT,
        index,
        value,
    ));
    Rc::new(Node::Branch(Rc::new(children)))
}

fn pop_tail<E: Clone>(node: &Rc<Node<E>>, level: usize, size: usize) -> Option<Rc<Node<E>>> {
    let Node::Branch(existing) = node.as_ref() else {
        unreachable!("vector path must contain branches")
    };
    let index = ((size - 2) >> level) & NODE_MASK;
    if level > NODE_SHIFT {
        let child = pop_tail(
            existing[index].as_ref().expect("existing vector path"),
            level - NODE_SHIFT,
            size,
        );
        if child.is_none() && index == 0 {
            return None;
        }
        let mut children = (**existing).clone();
        children[index] = child;
        Some(Rc::new(Node::Branch(Rc::new(children))))
    } else if index == 0 {
        None
    } else {
        let mut children = (**existing).clone();
        children[index] = None;
        Some(Rc::new(Node::Branch(Rc::new(children))))
    }
}

pub struct Iter<'a, E> {
    vector: &'a Standard<E>,
    index: usize,
}

impl<'a, E: Clone> Iterator for Iter<'a, E> {
    type Item = &'a E;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.vector.get(self.index);
        self.index += usize::from(value.is_some());
        value
    }
}

impl<E: Clone> FromIterator<E> for Standard<E> {
    fn from_iter<T: IntoIterator<Item = E>>(iter: T) -> Self {
        Self::from_iter(iter)
    }
}

impl<E: Clone> From<Vec<E>> for Standard<E> {
    fn from(values: Vec<E>) -> Self {
        Self::from_iter(values)
    }
}

impl<E: Clone> IntoIterator for Standard<E> {
    type Item = E;
    type IntoIter = std::vec::IntoIter<E>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter().cloned().collect::<Vec<_>>().into_iter()
    }
}

impl<E: Clone> std::ops::Index<usize> for Standard<E> {
    type Output = E;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("vector index out of bounds")
    }
}

impl<E: Clone + PartialEq> PartialEq for Standard<E> {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.iter().eq(other.iter())
    }
}

impl<E: Clone + PartialEq> IEquality for Standard<E> {
    fn equality(&self, other: &Self) -> bool {
        self == other
    }
}

impl<E: Clone> ICount for Standard<E> {
    fn count(&self) -> usize {
        self.size
    }
}

impl<E: Clone> INth<E> for Standard<E> {
    fn nth(&self, index: usize) -> Option<&E> {
        self.get(index)
    }
}

impl<E: Clone> IAssoc<usize, E> for Standard<E> {
    type Output = Self;
    fn assoc(&self, index: usize, value: E) -> Self {
        self.assoc_value(index, value)
            .expect("vector index out of bounds")
    }
}

impl<E: Clone> IPeekFirst<E> for Standard<E> {
    fn peek_first(&self) -> Option<E> {
        self.get(0).cloned()
    }
}
impl<E: Clone> IPeekLast<E> for Standard<E> {
    fn peek_last(&self) -> Option<E> {
        self.size
            .checked_sub(1)
            .and_then(|index| self.get(index))
            .cloned()
    }
}
impl<E: Clone> IPushLast<E> for Standard<E> {
    type Output = Self;
    fn push_last(&self, value: E) -> Self {
        Standard::push_last(self, value)
    }
}

impl<E: Clone> IPopLast for Standard<E> {
    type Output = Self;
    fn pop_last(&self) -> Self {
        self.pop_last_value().expect("cannot pop empty vector")
    }
}

impl<E: Clone> IConj<E> for Standard<E> {
    type Output = Self;
    fn conj(&self, value: E) -> Self {
        self.push_last(value)
    }
}

impl<E: Clone> IEmpty for Standard<E> {
    type Output = Self;
    fn empty(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            ..Self::new()
        }
    }
}

impl<E: Clone> IMetadata for Standard<E> {
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

impl<E: Clone> IPersistent for Standard<E> {}

impl<E: Clone> IToMutable for Standard<E> {
    type Mutable = Mutable<E>;

    fn to_mutable(&self) -> Self::Mutable {
        Mutable::from_standard(self)
    }
}

impl<E: Clone + std::fmt::Debug> IDisplay for Standard<E> {
    fn display(&self) -> String {
        format!(
            "[{}]",
            self.iter()
                .map(|value| format!("{value:?}"))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

impl<E: Clone + std::hash::Hash> IHash for Standard<E> {
    fn hash_calc(&self, _hash_type: HashType) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut state = std::collections::hash_map::DefaultHasher::new();
        self.iter().for_each(|value| value.hash(&mut state));
        state.finish()
    }
}

impl<E: Clone + std::fmt::Debug> IObjType for Standard<E> {
    fn obj_type(&self) -> ObjType {
        ObjType::Sequential
    }
}
impl<E> IColl<E> for Standard<E>
where
    E: Clone + PartialEq + std::fmt::Debug + std::hash::Hash,
{
    fn start_string(&self) -> &'static str {
        "["
    }
    fn end_string(&self) -> &'static str {
        "]"
    }
}

#[derive(Debug, Clone)]
pub struct Mutable<E> {
    editable: Rc<Cell<bool>>,
    values: Vec<E>,
    metadata: Option<Rc<crate::lang::data::Metadata>>,
}

impl<E: Clone> Mutable<E> {
    pub fn new() -> Self {
        Self {
            editable: Rc::new(Cell::new(true)),
            values: Vec::new(),
            metadata: None,
        }
    }

    pub fn from_iter(values: impl IntoIterator<Item = E>) -> Self {
        Self {
            values: values.into_iter().collect(),
            ..Self::new()
        }
    }

    fn from_standard(vector: &Standard<E>) -> Self {
        Self {
            editable: Rc::new(Cell::new(true)),
            values: vector.iter().cloned().collect(),
            metadata: vector.metadata.clone(),
        }
    }

    fn check_editable(&self) {
        assert!(
            self.editable.get(),
            "mutable vector used after to_persistent"
        );
    }

    pub fn len(&self) -> usize {
        self.check_editable();
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<&E> {
        self.check_editable();
        self.values.get(index)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, E> {
        self.check_editable();
        self.values.iter()
    }

    pub fn push_last(&mut self, value: E) -> &mut Self {
        self.check_editable();
        self.values.push(value);
        self
    }

    pub fn assoc(&mut self, index: usize, value: E) -> &mut Self {
        self.check_editable();
        if index == self.values.len() {
            return self.push_last(value);
        }
        self.values[index] = value;
        self
    }

    pub fn pop_last(&mut self) -> Option<E> {
        self.check_editable();
        self.values.pop()
    }

    pub fn empty(&mut self) -> &mut Self {
        self.check_editable();
        self.values.clear();
        self
    }
}

impl<E: Clone> Default for Mutable<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Clone> FromIterator<E> for Mutable<E> {
    fn from_iter<T: IntoIterator<Item = E>>(iter: T) -> Self {
        Self::from_iter(iter)
    }
}

impl<E: Clone> IMutable for Mutable<E> {}

impl<E: Clone> IToPersistent for Mutable<E> {
    type Persistent = Standard<E>;

    fn to_persistent(&mut self) -> Self::Persistent {
        self.check_editable();
        self.editable.set(false);
        let mut vector = Standard::from_iter(self.values.clone());
        vector.metadata = self.metadata.clone();
        vector
    }
}

#[derive(Debug, Clone)]
pub struct SubView<E> {
    vector: Standard<E>,
    start: usize,
    end: usize,
}

impl<E: Clone> SubView<E> {
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn get(&self, index: usize) -> Option<&E> {
        if index >= self.len() {
            None
        } else {
            self.vector.get(self.start + index)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Standard;
    use crate::lang::protocol::{IAssoc, IToMutable, IToPersistent};

    #[test]
    fn preserves_values_across_java_tree_boundaries() {
        let vector = (0..1057).collect::<Standard<_>>();
        let appended = vector.push_last(1057);
        let updated = appended.assoc(32, -1);

        assert_eq!(vector.len(), 1057);
        assert_eq!(vector[32], 32);
        assert_eq!(appended[1057], 1057);
        assert_eq!(updated[32], -1);
        assert_eq!(updated[1057], 1057);
    }

    #[test]
    fn tail_updates_share_the_tree() {
        let vector = (0..33).collect::<Standard<_>>();
        let appended = vector.push_last(33);

        assert!(vector.shares_root_with(&appended));
    }

    #[test]
    fn mutable_vector_matches_java_update_surface() {
        use super::Mutable;
        let mut vector = Mutable::from_iter([1, 2, 3]);
        assert_eq!(vector.len(), 3);
        assert_eq!(vector.get(1), Some(&2));
        vector.assoc(1, 5).assoc(3, 4);
        assert_eq!(vector.iter().copied().collect::<Vec<_>>(), vec![1, 5, 3, 4]);
        assert_eq!(vector.pop_last(), Some(4));
        vector.empty();
        assert!(vector.is_empty());
        assert_eq!(vector.pop_last(), None);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn mutable_vector_rejects_assoc_past_count() {
        let mut vector = super::Mutable::from_iter([1, 2, 3]);
        vector.assoc(4, 5);
    }

    #[test]
    #[should_panic(expected = "mutable vector used after to_persistent")]
    fn mutable_vector_is_invalid_after_persisting() {
        let vector = (0..4).collect::<Standard<_>>();
        let mut mutable = vector.to_mutable();
        let _persistent = mutable.to_persistent();
        mutable.push_last(5);
    }
}
