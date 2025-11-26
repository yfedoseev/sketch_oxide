//! Common types used across sketch implementations

/// Represents the difference between two sets for reconciliation
///
/// Used by the [`Reconcilable`](crate::common::Reconcilable) trait to encode
/// set differences for synchronization protocols.
///
/// For key-value stores (like IBLT), each item is a (key, value) tuple.
/// For simple sets, use empty values or duplicate the key.
///
/// # Example
/// ```
/// use sketch_oxide::common::SetDifference;
///
/// // Key-value pairs
/// let diff = SetDifference {
///     to_insert: vec![(b"key1".to_vec(), b"value1".to_vec())],
///     to_remove: vec![(b"key2".to_vec(), b"value2".to_vec())],
/// };
///
/// assert_eq!(diff.to_insert.len(), 1);
/// assert_eq!(diff.to_remove.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetDifference {
    /// Items (key-value pairs) that should be inserted into the target set
    pub to_insert: Vec<(Vec<u8>, Vec<u8>)>,

    /// Items (key-value pairs) that should be removed from the target set
    pub to_remove: Vec<(Vec<u8>, Vec<u8>)>,
}

impl SetDifference {
    /// Creates a new empty SetDifference
    pub fn new() -> Self {
        Self {
            to_insert: Vec::new(),
            to_remove: Vec::new(),
        }
    }

    /// Creates a SetDifference with specified insertions and deletions
    pub fn with_changes(
        to_insert: Vec<(Vec<u8>, Vec<u8>)>,
        to_remove: Vec<(Vec<u8>, Vec<u8>)>,
    ) -> Self {
        Self {
            to_insert,
            to_remove,
        }
    }

    /// Returns true if this difference represents no changes
    pub fn is_empty(&self) -> bool {
        self.to_insert.is_empty() && self.to_remove.is_empty()
    }

    /// Returns the total number of changes (insertions + deletions)
    pub fn total_changes(&self) -> usize {
        self.to_insert.len() + self.to_remove.len()
    }
}

impl Default for SetDifference {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let diff = SetDifference::new();
        assert!(diff.is_empty());
        assert_eq!(diff.total_changes(), 0);
    }

    #[test]
    fn test_with_changes() {
        let diff = SetDifference::with_changes(
            vec![(b"key1".to_vec(), b"insert".to_vec())],
            vec![(b"key2".to_vec(), b"remove".to_vec())],
        );
        assert!(!diff.is_empty());
        assert_eq!(diff.total_changes(), 2);
    }

    #[test]
    fn test_clone() {
        let diff1 = SetDifference::with_changes(
            vec![(b"a".to_vec(), b"va".to_vec())],
            vec![(b"b".to_vec(), b"vb".to_vec())],
        );
        let diff2 = diff1.clone();
        assert_eq!(diff1, diff2);
    }

    #[test]
    fn test_debug() {
        let diff = SetDifference::new();
        let debug_str = format!("{:?}", diff);
        assert!(debug_str.contains("SetDifference"));
    }
}
