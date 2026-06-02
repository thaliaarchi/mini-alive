//! Typed arena with IDs.

use std::{
    any, fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Index, Range},
};

/// A typed arena.
#[derive(Clone, PartialEq, Eq)]
pub struct Arena<T> {
    values: Vec<T>,
}

/// The ID of a value in an arena.
pub struct Id<T> {
    index: u32,
    marker: PhantomData<T>,
}

impl<T> Arena<T> {
    /// Constructs an empty arena.
    pub fn new() -> Self {
        Arena { values: Vec::new() }
    }

    /// Inserts a value into the context and returns its ID.
    pub fn insert(&mut self, value: T) -> Id<T> {
        self.values.push(value);
        Id::from_index(self.values.len() - 1)
    }

    /// Returns an iterator over the IDs of the values in the arena.
    pub fn iter_ids(&self) -> IdIter<T> {
        Id::iter(Id::from_index(0)..Id::from_index(self.values.len()))
    }

    /// Clears the arena, retaining the allocation.
    pub fn clear(&mut self) {
        self.values.clear();
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output {
        &self.values[id.index()]
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Arena::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for Arena<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Arena<{}> ", type_name::<T>())?;
        f.debug_map()
            .entries(self.iter_ids().map(|id| (id, &self[id])))
            .finish()
    }
}

impl<T> Id<T> {
    /// Constructs an ID from an index.
    pub fn from_index(index: usize) -> Self {
        let Ok(index) = u32::try_from(index) else {
            panic!("Id overflow");
        };
        Id {
            index,
            marker: PhantomData,
        }
    }

    /// Gets the index of the ID.
    pub fn index(self) -> usize {
        self.index as usize
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Id<T> {}
impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id<{}>({})", type_name::<T>(), self.index)
    }
}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}
impl<T> Eq for Id<T> {}
impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}
impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

/// Iterator over IDs in a half-open range.
pub struct IdIter<T> {
    front: u32,
    back: u32,
    marker: PhantomData<T>,
}

impl<T> Id<T> {
    /// Creates an iterator over IDs in a half-open range.
    pub fn iter(range: Range<Self>) -> IdIter<T> {
        IdIter {
            front: range.start.index,
            back: range.end.index.max(range.start.index),
            marker: PhantomData,
        }
    }
}

impl<T> Iterator for IdIter<T> {
    type Item = Id<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }
        let id = Id {
            index: self.front,
            marker: PhantomData,
        };
        self.front += 1;
        Some(id)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.back - self.front) as usize;
        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for IdIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        Some(Id {
            index: self.back,
            marker: PhantomData,
        })
    }
}

impl<T> ExactSizeIterator for IdIter<T> {}

impl<T> Clone for IdIter<T> {
    fn clone(&self) -> Self {
        IdIter {
            front: self.front,
            back: self.back,
            marker: PhantomData,
        }
    }
}
impl<T> fmt::Debug for IdIter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let t = type_name::<T>();
        write!(f, "IdIter<{t}>({}..{})", self.front, self.back)
    }
}

fn type_name<T>() -> &'static str {
    let type_name = any::type_name::<T>();
    let mut start = 0;
    let mut end = type_name.len();
    for (i, &b) in type_name.as_bytes().iter().enumerate() {
        if b == b'<' {
            end = i;
            break;
        } else if b == b':' && type_name.as_bytes().get(i + 1) == Some(&b':') {
            start = i + 2;
        }
    }
    &type_name[start..end]
}

#[cfg(test)]
mod tests {
    use crate::syntax::inst::Inst;

    use super::*;

    #[test]
    fn fmt_type_name() {
        assert_eq!(type_name::<Inst>(), "Inst");
        assert_eq!(type_name::<String>(), "String");
        assert_eq!(type_name::<Option<String>>(), "Option");
        assert_eq!(format!("{:?}", Id::<Inst>::from_index(0)), "Id<Inst>(0)");
        assert_eq!(
            format!("{:?}", Id::<String>::from_index(0)),
            "Id<String>(0)",
        );
        assert_eq!(
            format!("{:?}", Id::<Option<String>>::from_index(0)),
            "Id<Option>(0)",
        );
    }
}
