//! PinSketch — BCH-based set reconciliation with optimal sketch size.
//!
//! PinSketch (Eppstein, Goodrich, Uyeda & Varghese, "What's the Difference? Efficient Set
//! Reconciliation without Prior Context", SIGCOMM 2011 — the algorithm behind Bitcoin's
//! `minisketch` / Erlay [BIP-330]) summarizes a set of distinct field elements in `capacity`
//! field elements of space and recovers the **symmetric difference** of two sets *exactly* as
//! long as it has no more than `capacity` elements — independent of how large the sets
//! themselves are.
//!
//! # How it works
//!
//! Treat each set member as a nonzero element of `GF(2^field_bits)`. The sketch stores the odd
//! **power sums** (syndromes) of its members: `s_k = Σ x^(2k-1)` for `k = 1..=capacity`. In a
//! field of characteristic two, addition is XOR, so:
//!
//! - inserting an element twice **cancels** it (XOR), giving set semantics;
//! - the syndromes of a *symmetric difference* `A △ B` are just the XOR of the two sketches
//!   ([`merge`](PinSketch::merge)) — this is the whole trick: the shared elements cancel and
//!   only `A △ B` survives.
//!
//! To recover the elements from the syndromes, the even power sums are reconstructed via the
//! characteristic-two identity `p_{2k} = (p_k)^2`, [Berlekamp–Massey](https://en.wikipedia.org/wiki/Berlekamp%E2%80%93Massey_algorithm)
//! finds the error-locator polynomial whose degree is the size of the difference, and a Chien
//! search factors it — each root's inverse is a recovered element.
//!
//! # Guarantees
//!
//! - **Exact** recovery when `|A △ B| ≤ capacity`.
//! - Sketch size is exactly `capacity` field elements — provably optimal for set reconciliation.
//!
//! When the difference *exceeds* `capacity` the sketch is saturated: [`decode`](PinSketch::decode)
//! usually returns a [`ReconciliationError`](SketchError::ReconciliationError) (the locator fails
//! to factor over the field), but — as with any BCH code beyond its designed distance —
//! detection is **not** guaranteed: with only `capacity` syndromes a wrong set of `≤ capacity`
//! elements can satisfy them, so decode may also return an incorrect set. Size `capacity` ahead
//! of the expected difference, estimating it first with a difference estimator such as
//! [`StrataEstimator`](crate::reconciliation::StrataEstimator) when it is unknown.
//!
//! # Cost note — choosing `field_bits`
//!
//! Root finding here is a Chien search: [`decode`](PinSketch::decode) scans the whole field, so
//! it costs `O(2^field_bits)` field evaluations. This is fast for `field_bits ≤ 20` (≤ ~1M) and
//! quickly becomes intractable beyond that (a 32-bit field is 4·10⁹ evaluations per decode).
//! **Keep `field_bits ≤ 20`** — hash wide keys down into that range before inserting. Lifting
//! this limit means replacing the Chien step with `minisketch`'s Berlekamp-trace
//! factorization (which works directly in large fields); that is a self-contained follow-up to
//! [`decode`](PinSketch::decode) and does not change the sketch format.

use crate::common::{Result, SketchError};

/// A PinSketch over `GF(2^field_bits)` recovering up to `capacity` differing elements.
///
/// # Example
/// ```
/// use sketch_oxide::reconciliation::PinSketch;
///
/// // Alice and Bob share most elements; their sets differ in a few.
/// let mut alice = PinSketch::new(16, 8).unwrap();
/// for x in [10u64, 20, 30, 40, 50] { alice.insert(x).unwrap(); }
///
/// let mut bob = PinSketch::new(16, 8).unwrap();
/// for x in [10u64, 20, 30, 99, 123] { bob.insert(x).unwrap(); }
///
/// // XOR the sketches → a sketch of the symmetric difference {40, 50, 99, 123}.
/// alice.merge(&bob).unwrap();
/// let mut diff = alice.decode().unwrap();
/// diff.sort_unstable();
/// assert_eq!(diff, vec![40, 50, 99, 123]);
/// ```
#[derive(Debug, Clone)]
pub struct PinSketch {
    field_bits: u32,
    /// Irreducible reduction polynomial, *including* its `x^field_bits` term.
    poly: u64,
    capacity: usize,
    /// Odd power sums: `syndromes[k] = Σ x^(2k+1)` for `k = 0..capacity`.
    syndromes: Vec<u64>,
}

impl PinSketch {
    /// Creates a PinSketch over `GF(2^field_bits)` that can recover up to `capacity` differing
    /// elements. An irreducible polynomial for the field is found automatically (Rabin's test),
    /// so any `field_bits` in `2..=32` yields a valid field.
    ///
    /// Pick `field_bits` so every set member fits in `field_bits` bits, and `capacity` for the
    /// largest symmetric difference you expect to recover.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `field_bits` is outside `2..=32` or `capacity` is 0.
    pub fn new(field_bits: u32, capacity: usize) -> Result<Self> {
        if !(2..=32).contains(&field_bits) {
            return Err(SketchError::InvalidParameter {
                param: "field_bits".to_string(),
                value: field_bits.to_string(),
                constraint: "must be in 2..=32".to_string(),
            });
        }
        if capacity == 0 {
            return Err(SketchError::InvalidParameter {
                param: "capacity".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            field_bits,
            poly: find_irreducible(field_bits),
            capacity,
            syndromes: vec![0u64; capacity],
        })
    }

    /// Inserts an element (toggles its membership — inserting the same element twice removes it,
    /// matching the symmetric-difference semantics of set reconciliation).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `element` is 0 or does not fit in `field_bits` bits
    /// (0 has no recoverable locator).
    pub fn insert(&mut self, element: u64) -> Result<()> {
        if element == 0 || (self.field_bits < 64 && element >> self.field_bits != 0) {
            return Err(SketchError::InvalidParameter {
                param: "element".to_string(),
                value: element.to_string(),
                constraint: format!("must be in 1..2^{}", self.field_bits),
            });
        }
        // Add x^1, x^3, x^5, ... by stepping with x^2.
        let x2 = self.gf_mul(element, element);
        let mut pw = element;
        for k in 0..self.capacity {
            self.syndromes[k] ^= pw;
            pw = self.gf_mul(pw, x2);
        }
        Ok(())
    }

    /// Removes an element (identical to [`insert`](PinSketch::insert) given XOR semantics).
    ///
    /// # Errors
    /// Same as [`insert`](PinSketch::insert).
    pub fn remove(&mut self, element: u64) -> Result<()> {
        self.insert(element)
    }

    /// Merges another sketch by XOR, producing a sketch of the symmetric difference of the two
    /// underlying sets. Both must share `field_bits` and `capacity`.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the configurations differ.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.field_bits != other.field_bits || self.capacity != other.capacity {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "config mismatch: GF(2^{})×{} vs GF(2^{})×{}",
                    self.field_bits, self.capacity, other.field_bits, other.capacity
                ),
            });
        }
        for (a, b) in self.syndromes.iter_mut().zip(&other.syndromes) {
            *a ^= *b;
        }
        Ok(())
    }

    /// Recovers the set of elements summarized by this sketch (after a [`merge`](PinSketch::merge),
    /// the symmetric difference of the two sets).
    ///
    /// # Errors
    /// [`SketchError::ReconciliationError`] if the difference exceeds `capacity` and the locator
    /// polynomial does not fully factor over the field — the sketch is saturated and a larger
    /// `capacity` is needed. (Over-capacity is not always detectable; see the type-level docs.)
    pub fn decode(&self) -> Result<Vec<u64>> {
        // Reconstruct power sums p_1..p_{2·capacity}. p[j] is the (j+1)-th power sum.
        let n = 2 * self.capacity;
        let mut p = vec![0u64; n];
        for i in 0..self.capacity {
            p[2 * i] = self.syndromes[i]; // odd power sum p_{2i+1}
        }
        for k in 1..=self.capacity {
            // even power sum p_{2k} = (p_k)^2
            p[2 * k - 1] = self.gf_mul(p[k - 1], p[k - 1]);
        }

        let locator = self.berlekamp_massey(&p);
        let degree = locator.len() - 1;
        if degree > self.capacity {
            return Err(SketchError::ReconciliationError {
                reason: format!(
                    "symmetric difference exceeds capacity {} (locator degree {degree})",
                    self.capacity
                ),
            });
        }

        // Chien search: roots α of the locator give elements α^{-1}.
        let mut elements = Vec::with_capacity(degree);
        let field_size = 1u64 << self.field_bits;
        for alpha in 1..field_size {
            if self.poly_eval(&locator, alpha) == 0 {
                elements.push(self.gf_inv(alpha));
            }
        }
        if elements.len() != degree {
            return Err(SketchError::ReconciliationError {
                reason: format!(
                    "locator did not factor over the field ({} of {degree} roots found); \
                     difference likely exceeds capacity",
                    elements.len()
                ),
            });
        }
        Ok(elements)
    }

    /// Number of elements the sketch can recover.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Field bit width.
    #[inline]
    pub fn field_bits(&self) -> u32 {
        self.field_bits
    }

    // --- GF(2^field_bits) arithmetic ---------------------------------------------------------

    /// Carryless (Russian-peasant) multiply in the field.
    fn gf_mul(&self, mut a: u64, mut b: u64) -> u64 {
        let mut result = 0u64;
        let overflow = 1u64 << self.field_bits;
        while b != 0 {
            if b & 1 != 0 {
                result ^= a;
            }
            b >>= 1;
            a <<= 1;
            if a & overflow != 0 {
                a ^= self.poly; // poly includes the x^field_bits term, clearing the overflow bit
            }
        }
        result
    }

    /// `a^e` by square-and-multiply.
    fn gf_pow(&self, mut a: u64, mut e: u64) -> u64 {
        let mut result = 1u64;
        while e != 0 {
            if e & 1 != 0 {
                result = self.gf_mul(result, a);
            }
            a = self.gf_mul(a, a);
            e >>= 1;
        }
        result
    }

    /// Multiplicative inverse via Fermat: `a^(2^field_bits − 2)`.
    fn gf_inv(&self, a: u64) -> u64 {
        self.gf_pow(a, (1u64 << self.field_bits) - 2)
    }

    /// Evaluates a polynomial (low-order coefficient first) at `x` via Horner's rule.
    fn poly_eval(&self, coeffs: &[u64], x: u64) -> u64 {
        let mut acc = 0u64;
        for &c in coeffs.iter().rev() {
            acc = self.gf_mul(acc, x) ^ c;
        }
        acc
    }

    /// Berlekamp–Massey over the field: the shortest LFSR (error-locator polynomial, low-order
    /// coefficient first) generating the power-sum sequence `s`.
    fn berlekamp_massey(&self, s: &[u64]) -> Vec<u64> {
        let mut cur = vec![1u64]; // current connection polynomial C(x)
        let mut prev = vec![1u64]; // last C(x) before the most recent length change
        let mut l = 0usize; // current LFSR length
        let mut m = 1usize; // steps since the last length change
        let mut b = 1u64; // discrepancy at that last change

        for i in 0..s.len() {
            // Discrepancy d = s_i + Σ_{j=1..l} C_j · s_{i−j}.
            let mut d = s[i];
            for j in 1..=l {
                d ^= self.gf_mul(cur[j], s[i - j]);
            }
            if d == 0 {
                m += 1;
            } else {
                let coef = self.gf_mul(d, self.gf_inv(b));
                if 2 * l <= i {
                    let t = cur.clone();
                    self.poly_xor_scaled_shift(&mut cur, &prev, coef, m);
                    l = i + 1 - l;
                    prev = t;
                    b = d;
                    m = 1;
                } else {
                    self.poly_xor_scaled_shift(&mut cur, &prev, coef, m);
                    m += 1;
                }
            }
        }
        cur
    }

    /// `cur[j + shift] ^= coef · src[j]` for all `j` (in-place, growing `cur` as needed).
    fn poly_xor_scaled_shift(&self, cur: &mut Vec<u64>, src: &[u64], coef: u64, shift: usize) {
        if cur.len() < src.len() + shift {
            cur.resize(src.len() + shift, 0);
        }
        for (j, &sc) in src.iter().enumerate() {
            cur[j + shift] ^= self.gf_mul(coef, sc);
        }
    }
}

// --- field construction: smallest irreducible polynomial of the given degree -----------------

/// Finds the smallest-weight irreducible polynomial of degree `n` over GF(2), returned with its
/// `x^n` term included. Guaranteed to terminate: irreducible polynomials of every degree exist.
fn find_irreducible(n: u32) -> u64 {
    let high = 1u64 << n;
    // The constant term must be 1 (else x divides it), so try odd low parts.
    let mut low = 1u64;
    loop {
        let poly = high | low;
        if is_irreducible(poly, n) {
            return poly;
        }
        low += 2;
    }
}

/// Rabin's irreducibility test for a degree-`n` polynomial over GF(2).
fn is_irreducible(poly: u64, n: u32) -> bool {
    // f is irreducible iff x^(2^n) ≡ x (mod f) and gcd(x^(2^(n/p)) − x, f) = 1 for every prime
    // p | n.
    let x = 2u64; // the polynomial "x"
    if poly_pow2k_x(poly, n, n) != x {
        return false;
    }
    for p in distinct_prime_factors(n) {
        let r = poly_pow2k_x(poly, n / p, n);
        if poly_gcd(r ^ x, poly) != 1 {
            return false;
        }
    }
    true
}

/// Computes `x^(2^k) mod f` by `k` repeated squarings starting from `x`.
fn poly_pow2k_x(poly: u64, k: u32, n: u32) -> u64 {
    let mut r = 2u64; // x
    for _ in 0..k {
        r = poly_mulmod(r, r, poly, n);
    }
    r
}

/// `(a · b) mod f` in GF(2)[x], where `f` (degree `n`) includes its `x^n` term.
fn poly_mulmod(a: u64, b: u64, poly: u64, n: u32) -> u64 {
    // Carryless multiply into a wide accumulator, then reduce.
    let a = a as u128;
    let mut prod = 0u128;
    for i in 0..64 {
        if (b >> i) & 1 != 0 {
            prod ^= a << i;
        }
    }
    let f = poly as u128;
    while prod != 0 {
        let hb = 127 - prod.leading_zeros();
        if hb < n {
            break;
        }
        prod ^= f << (hb - n);
    }
    prod as u64
}

/// GCD of two GF(2)[x] polynomials (Euclid). Returns the monic gcd as a bitmask.
fn poly_gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        a = poly_rem(a, b);
        std::mem::swap(&mut a, &mut b);
    }
    a
}

/// Remainder of `a` divided by `b` in GF(2)[x].
fn poly_rem(mut a: u64, b: u64) -> u64 {
    if b == 0 {
        return a;
    }
    let db = 63 - b.leading_zeros();
    while a != 0 {
        let da = 63 - a.leading_zeros();
        if da < db {
            break;
        }
        a ^= b << (da - db);
    }
    a
}

/// Distinct prime factors of `n` (n ≤ 32, trial division).
fn distinct_prime_factors(mut n: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut d = 2;
    while d * d <= n {
        if n.is_multiple_of(d) {
            out.push(d);
            while n.is_multiple_of(d) {
                n /= d;
            }
        }
        d += 1;
    }
    if n > 1 {
        out.push(n);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(PinSketch::new(1, 8).is_err());
        assert!(PinSketch::new(33, 8).is_err());
        assert!(PinSketch::new(16, 0).is_err());
        assert!(PinSketch::new(16, 8).is_ok());
    }

    #[test]
    fn rejects_bad_elements() {
        let mut s = PinSketch::new(8, 4).unwrap();
        assert!(s.insert(0).is_err()); // zero has no locator
        assert!(s.insert(256).is_err()); // does not fit in 8 bits
        assert!(s.insert(255).is_ok());
    }

    /// The field built for every bit width must be a real field: every nonzero element has a
    /// multiplicative inverse. This is the definitive check that `find_irreducible` works.
    #[test]
    fn field_is_valid_for_every_width() {
        for fb in [4u32, 6, 8, 10, 12] {
            let s = PinSketch::new(fb, 1).unwrap();
            for a in 1..(1u64 << fb) {
                assert_eq!(
                    s.gf_mul(a, s.gf_inv(a)),
                    1,
                    "no inverse for {a} in GF(2^{fb})"
                );
            }
        }
    }

    #[test]
    fn decodes_symmetric_difference() {
        let mut alice = PinSketch::new(16, 8).unwrap();
        for x in [10u64, 20, 30, 40, 50, 60, 70] {
            alice.insert(x).unwrap();
        }
        let mut bob = PinSketch::new(16, 8).unwrap();
        for x in [10u64, 20, 30, 99, 123, 7777, 555] {
            bob.insert(x).unwrap();
        }
        alice.merge(&bob).unwrap();
        let mut diff = alice.decode().unwrap();
        diff.sort_unstable();
        // Symmetric difference: alice-only {40,50,60,70} ∪ bob-only {99,123,555,7777}.
        assert_eq!(diff, vec![40, 50, 60, 70, 99, 123, 555, 7777]);
    }

    #[test]
    fn empty_difference_decodes_to_nothing() {
        let mut a = PinSketch::new(16, 6).unwrap();
        let mut b = PinSketch::new(16, 6).unwrap();
        for x in [1u64, 2, 3, 100, 4000] {
            a.insert(x).unwrap();
            b.insert(x).unwrap();
        }
        a.merge(&b).unwrap();
        assert!(a.decode().unwrap().is_empty());
    }

    #[test]
    fn single_set_decodes_to_itself() {
        // With no merge, decode recovers the inserted set directly (difference vs empty set).
        let mut s = PinSketch::new(18, 5).unwrap();
        for x in [3u64, 17, 290, 1000, 99999] {
            s.insert(x).unwrap();
        }
        let mut got = s.decode().unwrap();
        got.sort_unstable();
        assert_eq!(got, vec![3, 17, 290, 1000, 99999]);
    }

    #[test]
    fn double_insert_cancels() {
        let mut s = PinSketch::new(16, 4).unwrap();
        s.insert(42).unwrap();
        s.insert(7).unwrap();
        s.insert(42).unwrap(); // cancels the first 42
        let got = s.decode().unwrap();
        assert_eq!(got, vec![7]);
    }

    #[test]
    fn saturation_does_not_yield_correct_full_set() {
        // Capacity 3 but 5 differing elements: the sketch is saturated. Decoding beyond a BCH
        // code's designed distance cannot be relied on — decode must NOT claim to have recovered
        // the true 5-element set. It either errors (locator fails to factor) or returns some
        // other (wrong/partial) set; it must never return exactly {1,2,3,4,5}.
        let mut a = PinSketch::new(16, 3).unwrap();
        for x in [1u64, 2, 3, 4, 5] {
            a.insert(x).unwrap();
        }
        match a.decode() {
            Err(SketchError::ReconciliationError { .. }) => {} // saturation detected — ideal
            Err(other) => panic!("unexpected error kind: {other}"),
            Ok(mut set) => {
                set.sort_unstable();
                assert_ne!(
                    set,
                    vec![1, 2, 3, 4, 5],
                    "must not claim correct over-capacity decode"
                );
            }
        }
    }

    #[test]
    fn merge_mismatch_errors() {
        let mut a = PinSketch::new(16, 8).unwrap();
        let b = PinSketch::new(16, 4).unwrap();
        assert!(a.merge(&b).is_err());
        let c = PinSketch::new(20, 8).unwrap();
        assert!(a.merge(&c).is_err());
    }
}
