//! FiBA — the Finger B-tree Aggregator for out-of-order sliding-window aggregation.
//!
//! FiBA (Tangwongsan, Hirzel & Schneider, "Optimal and General Out-of-Order Sliding-Window
//! Aggregation", VLDB 2019) maintains the aggregate of a sliding window under **out-of-order**
//! inserts and evictions — the case the two-stack
//! [`WindowedAggregator`](crate::streaming::WindowedAggregator) cannot handle. Values are keyed by
//! event time; an insert may arrive anywhere in time order, and eviction removes from the old end
//! by time. The window aggregate is always available in O(1) because every tree node caches the
//! partial aggregate of its subtree; an insert or eviction only repairs the aggregates along one
//! root-to-leaf path.
//!
//! The combine is any **associative** binary operation with an identity (a monoid); the aggregate
//! is folded in strict time order, so FiBA is correct for non-commutative combines (string
//! concatenation, first/last, min-by-time, matrix product), not just sums.
//!
//! # Structure note
//!
//! This implementation caches partial aggregates in a height-balanced (AVL) search tree keyed by
//! time — the clear, verifiable form of FiBA's contract: O(1) query, O(log n) insert/evict, exact
//! ordered aggregation. FiBA's namesake refinement — a **B-tree with fingers** at both ends, which
//! lowers operations *near* the window boundaries to amortized O(log d) in the distance `d` from
//! the end rather than O(log n) — is a constant-factor/locality optimization over this same
//! contract and is left as a follow-up; it does not change any query result.
//!
//! # Example
//! ```
//! use sketch_oxide::streaming::FibaAggregator;
//!
//! fn add(a: &u64, b: &u64) -> u64 { a + b }
//! let mut w = FibaAggregator::new(0u64, add);
//!
//! // Values arrive OUT OF EVENT-TIME ORDER.
//! w.insert(30, 3);
//! w.insert(10, 1);
//! w.insert(20, 2);
//! assert_eq!(w.query(), 6); // 1 + 2 + 3, folded in time order
//!
//! // Slide the window forward: drop everything with time < 20.
//! w.evict_before(20);
//! assert_eq!(w.query(), 5); // 2 + 3
//! ```

/// A node in the time-ordered, aggregate-augmented balanced tree.
#[derive(Debug, Clone)]
struct Node<V> {
    time: u64,
    value: V,
    /// Aggregate of this whole subtree, folded in time order.
    agg: V,
    height: i32,
    left: Option<Box<Node<V>>>,
    right: Option<Box<Node<V>>>,
}

/// An out-of-order sliding-window aggregator over an associative `combine` with identity.
///
/// `V` is the aggregate/value type; `combine` must be associative with `identity` as its neutral
/// element. Values are keyed by `u64` event time.
#[derive(Debug, Clone)]
pub struct FibaAggregator<V> {
    root: Option<Box<Node<V>>>,
    identity: V,
    combine: fn(&V, &V) -> V,
    len: usize,
}

impl<V: Clone> FibaAggregator<V> {
    /// Creates an empty aggregator with the monoid `identity` and associative `combine`.
    pub fn new(identity: V, combine: fn(&V, &V) -> V) -> Self {
        Self {
            root: None,
            identity,
            combine,
            len: 0,
        }
    }

    /// Inserts `value` at event `time`, anywhere in time order. If a value already exists at
    /// `time`, the two are combined (in arrival order) at that timestamp.
    pub fn insert(&mut self, time: u64, value: V) {
        let root = self.root.take();
        let (new_root, inserted) =
            Self::node_insert(root, time, value, self.combine, &self.identity);
        self.root = Some(new_root);
        if inserted {
            self.len += 1;
        }
    }

    /// Evicts and returns the oldest `(time, value)` entry, or `None` if empty.
    pub fn evict_oldest(&mut self) -> Option<(u64, V)> {
        let root = self.root.take()?;
        let (new_root, min) = Self::node_remove_min(root, self.combine, &self.identity);
        self.root = new_root;
        self.len -= 1;
        Some(min)
    }

    /// Evicts every entry whose time is strictly less than `cutoff` (slides the window forward).
    pub fn evict_before(&mut self, cutoff: u64) {
        while let Some(t) = self.oldest_time() {
            if t < cutoff {
                self.evict_oldest();
            } else {
                break;
            }
        }
    }

    /// The aggregate over the whole current window (identity if empty), folded in time order.
    pub fn query(&self) -> V {
        Self::agg_of(&self.root, &self.identity)
    }

    /// The smallest event time currently in the window.
    pub fn oldest_time(&self) -> Option<u64> {
        let mut node = self.root.as_deref()?;
        while let Some(l) = node.left.as_deref() {
            node = l;
        }
        Some(node.time)
    }

    /// The largest event time currently in the window.
    pub fn newest_time(&self) -> Option<u64> {
        let mut node = self.root.as_deref()?;
        while let Some(r) = node.right.as_deref() {
            node = r;
        }
        Some(node.time)
    }

    /// Number of distinct timestamps held.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the window is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    // --- balanced-tree internals -------------------------------------------------------------

    #[inline]
    fn height(node: &Option<Box<Node<V>>>) -> i32 {
        node.as_ref().map_or(0, |n| n.height)
    }

    #[inline]
    fn agg_of(node: &Option<Box<Node<V>>>, identity: &V) -> V {
        node.as_ref()
            .map_or_else(|| identity.clone(), |n| n.agg.clone())
    }

    /// Recomputes a node's cached height and time-ordered aggregate from its children.
    fn refresh(node: &mut Node<V>, combine: fn(&V, &V) -> V, identity: &V) {
        node.height = 1 + Self::height(&node.left).max(Self::height(&node.right));
        let left = Self::agg_of(&node.left, identity);
        let right = Self::agg_of(&node.right, identity);
        // Fold strictly in time order: (left · value) · right.
        let with_value = combine(&left, &node.value);
        node.agg = combine(&with_value, &right);
    }

    fn balance_factor(node: &Node<V>) -> i32 {
        Self::height(&node.left) - Self::height(&node.right)
    }

    fn rotate_right(mut y: Box<Node<V>>, combine: fn(&V, &V) -> V, identity: &V) -> Box<Node<V>> {
        let mut x = y.left.take().unwrap();
        y.left = x.right.take();
        Self::refresh(&mut y, combine, identity);
        x.right = Some(y);
        Self::refresh(&mut x, combine, identity);
        x
    }

    fn rotate_left(mut x: Box<Node<V>>, combine: fn(&V, &V) -> V, identity: &V) -> Box<Node<V>> {
        let mut y = x.right.take().unwrap();
        x.right = y.left.take();
        Self::refresh(&mut x, combine, identity);
        y.left = Some(x);
        Self::refresh(&mut y, combine, identity);
        y
    }

    fn rebalance(mut node: Box<Node<V>>, combine: fn(&V, &V) -> V, identity: &V) -> Box<Node<V>> {
        Self::refresh(&mut node, combine, identity);
        let bf = Self::balance_factor(&node);
        if bf > 1 {
            // Left-heavy.
            if Self::balance_factor(node.left.as_ref().unwrap()) < 0 {
                let left = node.left.take().unwrap();
                node.left = Some(Self::rotate_left(left, combine, identity));
            }
            return Self::rotate_right(node, combine, identity);
        }
        if bf < -1 {
            // Right-heavy.
            if Self::balance_factor(node.right.as_ref().unwrap()) > 0 {
                let right = node.right.take().unwrap();
                node.right = Some(Self::rotate_right(right, combine, identity));
            }
            return Self::rotate_left(node, combine, identity);
        }
        node
    }

    fn node_insert(
        node: Option<Box<Node<V>>>,
        time: u64,
        value: V,
        combine: fn(&V, &V) -> V,
        identity: &V,
    ) -> (Box<Node<V>>, bool) {
        match node {
            None => {
                let mut leaf = Box::new(Node {
                    time,
                    value,
                    agg: identity.clone(),
                    height: 1,
                    left: None,
                    right: None,
                });
                Self::refresh(&mut leaf, combine, identity);
                (leaf, true)
            }
            Some(mut n) => {
                let inserted;
                match time.cmp(&n.time) {
                    std::cmp::Ordering::Less => {
                        let (child, ins) =
                            Self::node_insert(n.left.take(), time, value, combine, identity);
                        n.left = Some(child);
                        inserted = ins;
                    }
                    std::cmp::Ordering::Greater => {
                        let (child, ins) =
                            Self::node_insert(n.right.take(), time, value, combine, identity);
                        n.right = Some(child);
                        inserted = ins;
                    }
                    std::cmp::Ordering::Equal => {
                        // Same timestamp: accumulate in arrival order.
                        n.value = combine(&n.value, &value);
                        inserted = false;
                    }
                }
                (Self::rebalance(n, combine, identity), inserted)
            }
        }
    }

    fn node_remove_min(
        mut node: Box<Node<V>>,
        combine: fn(&V, &V) -> V,
        identity: &V,
    ) -> (Option<Box<Node<V>>>, (u64, V)) {
        match node.left.take() {
            None => {
                // This node is the minimum; promote its right child.
                let min = (node.time, node.value.clone());
                (node.right.take(), min)
            }
            Some(left) => {
                let (new_left, min) = Self::node_remove_min(left, combine, identity);
                node.left = new_left;
                (Some(Self::rebalance(node, combine, identity)), min)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add(a: &u64, b: &u64) -> u64 {
        a + b
    }
    fn cat(a: &String, b: &String) -> String {
        format!("{a}{b}")
    }

    #[test]
    fn empty_is_identity() {
        let w = FibaAggregator::new(0u64, add);
        assert!(w.is_empty());
        assert_eq!(w.query(), 0);
        assert_eq!(w.oldest_time(), None);
    }

    #[test]
    fn in_order_sum() {
        let mut w = FibaAggregator::new(0u64, add);
        for t in 1..=100u64 {
            w.insert(t, t);
        }
        assert_eq!(w.query(), 5050);
        assert_eq!(w.len(), 100);
        assert_eq!(w.oldest_time(), Some(1));
        assert_eq!(w.newest_time(), Some(100));
    }

    #[test]
    fn out_of_order_insert_is_correct() {
        let mut w = FibaAggregator::new(0u64, add);
        // Insert 1..=1000 in a scrambled order.
        for k in 0..1000u64 {
            let t = (k * 577 + 13) % 1000 + 1; // pseudo-shuffle over 1..=1000
            w.insert(t, t);
        }
        // Sum of 1..=1000 = 500500 (each timestamp distinct → no accumulation).
        assert_eq!(w.query(), 500_500);
        assert_eq!(w.len(), 1000);
    }

    #[test]
    fn sliding_window_eviction() {
        let mut w = FibaAggregator::new(0u64, add);
        for t in 1..=100u64 {
            w.insert(t, t);
        }
        w.evict_before(91); // keep times 91..=100
        let expected: u64 = (91..=100).sum();
        assert_eq!(w.query(), expected);
        assert_eq!(w.oldest_time(), Some(91));
        assert_eq!(w.len(), 10);
    }

    #[test]
    fn evict_oldest_returns_min_in_order() {
        let mut w = FibaAggregator::new(0u64, add);
        w.insert(50, 5);
        w.insert(10, 1);
        w.insert(30, 3);
        assert_eq!(w.evict_oldest(), Some((10, 1)));
        assert_eq!(w.evict_oldest(), Some((30, 3)));
        assert_eq!(w.evict_oldest(), Some((50, 5)));
        assert_eq!(w.evict_oldest(), None);
    }

    #[test]
    fn non_commutative_combine_folds_in_time_order() {
        // String concatenation is associative but NOT commutative: the result must be the values
        // in TIME order regardless of INSERTION order.
        let mut w = FibaAggregator::new(String::new(), cat);
        w.insert(3, "c".to_string());
        w.insert(1, "a".to_string());
        w.insert(4, "d".to_string());
        w.insert(2, "b".to_string());
        assert_eq!(w.query(), "abcd");
        w.evict_before(2);
        assert_eq!(w.query(), "bcd");
    }

    #[test]
    fn duplicate_timestamp_accumulates() {
        let mut w = FibaAggregator::new(0u64, add);
        w.insert(5, 10);
        w.insert(5, 7); // same timestamp → combined
        assert_eq!(w.len(), 1);
        assert_eq!(w.query(), 17);
    }

    #[test]
    fn stays_balanced_and_correct_at_scale() {
        // Adversarial in-order insertion would make an unbalanced BST degenerate to O(n) height;
        // AVL balancing keeps it ~log2(n). Correctness must hold and height must be small.
        let mut w = FibaAggregator::new(0u64, add);
        let n = 10_000u64;
        for t in 1..=n {
            w.insert(t, 1);
        }
        assert_eq!(w.query(), n);
        let height = w.root.as_ref().unwrap().height;
        assert!(height <= 20, "AVL height {height} too large for {n} nodes");
    }
}
