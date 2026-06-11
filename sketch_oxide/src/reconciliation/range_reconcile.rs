//! Range-Based Set Reconciliation (RBSR / negentropy).
//!
//! Range-Based Set Reconciliation (Meyer, "Range-Based Set Reconciliation", 2023; the basis of
//! Nostr's *negentropy* protocol) reconciles two sorted sets by comparing **range fingerprints**.
//! Each side can summarize any contiguous range of its sorted keys with a small fingerprint (an XOR
//! of per-key hashes) and a count. To reconcile, the two sides compare the fingerprint of the whole
//! range: if they match, the range is already synchronized and is skipped entirely; if they differ
//! and the range is small, the few items are exchanged directly; otherwise the range is split and
//! each half reconciled recursively. Synchronized regions therefore cost almost nothing — work is
//! spent only where the sets actually differ, in `O(d·log n)` communication for a difference of
//! size `d`.
//!
//! Unlike an IBLT this needs no special decoding and no pre-agreed difference bound; it degrades
//! gracefully from tiny to large differences.

/// The symmetric difference produced by [`RangeReconciler::reconcile`], as sorted `u64` keys.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RangeDiff {
    /// Keys present remotely but not locally (the local side should insert these).
    pub to_insert: Vec<u64>,
    /// Keys present locally but not remotely (the local side has these extra).
    pub to_remove: Vec<u64>,
}

/// A reconcilable view over a sorted, de-duplicated `u64` key set.
///
/// # Example
/// ```
/// use sketch_oxide::reconciliation::RangeReconciler;
///
/// let alice = RangeReconciler::new(vec![1, 2, 3, 4, 5, 100, 200]);
/// let bob = RangeReconciler::new(vec![1, 2, 3, 4, 5, 300, 400]);
///
/// let diff = alice.reconcile(&bob, 8);
/// // Alice has {100, 200} that Bob lacks; Bob has {300, 400} that Alice lacks.
/// assert_eq!(diff.to_remove, vec![100, 200]); // local-only
/// assert_eq!(diff.to_insert, vec![300, 400]); // remote-only
/// ```
#[derive(Debug, Clone)]
pub struct RangeReconciler {
    keys: Vec<u64>,
}

impl RangeReconciler {
    /// Builds a reconciler over `keys` (sorted and de-duplicated internally).
    pub fn new(mut keys: Vec<u64>) -> Self {
        keys.sort_unstable();
        keys.dedup();
        Self { keys }
    }

    /// The sorted keys.
    #[inline]
    pub fn keys(&self) -> &[u64] {
        &self.keys
    }

    /// SplitMix64 finalizer — a strong per-key hash for fingerprints.
    #[inline]
    fn mix(mut z: u64) -> u64 {
        z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Fingerprint (XOR of key hashes) of a slice of keys.
    #[inline]
    fn fingerprint(slice: &[u64]) -> u64 {
        slice.iter().fold(0u64, |acc, &k| acc ^ Self::mix(k))
    }

    /// Reconciles against `other`, returning the symmetric difference. `split_threshold` is the
    /// range size at or below which items are compared directly instead of split further.
    pub fn reconcile(&self, other: &Self, split_threshold: usize) -> RangeDiff {
        let mut to_remove = Vec::new(); // local-only (in self, not other)
        let mut to_insert = Vec::new(); // remote-only (in other, not self)
        Self::recurse(
            &self.keys,
            &other.keys,
            split_threshold.max(1),
            &mut to_remove,
            &mut to_insert,
        );
        to_remove.sort_unstable();
        to_insert.sort_unstable();
        RangeDiff {
            to_insert,
            to_remove,
        }
    }

    fn recurse(
        a: &[u64],
        b: &[u64],
        threshold: usize,
        to_remove: &mut Vec<u64>,
        to_insert: &mut Vec<u64>,
    ) {
        // Identical range: same count and fingerprint ⇒ skip (the whole point of RBSR).
        if a.len() == b.len() && Self::fingerprint(a) == Self::fingerprint(b) {
            return;
        }
        // Small enough: compare directly.
        if a.len() + b.len() <= threshold {
            Self::direct_diff(a, b, to_remove, to_insert);
            return;
        }
        // Split the value range in half and recurse on each side.
        let lo = *a
            .first()
            .unwrap_or(&u64::MAX)
            .min(b.first().unwrap_or(&u64::MAX));
        let hi = *a.last().unwrap_or(&0).max(b.last().unwrap_or(&0));
        if hi.saturating_sub(lo) <= 1 {
            // Cannot split the value range further; compare directly.
            Self::direct_diff(a, b, to_remove, to_insert);
            return;
        }
        let mid = lo + (hi - lo) / 2;
        let ai = a.partition_point(|&k| k <= mid);
        let bi = b.partition_point(|&k| k <= mid);
        Self::recurse(&a[..ai], &b[..bi], threshold, to_remove, to_insert);
        Self::recurse(&a[ai..], &b[bi..], threshold, to_remove, to_insert);
    }

    /// Direct sorted-merge difference of two key ranges.
    fn direct_diff(a: &[u64], b: &[u64], to_remove: &mut Vec<u64>, to_insert: &mut Vec<u64>) {
        let (mut i, mut j) = (0, 0);
        while i < a.len() && j < b.len() {
            match a[i].cmp(&b[j]) {
                std::cmp::Ordering::Less => {
                    to_remove.push(a[i]);
                    i += 1;
                }
                std::cmp::Ordering::Greater => {
                    to_insert.push(b[j]);
                    j += 1;
                }
                std::cmp::Ordering::Equal => {
                    i += 1;
                    j += 1;
                }
            }
        }
        to_remove.extend_from_slice(&a[i..]);
        to_insert.extend_from_slice(&b[j..]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diff_keys(a: Vec<u64>, b: Vec<u64>, t: usize) -> (Vec<u64>, Vec<u64>) {
        let d = RangeReconciler::new(a).reconcile(&RangeReconciler::new(b), t);
        (d.to_remove, d.to_insert)
    }

    /// Reference symmetric difference for cross-checking.
    fn truth(a: &[u64], b: &[u64]) -> (Vec<u64>, Vec<u64>) {
        use std::collections::BTreeSet;
        let sa: BTreeSet<u64> = a.iter().copied().collect();
        let sb: BTreeSet<u64> = b.iter().copied().collect();
        (
            sa.difference(&sb).copied().collect(),
            sb.difference(&sa).copied().collect(),
        )
    }

    #[test]
    fn identical_sets_have_no_difference() {
        let (rm, ins) = diff_keys((0..1000).collect(), (0..1000).collect(), 16);
        assert!(rm.is_empty() && ins.is_empty());
    }

    #[test]
    fn half_overlap() {
        let a: Vec<u64> = (0..1000).collect();
        let b: Vec<u64> = (500..1500).collect();
        let (rm, ins) = diff_keys(a.clone(), b.clone(), 16);
        let (t_rm, t_ins) = truth(&a, &b);
        assert_eq!(rm, t_rm);
        assert_eq!(ins, t_ins);
    }

    #[test]
    fn sparse_differences_in_large_sets() {
        let a: Vec<u64> = (0..100_000).collect();
        let mut b = a.clone();
        // Mutate a few entries: remove some, add some far away.
        b.retain(|&k| k != 42 && k != 9999 && k != 73_000);
        b.extend([1_000_000, 2_000_000, 3_000_000]);
        let (rm, ins) = diff_keys(a.clone(), b.clone(), 16);
        let (t_rm, t_ins) = truth(&a, &b);
        assert_eq!(rm, t_rm);
        assert_eq!(ins, t_ins);
    }

    #[test]
    fn disjoint_sets() {
        let a: Vec<u64> = (0..50).collect();
        let b: Vec<u64> = (1000..1050).collect();
        let (rm, ins) = diff_keys(a.clone(), b.clone(), 8);
        assert_eq!(rm, a);
        assert_eq!(ins, b);
    }

    #[test]
    fn one_side_empty() {
        let a: Vec<u64> = (0..100).collect();
        let (rm, ins) = diff_keys(a.clone(), vec![], 8);
        assert_eq!(rm, a);
        assert!(ins.is_empty());
        let (rm2, ins2) = diff_keys(vec![], a.clone(), 8);
        assert!(rm2.is_empty());
        assert_eq!(ins2, a);
    }

    #[test]
    fn deduplicates_input() {
        let r = RangeReconciler::new(vec![5, 5, 1, 1, 3]);
        assert_eq!(r.keys(), &[1, 3, 5]);
    }

    #[test]
    fn small_threshold_still_correct() {
        // Even with the minimum threshold the recursion must reach the exact difference.
        let a: Vec<u64> = (0..2000).map(|i| i * 3).collect();
        let b: Vec<u64> = (0..2000).map(|i| i * 3 + 1).collect();
        let (rm, ins) = diff_keys(a.clone(), b.clone(), 1);
        let (t_rm, t_ins) = truth(&a, &b);
        assert_eq!(rm, t_rm);
        assert_eq!(ins, t_ins);
    }
}
