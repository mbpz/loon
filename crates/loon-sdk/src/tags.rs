//! Tag matchers and lightweight wrapper types for the SDK surface.
//!
//! Parlcant's SDK uses `AnyOf` / `AllOf` tag matchers as the
//! preferred way to express "any of these tags" / "all of these
//! tags" filters in client code. They're thin wrappers around
//! `Vec<TagId>` with the matching semantic encoded in the type.
//! Stores' `list(tags: &[TagId])` use AND semantics by default
//! (matches the InMemory impl); `AnyOf` is the OR variant.

use loon_core::TagId;

/// Tag filter that matches when the stored entity carries *any* of
/// the listed tags. Wraps a `Vec<TagId>` with the matching semantic
/// encoded in the type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnyOf(pub Vec<TagId>);

impl AnyOf {
    pub fn new(tags: impl IntoIterator<Item = TagId>) -> Self {
        Self(tags.into_iter().collect())
    }
    pub fn as_slice(&self) -> &[TagId] {
        &self.0
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<Vec<TagId>> for AnyOf {
    fn from(v: Vec<TagId>) -> Self {
        Self(v)
    }
}

impl IntoIterator for AnyOf {
    type Item = TagId;
    type IntoIter = std::vec::IntoIter<TagId>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Tag filter that matches when the stored entity carries *all* of
/// the listed tags. Wraps a `Vec<TagId>` with the matching semantic
/// encoded in the type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AllOf(pub Vec<TagId>);

impl AllOf {
    pub fn new(tags: impl IntoIterator<Item = TagId>) -> Self {
        Self(tags.into_iter().collect())
    }
    pub fn as_slice(&self) -> &[TagId] {
        &self.0
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<Vec<TagId>> for AllOf {
    fn from(v: Vec<TagId>) -> Self {
        Self(v)
    }
}

impl IntoIterator for AllOf {
    type Item = TagId;
    type IntoIter = std::vec::IntoIter<TagId>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anyof_collects_from_iterator() {
        let a = TagId::new();
        let b = TagId::new();
        let f = AnyOf::new([a.clone(), b.clone()]);
        assert_eq!(f.len(), 2);
        assert!(f.as_slice().contains(&a));
        assert!(f.as_slice().contains(&b));
    }

    #[test]
    fn allof_from_vec_round_trips() {
        let tags = vec![TagId::new(), TagId::new()];
        let f: AllOf = tags.clone().into();
        assert_eq!(f.0, tags);
    }

    #[test]
    fn anyof_into_iter_yields_tags() {
        let a = TagId::new();
        let b = TagId::new();
        let f = AnyOf::new([a.clone(), b.clone()]);
        let collected: Vec<TagId> = f.into_iter().collect();
        assert_eq!(collected, vec![a, b]);
    }
}
