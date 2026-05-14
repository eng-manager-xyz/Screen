//! Ordinal scale — category → index lookups.
//!
//! Used by colour encodings (`Color(Channel::field("region"))`) to
//! produce a stable palette index per category. The actual colour
//! comes from a palette indexed by `OrdinalScale::index_of`.

use std::collections::HashMap;
use std::hash::Hash;

/// Maps an ordered list of categories to dense `0..n` indices.
///
/// Lookup is O(1) via an internal hashmap. Categories iterated in
/// insertion order.
///
/// # Example
///
/// ```
/// use wisp_chart::scale::OrdinalScale;
/// let scale = OrdinalScale::new(["NA", "EU", "APAC"]);
/// assert_eq!(scale.index_of(&"NA"), Some(0));
/// assert_eq!(scale.index_of(&"APAC"), Some(2));
/// assert_eq!(scale.index_of(&"AF"), None);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct OrdinalScale<C: Eq + Hash + Clone> {
    categories: Vec<C>,
    lookup: HashMap<C, usize>,
}

impl<C: Eq + Hash + Clone> OrdinalScale<C> {
    /// Construct from a category iterator. Duplicate categories
    /// are de-duplicated keeping their first-seen position.
    pub fn new<I: IntoIterator<Item = C>>(categories: I) -> Self {
        let mut cats = Vec::new();
        let mut lookup = HashMap::new();
        for c in categories {
            if let std::collections::hash_map::Entry::Vacant(e) = lookup.entry(c.clone()) {
                e.insert(cats.len());
                cats.push(c);
            }
        }
        Self {
            categories: cats,
            lookup,
        }
    }

    /// Number of distinct categories.
    #[must_use]
    pub fn category_count(&self) -> usize {
        self.categories.len()
    }

    /// Index of `category` (insertion order), or `None` if absent.
    #[must_use]
    pub fn index_of(&self, category: &C) -> Option<usize> {
        self.lookup.get(category).copied()
    }

    /// Iterate categories in insertion order.
    pub fn categories(&self) -> impl Iterator<Item = &C> {
        self.categories.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_returns_insertion_order_index() {
        let s = OrdinalScale::new(["a", "b", "c"]);
        assert_eq!(s.index_of(&"a"), Some(0));
        assert_eq!(s.index_of(&"b"), Some(1));
        assert_eq!(s.index_of(&"c"), Some(2));
    }

    #[test]
    fn duplicates_keep_first_seen_position() {
        let s = OrdinalScale::new(["a", "b", "a", "c"]);
        assert_eq!(s.category_count(), 3);
        assert_eq!(s.index_of(&"a"), Some(0));
        assert_eq!(s.index_of(&"b"), Some(1));
        assert_eq!(s.index_of(&"c"), Some(2));
    }

    #[test]
    fn missing_category_returns_none() {
        let s = OrdinalScale::new(["a", "b"]);
        assert_eq!(s.index_of(&"ghost"), None);
    }

    #[test]
    fn categories_iter_preserves_order() {
        let s = OrdinalScale::new(["c", "a", "b"]);
        let collected: Vec<_> = s.categories().copied().collect();
        assert_eq!(collected, vec!["c", "a", "b"]);
    }
}
