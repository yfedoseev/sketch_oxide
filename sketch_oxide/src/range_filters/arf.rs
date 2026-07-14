//! Adaptive Range Filter (ARF) — a self-tuning range filter that learns empty regions from queries
//! (Alexiou, Kossmann & Larson, "Adaptive Range Filters for Cold Data: Avoiding Trips to Siberia",
//! VLDB 2013).
//!
//! What a Bloom filter is for point queries, an ARF is for **range** queries: it answers "does the set
//! contain *any* key in `[lo, hi]`?" and may only err on the *false-positive* side. Unlike the static
//! range filters here, an ARF is **adaptive** — it is a binary trie over the key domain whose leaves
//! carry an `occupied` bit (maybe-contains-a-key) or are marked *empty* (definitely no key), and it
//! reshapes itself to spend its bit budget where queries actually land:
//!
//! * **Query** `[lo, hi]`: it is positive if *any* leaf overlapping `[lo, hi]` is occupied, negative
//!   only if every overlapping leaf is empty.
//! * **Escalation (learn).** When a query was a false positive — it said "maybe" but the data has
//!   nothing in `[lo, hi]` — [`learn_empty`](Arf::learn_empty) **splits** the offending leaves until
//!   `[lo, hi]` aligns to leaf boundaries, then marks those leaves empty, so the same mistake is never
//!   repeated. Splitting always halves a node's range (dyadic), so no range delimiters are stored.
//! * **De-escalation (budget).** When the trie outgrows its leaf budget it **merges** sibling leaves,
//!   guided by a clock/usage policy (least-recently-queried *empty* leaves are evicted first); a merged
//!   leaf is occupied if *either* child was, which keeps the no-false-negative guarantee while trading
//!   away some precision.
//!
//! A freshly built ARF is a single occupied leaf (everything "maybe"), and it is always safe to use
//! pay-as-you-go. [`insert`](Arf::insert) marks a key's leaf occupied so newly added keys never cause a
//! false negative.

use crate::common::{Result, SketchError};

/// A leaf payload: whether its range may contain a key, and whether a recent query touched it.
#[derive(Debug, Clone)]
struct Leaf {
    occupied: bool,
    used: bool,
}

/// A node of the ARF trie. Ranges are implicit (each split halves the parent's range).
#[derive(Debug, Clone)]
enum Node {
    Leaf(Leaf),
    Internal(Box<Node>, Box<Node>),
}

/// An Adaptive Range Filter over keys in `[0, 2^domain_bits)`.
///
/// # Example
/// ```
/// use sketch_oxide::range_filters::Arf;
///
/// // Domain [0, 256), budget of 64 leaves.
/// let mut arf = Arf::new(8, 64).unwrap();
/// arf.insert(100);
///
/// // A range around the key is positive; an empty range starts as a false positive...
/// assert!(arf.query(98, 102));
/// assert!(arf.query(40, 50));      // false positive (nothing there yet, but ARF can't know)
/// arf.learn_empty(40, 50);          // the data confirmed [40,50] is empty
/// assert!(!arf.query(40, 50));     // ...and is now correctly rejected
/// assert!(arf.query(98, 102));     // the real key is still found (no false negatives)
/// ```
#[derive(Debug, Clone)]
pub struct Arf {
    max_leaves: usize,
    domain_hi: u64,
    root: Node,
}

impl Arf {
    /// Creates an ARF over `[0, 2^domain_bits)` with a budget of `max_leaves` leaves.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `domain_bits` is not in `1..=32` or `max_leaves == 0`.
    pub fn new(domain_bits: u32, max_leaves: usize) -> Result<Self> {
        if !(1..=32).contains(&domain_bits) {
            return Err(SketchError::InvalidParameter {
                param: "domain_bits".to_string(),
                value: domain_bits.to_string(),
                constraint: "must be in 1..=32".to_string(),
            });
        }
        if max_leaves == 0 {
            return Err(SketchError::InvalidParameter {
                param: "max_leaves".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        Ok(Self {
            max_leaves,
            domain_hi: (1u64 << domain_bits) - 1,
            root: Node::Leaf(Leaf {
                occupied: true,
                used: false,
            }),
        })
    }

    /// Returns `true` if some key in `[lo, hi]` may be present, `false` if certainly none is.
    /// `[lo, hi]` is clamped to the domain. Touched empty leaves are marked recently-used.
    pub fn query(&mut self, lo: u64, hi: u64) -> bool {
        let (lo, hi) = (lo.min(self.domain_hi), hi.min(self.domain_hi));
        if lo > hi {
            return false;
        }
        Self::query_node(&mut self.root, 0, self.domain_hi, lo, hi)
    }

    fn query_node(node: &mut Node, nlo: u64, nhi: u64, lo: u64, hi: u64) -> bool {
        if hi < nlo || nhi < lo {
            return false; // disjoint
        }
        match node {
            Node::Leaf(l) => {
                if !l.occupied {
                    l.used = true; // an empty leaf that a query touched is worth protecting
                }
                l.occupied
            }
            Node::Internal(left, right) => {
                let mid = nlo + (nhi - nlo) / 2;
                // Evaluate both sides (no short-circuit) so usage bits are updated everywhere.
                let a = Self::query_node(left, nlo, mid, lo, hi);
                let b = Self::query_node(right, mid + 1, nhi, lo, hi);
                a || b
            }
        }
    }

    /// Marks the key's leaf occupied so an inserted key never causes a false negative.
    pub fn insert(&mut self, key: u64) {
        if key > self.domain_hi {
            return;
        }
        Self::set_occupied(&mut self.root, 0, self.domain_hi, key);
    }

    fn set_occupied(node: &mut Node, nlo: u64, nhi: u64, key: u64) {
        match node {
            Node::Leaf(l) => l.occupied = true,
            Node::Internal(left, right) => {
                let mid = nlo + (nhi - nlo) / 2;
                if key <= mid {
                    Self::set_occupied(left, nlo, mid, key);
                } else {
                    Self::set_occupied(right, mid + 1, nhi, key);
                }
            }
        }
    }

    /// Teaches the filter that `[lo, hi]` is confirmed empty (call after a false positive): escalates to
    /// represent the gap exactly, then de-escalates back within the leaf budget.
    pub fn learn_empty(&mut self, lo: u64, hi: u64) {
        let (lo, hi) = (lo.min(self.domain_hi), hi.min(self.domain_hi));
        if lo > hi {
            return;
        }
        Self::mark_empty(&mut self.root, 0, self.domain_hi, lo, hi);
        Self::collapse_redundant(&mut self.root);
        while self.leaf_count() > self.max_leaves {
            match Self::min_merge_cost(&self.root) {
                Some(c) => {
                    if !Self::merge_first_with_cost(&mut self.root, c) {
                        break;
                    }
                }
                None => break,
            }
        }
    }

    /// Splits down until `[lo, hi]` aligns to leaf boundaries and marks the contained leaves empty.
    fn mark_empty(node: &mut Node, nlo: u64, nhi: u64, lo: u64, hi: u64) {
        if hi < nlo || nhi < lo {
            return; // disjoint
        }
        if lo <= nlo && nhi <= hi {
            // This whole range is confirmed empty: collapse to a single empty leaf.
            *node = Node::Leaf(Leaf {
                occupied: false,
                used: false,
            });
            return;
        }
        // Partial overlap: an occupied leaf must split; an already-empty leaf needs no change.
        if let Node::Leaf(l) = node {
            if !l.occupied {
                return;
            }
            *node = Node::Internal(
                Box::new(Node::Leaf(Leaf {
                    occupied: true,
                    used: false,
                })),
                Box::new(Node::Leaf(Leaf {
                    occupied: true,
                    used: false,
                })),
            );
        }
        if let Node::Internal(left, right) = node {
            let mid = nlo + (nhi - nlo) / 2;
            Self::mark_empty(left, nlo, mid, lo, hi);
            Self::mark_empty(right, mid + 1, nhi, lo, hi);
        }
    }

    /// Losslessly collapses any `Internal(Leaf a, Leaf b)` whose children agree on `occupied` (two
    /// same-valued siblings carry no information).
    fn collapse_redundant(node: &mut Node) {
        if let Node::Internal(left, right) = node {
            Self::collapse_redundant(left);
            Self::collapse_redundant(right);
            if let (Node::Leaf(a), Node::Leaf(b)) = (left.as_ref(), right.as_ref()) {
                if a.occupied == b.occupied {
                    let occupied = a.occupied;
                    let used = a.used || b.used;
                    *node = Node::Leaf(Leaf { occupied, used });
                }
            }
        }
    }

    fn leaf_count(&self) -> usize {
        fn count(n: &Node) -> usize {
            match n {
                Node::Leaf(_) => 1,
                Node::Internal(l, r) => count(l) + count(r),
            }
        }
        count(&self.root)
    }

    /// Merge cost of a mergeable `Internal(Leaf, Leaf)` node: how many of its *empty* children were
    /// recently used (used empty leaves carry protected information). Lower = better eviction victim.
    fn merge_cost(a: &Leaf, b: &Leaf) -> u32 {
        let c = |l: &Leaf| if !l.occupied && l.used { 1 } else { 0 };
        c(a) + c(b)
    }

    /// The minimum merge cost over all mergeable nodes in the subtree, if any.
    fn min_merge_cost(node: &Node) -> Option<u32> {
        match node {
            Node::Leaf(_) => None,
            Node::Internal(left, right) => {
                let mut best = None;
                if let (Node::Leaf(a), Node::Leaf(b)) = (left.as_ref(), right.as_ref()) {
                    best = Some(Self::merge_cost(a, b));
                }
                for child in [left.as_ref(), right.as_ref()] {
                    if let Some(c) = Self::min_merge_cost(child) {
                        best = Some(best.map_or(c, |b: u32| b.min(c)));
                    }
                }
                best
            }
        }
    }

    /// Collapses the first mergeable node whose cost equals `target` (occupied = either child), trading
    /// precision for space. Returns whether a merge happened.
    fn merge_first_with_cost(node: &mut Node, target: u32) -> bool {
        if let Node::Internal(left, right) = node {
            if let (Node::Leaf(a), Node::Leaf(b)) = (left.as_ref(), right.as_ref()) {
                if Self::merge_cost(a, b) == target {
                    let occupied = a.occupied || b.occupied;
                    *node = Node::Leaf(Leaf {
                        occupied,
                        used: false,
                    });
                    return true;
                }
            }
            if Self::merge_first_with_cost(left, target) {
                return true;
            }
            return Self::merge_first_with_cost(right, target);
        }
        false
    }

    /// Current number of leaves in the trie.
    pub fn size(&self) -> usize {
        self.leaf_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(Arf::new(0, 64).is_err());
        assert!(Arf::new(33, 64).is_err());
        assert!(Arf::new(8, 0).is_err());
        assert!(Arf::new(8, 64).is_ok());
    }

    #[test]
    fn learns_to_reject_empty_range() {
        let mut arf = Arf::new(10, 256).unwrap();
        arf.insert(500);
        assert!(arf.query(500, 500));
        // An empty region is a false positive until taught.
        assert!(arf.query(100, 150));
        arf.learn_empty(100, 150);
        assert!(
            !arf.query(100, 150),
            "should reject the learned-empty range"
        );
        // The real key is still found.
        assert!(arf.query(500, 500));
    }

    #[test]
    fn no_false_negatives_after_learning() {
        let mut arf = Arf::new(12, 512).unwrap();
        let keys = [10u64, 1000, 2000, 3000, 4000];
        for &k in &keys {
            arf.insert(k);
        }
        // Teach many empty gaps — but only ranges that genuinely hold no key (lying about an
        // occupied range would, correctly, create a false negative).
        for g in 0..50u64 {
            let lo = g * 80 + 1;
            let hi = lo + 30;
            if keys.iter().any(|&k| k >= lo && k <= hi) {
                continue;
            }
            arf.learn_empty(lo, hi);
        }
        // Every inserted key must still be reported present.
        for &k in &keys {
            assert!(arf.query(k, k), "false negative for {k}");
        }
    }

    #[test]
    fn insert_after_learn_empty_restores_presence() {
        let mut arf = Arf::new(10, 128).unwrap();
        arf.learn_empty(42, 42);
        assert!(!arf.query(42, 42));
        arf.insert(42); // a key arrives in a previously-empty spot
        assert!(
            arf.query(42, 42),
            "inserted key must be found (no false negative)"
        );
    }

    #[test]
    fn respects_leaf_budget() {
        let mut arf = Arf::new(16, 32).unwrap();
        arf.insert(65000); // sits above every learned gap, so it is never (falsely) marked empty
        // Learning many fine-grained gaps would grow the trie unboundedly without de-escalation.
        for g in 0..400u64 {
            arf.learn_empty(g * 100, g * 100 + 10);
        }
        assert!(arf.size() <= 32, "leaf budget exceeded: {}", arf.size());
        // Budget pressure must not introduce false negatives.
        assert!(arf.query(65000, 65000));
    }

    #[test]
    fn full_empty_range_collapses() {
        let mut arf = Arf::new(8, 64).unwrap();
        // Learn the entire domain empty (no keys) — collapses to a single empty leaf.
        arf.learn_empty(0, 255);
        assert_eq!(arf.size(), 1);
        assert!(!arf.query(0, 255));
        assert!(!arf.query(100, 100));
    }
}
