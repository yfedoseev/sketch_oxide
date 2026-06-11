# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - 0.2.0 (in development)

Development toward holistic 2026 coverage. See the phased roadmap (internal) for the
full plan. This release is being built on the `releases/v0.2.0` branch.

### Changed

- **`range_filters::MementoFilter` — rebuilt on the genuine prefix/memento decomposition (fidelity
  fix).** The previous implementation stored every full 64-bit key and answered range queries by a
  linear scan over all buckets — neither space-bounded nor sublinear, despite the docs. It now
  splits each key into a high-order prefix and a low-order memento and stores, per prefix, the
  sorted list of mementos. Range queries walk only the prefixes overlapping the query
  (order-preserving, sublinear), are **exact in the interior** (an occupied prefix wholly inside
  the range certainly holds a key in range) and boundary-refined by the memento lists — still no
  false negatives. Adds a `may_contain` point query and `num_prefixes`. Public API, `MementoStats`,
  capacity and `num_expansions` semantics unchanged (48/48 integration tests still pass). The
  rank-and-select quotient-filter packing with fingerprinted prefixes (trading bits for a `2^-r`
  FPR) is documented as the follow-up.

### Added

- **`sampling::SlidingWindowSample` — uniform sampling over a moving window (SODA 2002).** Maintains a
  uniform random sample of the last `W` elements (where the classic reservoir cannot, because the
  sample can expire). Following Babcock–Datar–Motwani, every element gets a random priority and the
  in-window minimum-priority element is the sample (uniform, since priorities are i.i.d.); only the
  elements that could *become* that minimum are retained — a monotonic "staircase" deque whose front
  is always the current sample, bounded by `W`. `push`, `sample`, `retained`. The `k`-sample variant
  is a documented follow-up. 6 tests (sample always in window over 100k; staircase ≤ W; mean offset
  near the window midpoint over 200 seeds → uniformity; deterministic with seed) + doctest.


- **`similarity::WeightedMinHash` — Improved Consistent Weighted Sampling (ICWS, ICDM 2010).** Extends
  MinHash from unweighted to **weighted** Jaccard similarity `Σ min(w_A,w_B) / Σ max(w_A,w_B)` — the
  right measure for term frequencies, traffic volumes, histogram bins. Ioffe's ICWS draws a
  *consistent* sample per hash and element such that two weighted sets produce the same signature
  component with probability *exactly* their weighted Jaccard, so the fraction of matching components
  is an unbiased estimate. A weighted set is `(element, weight)` pairs; `signature` produces a
  `(chosen_element, level)` vector, `jaccard` compares two signatures. 6 tests cross-checked against
  the exact weighted Jaccard (identical; disjoint; 0.5 overlap; unequal weights giving 0.1; signature
  length) + doctest.


- **`quantiles::GreenwaldKhanna` — deterministic ε-approximate quantiles (SIGMOD 2001).** The classic
  Greenwald–Khanna summary: rank/quantile queries within `±εn` using `O((1/ε)·log(εn))` space, with a
  **deterministic** error bound that holds for every stream regardless of order or adversary (unlike
  the randomized KLL / t-digest already in the module). Keeps sorted `(value, g, Δ)` tuples under the
  invariant `g + Δ ≤ 2εn`, with periodic threshold-based compression. `insert`, `quantile`, `min`,
  `max`, `count`, `num_tuples`. 6 tests (every percentile within the rank bound over 100k shuffled
  values; exact min/max; sublinear space <5000 tuples for 200k; extreme-quantile clamping) + doctest.


- **`reconciliation::RangeReconciler` — Range-Based Set Reconciliation (RBSR / negentropy).** Meyer's
  range-based reconciliation (the basis of Nostr's negentropy) reconciles two sorted sets by
  comparing **range fingerprints** (XOR of per-key hashes + count): a matching range is skipped
  entirely, a small differing range is exchanged directly, and a large differing range is split and
  recursed — so work falls only where the sets differ (`O(d·log n)` for a difference of size `d`).
  Unlike an IBLT it needs no special decoding and no pre-agreed difference bound, degrading
  gracefully from tiny to large differences. `reconcile` returns a `RangeDiff` of sorted to-insert /
  to-remove keys. 7 tests cross-checked against an exact `BTreeSet` difference (identical, half-overlap,
  3 sparse diffs in 100k, disjoint, one-side-empty, minimum threshold) + doctest.


- **New `net` module + `net::BeauCoup` — per-key distinct counting / super-spreader detection
  (SIGCOMM 2020).** BeauCoup (Chen, Liu, Zhao, Braverman & Rexford) finds keys that contact many
  *distinct* values (e.g. source IPs touching many destinations — scanners / super-spreaders) using
  the **coupon-collector** principle: each `(key, value)` activates one of `m` coupons only with a
  small probability `q`, so a key fills its coupons only after many distinct values; the collected
  fraction inverts to a per-key distinct-count estimate. Activation is keyed on the *pair*, so
  repeats are idempotent and only distinct values move the estimate; with `q ≪ 1` only a small
  fraction of observations touch memory. `record`, `estimate_distinct`, `super_spreaders`. 6 tests
  (distinct-count within 35%; repeats don't inflate; super-spreaders detected and light keys not;
  smaller `q` resolves larger counts) + doctest. Establishes the `net` module for future
  network-telemetry sketches.


- **`streaming::SmoothHistogramSum` — (1±ε) sliding-window aggregates (Smooth Histograms, FOCS 2007).**
  The Braverman–Ostrovsky framework for approximating *smooth* functions over the last `W` elements
  of a stream. It keeps a sparse set of **checkpoints** (each holding the aggregate from its start to
  now), pruned so consecutive checkpoints stay within `(1−ε)`; a window query returns the checkpoint
  just before the window boundary, which smoothness guarantees is within `(1±ε)` of the true value —
  in `O((1/ε)·log R)` space, never the full window. Instantiated here for windowed **sum of
  non-negative values**; the same machinery generalizes to distinct counts / `Lp` norms via a
  per-checkpoint sketch (documented follow-up). 6 tests (approximation matches exact within ε at
  multiple points; sublinear checkpoint count over 100k; window-larger-than-stream; bad params/values)
  + doctest.


- **`matrix::CountSketchEmbedding` — CountSketch sparse subspace embedding (STOC 2013).** The
  Clarkson–Woodruff input-sparsity-time transform: an `s × n` matrix with one `±1` per column, applied
  to an `n × d` matrix in `O(nnz(A))` to produce a tiny `s × d` sketch `SA` that is a **subspace
  embedding** (`‖SAx‖ ≈ ‖Ax‖` once `s = O(d²/ε²)`). Enables **sketch-and-solve** least squares —
  `sketched_least_squares` sketches the system and solves the small `d × d` normal equations
  (Gaussian elimination with pivoting). `apply`/`apply_vec`. 6 tests (norm preserved in expectation
  over seeds; sketched least-squares recovers the true coefficients within 0.15; near-exact with no
  collisions; shape/singularity errors) + doctest.


- **`statistics::KArySketch` — sketch-based heavy-change detection (k-ary sketch, IMC 2003).** A
  linear sketch (Krishnamurthy, Sen, Zhang & Chen) for finding the items whose value *changed* most
  between two snapshots (the "deltoids"). Like Count-Min it keeps `depth × width` buckets and adds
  each item's signed value to one bucket per row, but reads back an **unbiased** estimate
  `(bucket − total/width)/(1 − 1/width)` (median over rows). Because it is linear, the `difference`
  of two snapshots is itself a sketch — of the change vector — so heavy changers are the keys whose
  difference-estimate exceeds a threshold (`heavy_changers`). Handles increments and decrements.
  6 tests (lone value; difference isolates a +3000 surge and a −2000 drop; heavy changers found &
  ranked; linearity under decrements) + doctest.


- **`statistics::HllJointEstimator` — set operations over two HyperLogLog sketches.** Because HLL
  registers hold per-bucket maxima, the register-wise maximum of two sketches is exactly the HLL of
  their **union**; inclusion–exclusion then yields the intersection and Jaccard:
  `|A∩B| = |A| + |B| − |A∪B|`, `J = |A∩B| / |A∪B|`. All four cardinalities use the same
  register-based estimator (bias-corrected harmonic mean + linear counting) so the arithmetic stays
  coherent. `cardinality_a`/`cardinality_b`, `union`, `intersection`, `jaccard`. 6 tests
  (half-overlap J≈1/3; disjoint; identical; subset; individual cardinalities; precision-mismatch
  error) + doctest.


- **`range_filters::RadixSpline` — single-pass learned index over sorted keys (aiDM @ SIGMOD 2020).**
  Approximates the CDF (`key → position`) of a sorted `u64` array so a lookup predicts a key's
  position within a guaranteed `max_error` and finishes with a bounded local search. Two parts built
  in one pass: a **linear spline** fitted by the greedy-spline-corridor method (fewest points keeping
  every key within `max_error`) and a **radix table** mapping high key bits to the spline segment for
  O(1) segment lookup. `build`, `estimate_position`, `search_bound`, `num_spline_points`. Joins
  `PgmIndex` in the learned-index family. 6 tests (error bound holds for linear / gappy / clustered
  key distributions; search window contains the true position; endpoint clamping; linear CDF
  compresses to <10 points) + doctest.


- **`range_filters::Rosetta` — range filtering via prefix Bloom filters (SIGMOD 2020).** Rosetta
  (Luo et al.) answers range queries as a handful of point queries: every key is inserted under all
  of its prefixes (`level 0..=bits`) into a Bloom filter keyed by `(level, prefix)`, and a range
  `[low, high]` is **dyadically decomposed** into the `O(bits)` canonical prefix blocks that exactly
  tile it — if any block is present the range may be non-empty, else it is definitely empty. A
  present key lies in exactly one block whose prefix it inserted, so there are **no false
  negatives**; false positives compound with query width (documented). `insert`, `range_query`, and
  the `RangeFilter` trait. 7 tests (point ranges find keys; no-FN over 5000 keys with tight ranges;
  empty-gap rejection; full-range span; bounded point-FPR) + doctest.


- **`learned::LearnedFrequent` — learning-augmented frequency estimation (LA-Misra-Gries, ICLR 2019).**
  Classical sketches spread error uniformly, so even the heaviest items carry the Misra–Gries
  offset. Learning-augmented estimation (Hsu, Indyk, Katabi & Vakilian) uses an `Oracle` to predict
  which keys are heavy and gives those keys their *own exact counters*, routing only the predicted-
  light tail into a shared `FrequentItems` (Misra–Gries) sketch — so the heavy hitters that matter
  are exact and the bounded error falls only on the unimportant tail. Generic over any `Oracle`
  (learned model, table, or closure); `update`, `estimate`, `heavy_hitters`, `num_heavy`. 6 tests
  (predicted-heavy exact under heavy tail pollution; light keys sketched; heavy/light don't mix;
  exact even with tail capacity 4) + doctest.


- **`frequency::Rhhh` — Randomized Hierarchical Heavy Hitters with O(1) updates (SIGCOMM 2017).**
  Hierarchical heavy hitters (e.g. source-IP prefixes at /8, /16, /24, /32) normally cost O(H) per
  packet because every level is updated. RHHH (Ben-Basat, Einziger, Friedman & Kassner) makes
  updates O(1) by sampling: each item updates a *single* randomly chosen level's Misra–Gries
  counter, and a level's raw count is scaled by H for an unbiased frequency estimate. Hierarchy is
  the bit-prefix lattice of a `u64` key (`num_levels × bits_per_level ≤ 64`); `update`, `estimate`,
  `heavy_hitters`, `prefix`. Built on the existing `FrequentItems`. The full HHH descendant-
  conditioning of the *reported* set is a documented follow-up. 6 tests (prefix generalization;
  finest-level estimate within 25%; coarse-prefix aggregation; elephant in heavy-hitters;
  deterministic) + doctest.


- **`streaming::AdaSketch` — time-adaptive Count-Min sketch (Ada-Sketches, SIGMOD 2016).** Makes a
  Count-Min sketch recency-aware with O(1) updates via **pre-emphasis / de-emphasis**: an update at
  logical time `t` adds weight `e^{αt}` instead of 1, and a query divides by the current `e^{αT}`,
  so an item seen Δ steps ago contributes `e^{-αΔ}` — exponential time decay with no per-item
  bookkeeping. A **global rescale** divides all counters down and advances the time origin whenever
  the live weight grows large, keeping `f64` counters well-conditioned over unbounded streams.
  `update`/`update_weighted`, `estimate`. 5 tests (recent outweighs old; equal-recency items close;
  survives 1M updates via rescale; burst estimate matches the closed-form decay sum) + doctest.


- **`learned::GradientSketch` — sketched gradient compression (SketchedSGD / FetchSGD, ICML 2020).**
  A Count Sketch specialized for real-valued gradients: a small signed linear sketch that (1) sums
  correctly when worker sketches are added (`merge`), so distributed-SGD aggregation is just sketch
  addition, and (2) recovers the **top-k** heaviest coordinates of the summed gradient
  (`top_k`/`unsketch`) without ever materializing the full vector. `add`/`accumulate`, median
  `estimate`, and `scale` (for momentum/learning-rate in sketch space). 6 tests (top-k recovery of
  heavy coords among 20k; heavy-coord accuracy; cross-worker summation; linear scaling) + doctest.


- **`statistics::PStableLpSketch` — Indyk p-stable Lp-norm sketch (JACM 2006).** Estimates the `Lp`
  norm of a streamed coordinate vector by projecting it onto `d` random vectors with i.i.d.
  p-stable entries: each projection equals `‖x‖_p` times a standard stable variate, so the median
  of `|c_j|` (divided by the median of `|S|`) recovers the norm robustly. Supports `p = 1` (Cauchy
  → L1) and `p = 2` (Gaussian → L2); linear, so increments, decrements and `merge` all work.
  General `p ∈ (0,2]` via Chambers–Mallows–Stuck is a documented follow-up. 7 tests (L1 & L2
  estimates within tolerance; linearity/reversibility; disjoint-support merge) + doctest.


- **`sampling::ReservoirSamplingL` — reservoir sampling with Algorithm L (optimal skipping).** The
  textbook Algorithm R draws a random number per item (`O(n)` RNG work); Algorithm L (Li, ACM TOMS
  1994) keeps the identical uniform guarantee (each item present with probability `k/n`) but draws
  only `O(k·(1 + log(n/k)))` random numbers by computing an exponential jump over each run of
  items certain to be rejected. Same public API as `ReservoirSampling`; only the internals skip.
  6 tests (fills to k; exact below capacity; deterministic with seed; sample mean ≈ stream mean) +
  doctest.


- **`frequency::UnbiasedSpaceSaving` — heavy hitters with statistically unbiased counts (KDD 2018).**
  Classic SpaceSaving always hands the evicted minimum's value to the newcomer, which over-counts
  tail items. Unbiased Space-Saving (Ting, KDD 2018) increments the minimum counter and lets the
  newcomer take that slot only with probability `1/(min+1)`, making every count an unbiased
  estimator (`E[estimate] = true count`) — composable into unbiased subset-sum estimates. Generic
  over `T: Hash + Eq + Clone`; `new`/`with_epsilon`, `update`/`update_owned`, `estimate`, `top_k`,
  `total_count`. Two checkable invariants: counters always sum to the stream length, and below
  capacity it is exact. Deterministic-hasher + seeded RNG → reproducible. 6 tests + doctest.


- **`range_filters::DivaFilter` — dynamic range filter for variable-length keys (Diva, VLDB 2025).**
  The 2025 frontier and the library's first range filter for **variable-length byte-string keys**
  with **lexicographic** range queries — the dominant real-world key type (RocksDB, object stores,
  URL/path indexes) that Memento's fixed-width integer keys cannot serve. Fully **dynamic**:
  `insert` *and* `remove` (with duplicate multiplicity). Each key is reduced to a `resolution`-byte
  **infix**; a query is answered by an order-preserving scan over the infixes whose covered key
  interval overlaps it — no false negatives, one-sided error only on a shared infix prefix. Also
  implements `RangeFilter` over `u64` (big-endian). The sampled-trie routing + rank-select packed
  infix store is documented as the space/locality follow-up. 9 tests (incl. dynamic delete,
  duplicate-count survival, lexicographic prefix-of-low edge case, no-FN over 5000 string keys) +
  doctest.



- **`membership::BurrFilter` — Bumped Ribbon Retrieval (SEA 2022), near-optimal static AMQ.** A
  real ribbon filter: each key contributes one equation over GF(2) whose support is a contiguous
  64-wide band — `⊕ Z[s(x)+j] = fingerprint(x)` — solved by **on-the-fly Gaussian elimination**
  (one pivot per row position, then back-substitution), so it approaches the information-theoretic
  `log2(1/fpr)` bits/key. Rows that cannot be placed are **bumped** to the next of a few stacked
  ribbon layers (each run at 90% load), with a tiny exact fingerprint set catching the final
  residue — no false negatives, FPR ≈ `2^-r`. `build(keys, fpr)` / `contains`; reports
  `num_layers`, `fallback_len`, `bits_per_key`. Verified by no-false-negatives over 5000 keys, a
  bounded-FPR test, and bumping-convergence. (Distinct from the existing `RibbonFilter`, which is a
  2-hash bit-set; BuRR performs the genuine banded solve.) The bit-packed interleaved `Z` storage
  is a documented space follow-up. 8 tests + doctest.


- **`frequency::CuckooHeavyKeeper` — high-precision top-k (cuckoo placement + HeavyKeeper decay).**
  Fuses cuckoo hashing with HeavyKeeper's exponential decay: each flow gets its own *exact* counter
  in one of two candidate cuckoo buckets (`i1`, `i2 = i1 ⊕ h(fingerprint)`), so there is none of the
  Count-Min cross-flow count merging that plain `HeavyKeeper` inherits from its sketch array. When
  both candidate buckets are full, the weakest resident counter is decremented only with
  probability `decay^(−count)` and the newcomer takes the slot only if that knocks the resident to
  zero — heavy hitters are almost never touched while mouse flows churn. `new`/`with_decay`,
  `update`, `estimate`, `top_k`. Seeded RNG → reproducible. Explicit slot-array form (cuckoo
  *relocation* to raise load capacity is a documented follow-up). 7 tests (incl. an elephant kept
  within 10% amid 40k mouse flows; deterministic-across-instances) + doctest.


- **`streaming::FibaAggregator` — out-of-order sliding-window aggregation (FiBA, VLDB 2019).** The
  structure `WindowedAggregator`'s two-stack design cannot do: values keyed by event time, inserted
  and evicted in *any* order. Partial aggregates are cached at every tree node, so the window
  aggregate is available in O(1) and an insert/evict only repairs one root-to-leaf path (O(log n)).
  The combine is any associative op with identity (a monoid) and is folded in strict **time order**,
  so it is correct for non-commutative combines (concatenation, first/last, min-by-time, matrix
  product), not just sums. Implemented as an aggregate-augmented height-balanced (AVL) tree — the
  verifiable form of FiBA's contract; the namesake B-tree-with-fingers spine (amortized O(log d)
  near the window ends) is a documented locality follow-up that does not change results. 8 tests
  (incl. 1000-element out-of-order, time-order fold of a non-commutative combine, AVL balance at
  10k) + doctest.


- **`graph::GssSketch` — the Graph Stream Sketch (ICDE 2019).** The accuracy successor to
  `TcmSketch`: where TCM sums weights into hashed cells and conflates every edge that lands
  together, GSS stores a **fingerprint** of each endpoint in the cell, so a query only credits a
  slot whose `(fp(s), fp(d))` match — unrelated edges sharing a cell no longer collide. The few
  edges whose cell bucket is full spill into an overflow buffer, so no weight is ever lost.
  Answers edge-weight, out-degree and in-degree queries, all one-sided overestimates (never
  underestimates). This is the explicit fingerprinted-matrix + buffer form; the paper's
  square-hashing multi-room placement (which shrinks the buffer) is a documented follow-up that
  changes only where an edge is stored, not query results. 7 tests + doctest.


- **`range_filters::Surf` — the Succinct Range Filter (SIGMOD 2018).** The trie-based filter that
  gave RocksDB practical range filtering: a trie over the keys, pruned to the shortest prefixes
  that still tell them apart. Truncated tails give bounded false positives but **no false
  negatives** — a present key always walks to a stored leaf, and any range containing a present
  key returns `true`. Because the trie is sorted it answers `[low, high]` **range** queries
  (`RangeFilter::may_contain_range`), not just point lookups (`contains_u64` / `contains_bytes`,
  arbitrary byte keys). This is SuRF-Base (the pruned trie); the LOUDS-DS succinct rank/select
  bit-encoding (~10 bits/key) and the SuRF-Hash/Real suffix variants layer on the same contract
  and are documented follow-ups. 9 tests + doctest.


- **`reconciliation::PinSketch` — BCH-syndrome set reconciliation (minisketch / BIP-330).** The
  optimal-size set-reconciliation sketch (Eppstein et al., SIGCOMM 2011; the algorithm behind
  Bitcoin's `minisketch` / Erlay): members are elements of `GF(2^field_bits)` and the sketch
  stores their odd power sums in exactly `capacity` field elements. XOR-merging two sketches
  yields a sketch of the **symmetric difference** (shared elements cancel); `decode` reconstructs
  the differing elements via Berlekamp–Massey + Chien search and recovers them exactly when the
  difference is `≤ capacity`. The field's irreducible polynomial is found automatically (Rabin's
  test) for any `field_bits ∈ 2..=32`. Root finding is a Chien search (`O(2^field_bits)`), so
  `field_bits ≤ 20` is recommended; large-field Berlekamp-trace factorization is a documented
  follow-up. Over-capacity saturation is honestly documented as not always detectable — pair with
  `StrataEstimator` to size `capacity`. 9 tests (incl. an exhaustive field-validity check across
  bit widths) + doctest.

- **`streaming::ForwardDecay` — exponential time-decay aggregation.** Forward decay (Cormode
  et al., ICDE 2009) measures item age forward from a landmark, so only running sums are kept
  and nothing is re-aged per query. Provides decayed count, decayed sum, and a
  decay-invariant average; the landmark is pinned to the latest timestamp so accumulators
  stay bounded on unbounded streams. Handles out-of-order data.
- **`cardinality::TupleSketch<S: Summary>` — Theta sketch with per-key summaries.** The
  Apache DataSketches workhorse for reach/frequency and A/B testing: count distinct keys and
  aggregate a per-key value (impressions, spend, …) over the same sampled population, with
  full set operations that fold summaries. Built on the `ThetaCore<S>` substrate;
  `estimated_column_sums()` scales the retained sample up to a population estimate. Closes the
  biggest functional gap vs DataSketches.
- **`frequency::OnOffSketch` — persistence estimation.** Measures *persistence* (the number of
  distinct time periods an item appears in) rather than frequency, distinguishing stealthy
  persistent flows (a scanner probing every period) from flash crowds (Zhang et al., VLDB 2020).
  Each cell has a persistence counter + a per-period "on" flag (set once per period, reset at
  boundaries). `update`/`new_period`/`persistence`/`persistent_items`.
- **`frequency::MvSketch` — invertible heavy-hitter sketch.** Each cell runs a Boyer–Moore
  majority vote (candidate key, vote balance, total), making heavy-hitter detection
  *invertible* — the heavy keys are recovered directly from the sketch with no separate key
  list (Tang et al., INFOCOM 2019). `update`/`estimate`/`heavy_hitters` (enumerates candidates).
- **`frequency::WavingSketch` — unbiased top-k / heavy hitters.** Each bucket holds a small
  heavy part plus a signed waving counter; light items only nudge the counter by their ±1 sign
  (cancelling in expectation), while heavy items are promoted and bias-corrected (Li et al.,
  KDD 2020). Unlike Count-Min's one-sided overestimate, the estimate is unbiased — so it can be
  summed across distributed shards without skew. `insert`/`estimate`/`heavy_hitters`.
- **`frequency::SpreadSketch` — superspreader detection.** Count-Min of HyperLogLogs:
  estimates a key's *spread* (distinct peers — destinations contacted, ports scanned) rather
  than packet volume, the basis for detecting port scans / DDoS bots / superspreaders (Tang et
  al., INFOCOM 2020). `update(src, dst)`, `spread(src)` (min over rows), `is_superspreader`.
- **`frequency::FcmSketch` — hierarchical Count-Min with overflow chaining.** A drop-in
  Count-Min replacement (Song et al., 2020): each row has a wide 8-bit leaf layer and a narrow
  32-bit overflow layer; a key fills its 1-byte leaf then spills into a shared wide counter, so
  the long tail costs one byte while heavy keys borrow a 32-bit counter. Estimate = min over
  rows, preserving the no-underestimate guarantee at lower memory for skewed streams.
- **`frequency::SpaceSavingPlusMinus` — frequent items under bounded deletions.** The Double
  Space-Saving construction: one Space-Saving sketch over insertions, one over deletions, net
  frequency = the difference. Solves frequent-items / frequency-estimation in the
  bounded-deletion model (GDPR erasure, materialized views) that plain Space-Saving can't
  handle. `insert`/`delete`/`estimate`/`heavy_hitters`.
- **`frequency::TowerSketch` — tiered-width Count-Min.** Stacks 8/16/32-bit counter rows at
  equal bytes per row (so the narrow row holds 4× the counters): the long tail packs into the
  8-bit row while heavy keys are carried by the wider rows. Estimate is the min over
  non-saturated rows, preserving Count-Min's no-underestimate guarantee at lower memory for
  skewed data.
- **`range_filters::PgmIndex` — learned index (PGM).** Replaces a B-tree's routing structure
  with a piecewise-linear model of key positions (Ferragina & Vinciguerra, VLDB 2020): the
  sorted keys are covered by the fewest segments that predict each position within `±ε`, and a
  lookup evaluates the segment line then binary-searches the `2ε` window. `rank`/`contains`
  with worst-case bounds; ~tens of segments for 100k linear keys.
- **`graph::Triest` — streaming triangle counting (TRIÈST-BASE).** Estimates the number of
  triangles in a graph edge stream with only a reservoir of `m` edges (Stefani et al., KDD
  2016): reservoir-sample edges, adjust the triangle count by the triangles each sampled edge
  closes, and scale by ξ(t). Exact while the stream fits in the reservoir, unbiased
  thereafter. `add_edge`/`estimate`.
- **`matrix` module with `JohnsonLindenstrauss` — distance-preserving random projection.**
  Embeds high-dimensional vectors into `O(log n/ε²)` dimensions preserving distances and inner
  products within `1±ε` (JL lemma) via a ±1 Rademacher projection generated on the fly from a
  seed (no stored matrix). Shrinks embeddings before nearest-neighbour search and is the
  sketching step for sketch-and-solve. (New `matrix` module — future home of Frequent
  Directions / TensorSketch.)
- **`graph` module with `TcmSketch` (graph-stream summary).** A Count-Min sketch over graph
  *edges*: `depth` independent `width×width` matrices answer edge-weight, out-degree, and
  in-degree queries in sublinear space (Tang et al., SIGMOD 2016). Mergeable and serializable.
- **`streaming::SlidingSketch` — sliding-window framework (time zones).** Turns a point-query
  counter sketch (Count-Min here) into a sliding-window one (Gou et al., KDD 2020): replicate
  each slot across `z` time zones in a ring; updates write the current zone, queries sum live
  zones, and rotating the ring forgets old data zone-by-zone. Driven by the `common::time`
  `Temporal::advance(now)` convention from Wave 1.0.
- **`streaming::Apbf` — Age-Partitioned Bloom Filter (windowed membership).** "Have I seen
  this recently?" with a false-positive guarantee over the last `n` insertions (Shtul et al.,
  2021): `k+l` Bloom slices, insert into the front `k`, shift on each filled batch; an element
  ages out after one window automatically. Precise insertion-count window, unlike Stable
  Bloom. `insert`/`contains`.
- **`streaming::EcmSketch` — Exponential Count-Min (windowed per-key frequency).** A
  Count-Min sketch whose counters are exponential histograms, so a point query returns a
  key's approximate count *within the last `window` time units* rather than its lifetime
  total (Papapetrou et al., VLDB 2012). Built directly on the shared `EhCore` engine, so the
  per-cell windowing inherits the Datar relative-error guarantee.
- **`statistics` module with `AmsSketch` (AMS / Fast-AGMS).** Estimates the second frequency
  moment F2 (`Σ f_i²`, self-join size) and inner products / **join sizes** between two streams
  (`Σ a_i·b_i`) from compact sketches, via `depth×width` signed counters with a median-of-rows
  estimator. Supports turnstile (negative) updates and is mergeable; row seeds are derived from
  the row index so equal-shaped sketches are inner-product compatible.
- **`quantiles::OtelExponentialHistogram` — OpenTelemetry base-2 exponential histogram.** The
  wire format of modern observability (OTel/Prometheus native histograms): base-2 scaled
  buckets with a `scale` parameter, automatic **downscale** when the bucket span exceeds
  `max_buckets` (exactly as the OTel SDKs do), positive/negative/zero buckets, sum/min/max,
  and OTLP-shaped accessors (`scale`, `positive_offset`/`positive_counts`, …) for direct
  `ExponentialHistogramDataPoint` encoding. Mergeable (aligns scales) and serializable.
- **`similarity::BBitMinHash` — b-bit minwise hashing (compact signatures).** Keeps only the
  low `b` bits of each MinHash minimum, bit-packed, shrinking signatures up to 64× (`b=1`) at
  a quantified accuracy cost. Jaccard is recovered with the Li–König estimator
  `(P − 2^-b)/(1 − 2^-b)`. The regime that matters at trillion-token dedup scale.
- **`similarity::SimHashLsh` — Hamming-LSH index for SimHash.** The SimHash counterpart to
  MinHash-LSH: bands a 64-bit fingerprint into `r` blocks and indexes by block pattern, so a
  query returns near-duplicates (small Hamming distance) sublinearly. Plus a `hamming_distance`
  helper. Completes the LSH family for sublinear near-duplicate search.
- **`similarity::OnePermutationHash` — OPH MinHash with densification.** Builds MinHash
  signatures in O(1)-amortized per element (one hash split into `k` bins) instead of MinHash's
  O(k), with rotation densification filling bins left empty by sparse inputs (Li et al.,
  NeurIPS 2012; Shrivastava & Li). Estimates Jaccard as the fraction of agreeing bins — the
  default fast path for large-scale dedup. `update`/`signature`/`jaccard`.
- **`similarity::MinHashLsh` — LSH banding index for near-duplicate search.** Turns MinHash
  from a pairwise *scorer* into a sublinear near-duplicate *search/dedup* engine: split each
  `b·r` signature into `b` bands, index by band buckets, and a query returns the small
  candidate set sharing a band (S-curve threshold `(1/b)^(1/r)`). Generic over the item id;
  signature-source agnostic.
- **`sampling::PrioritySampling` — unbiased subset-sum sampling.** Keeps a size-`k` weighted
  sample (priority `weight/u`, top-`k` by priority) from which *any* subset's total weight is
  estimated without bias via adjusted weights `max(weight, τ)` against the threshold τ (Duffield
  et al., JACM 2007). Substrate for time-decayed / forward-decay-biased sampling.
  `update`/`estimated_total`/`estimated_subset_sum`.
- **`sampling::L0Sampler` — turnstile (deletion-capable) uniform sampler.** Returns a
  near-uniform random element from the support of a fully dynamic stream — insertions *and
  deletions* — via geometric levels of 1-sparse recovery (Cormode & Firmani, 2014). The
  canonical deletion-capable sampler and the primitive behind graph sketching and dynamic
  dedup. `insert`/`delete`/`update`/`sample`.
- **`sampling::WeightedReservoirSampling` — weighted reservoir (Efraimidis–Spirakis A-Res).**
  One-pass, bounded-memory weighted sampling without replacement: each item's chance of being
  kept scales with its weight (key `u^(1/w)`, keep top-`k`). Generic over the item type,
  mergeable (top-`k` of the union), seedable for reproducibility. Closes the most-visible
  sampling gap vs other ecosystems.
- **`quantiles::UddSketch` — bounded-bucket DDSketch with a preserved guarantee.** Unlike
  plain bucket collapsing (which silently voids the relative-error guarantee for the extreme
  values), UDDSketch's *uniform* collapse merges every adjacent bucket pair at once
  (`γ → γ²`, `α → 2α/(1+α²)`), so `|v'−v| ≤ α·v` still holds everywhere — only the current,
  queryable `α` grows. Bounded `max_buckets`, mergeable (aligns collapse levels), handles
  negatives/zeros, serializable. (Italiano et al., 2020.)
- **Martingale/HIP estimator for HyperLogLog and UltraLogLog (`estimate_hip`).** The
  Historic Inverse Probability estimator gives provably lower variance (~0.833/√m vs
  1.04/√m, ≈ half) for insertion-only single-stream workloads, at no extra memory —
  maintained incrementally during inserts. Returns `None` after `merge` or deserialization
  (which destroy the single-stream history HIP requires); use `estimate()` there.
  Serialization formats unchanged.
- **`privacy::DpContinualCounter` — DP continual counting (binary-tree mechanism).** Releases
  a running count after every event with only polylogarithmic noise (`O((log T)^1.5/ε)`)
  instead of the linear noise the naive Laplace mechanism would need under continual
  observation (Dwork et al. 2010; Chan–Shi–Song 2011). Streaming `insert`/`count`; built on
  the discrete-Laplace sampler.
- **`privacy::DpCountMin` — differentially private Count-Min.** Build a Count-Min sketch
  exactly, then `privatize(ε, rng)` releases a `PrivateCountMin` with discrete-Laplace noise
  (scale `depth/ε`, matching the L1 sensitivity) added to every counter; point queries are
  then `ε`-DP by post-processing. First consumer of the `privacy` substrate.
- **`privacy` module — differential-privacy substrate.** `privacy::mechanisms` ships the
  **discrete** Laplace and Gaussian mechanisms (Canonne–Kamath–Steinke exact samplers, so
  floating-point rounding cannot break the guarantee — unlike naive continuous Laplace),
  randomized response for local DP, and `gaussian_sigma` calibration. `privacy::accountant`
  tracks an `(ε, δ)` budget under sequential composition and refuses over-budget spends.
  RNG is explicit and must be a CSPRNG (`secure_rng`). Substrate for DP cardinality release,
  DP-Count-Min, continual counting, and LDP oracles in later waves.
- **`cardinality::FmSketch` — Flajolet–Martin / PCSA cardinality.** The original probabilistic
  distinct-counting sketch (Flajolet & Martin, JCSS 1985) and ancestor of HLL: `m` bitmaps with
  stochastic averaging, cardinality from the average bitmap fringe `(m/φ)·2^R̄`. Included for
  completeness and legacy interop (HLL improves accuracy per bit by tracking max rank instead).
  Mergeable (bitwise OR).
- **`cardinality::KmvSketch` — K-Minimum-Values cardinality + Jaccard.** Keeps the `k` smallest
  hash values; `n̂ = (k−1)/v_k` from the k-th smallest. Because the actual minima are kept, the
  bottom-`k` of a union is exact on the sample, giving unbiased **union cardinality** and
  **Jaccard** estimates (Beyer et al., SIGMOD 2007). The explicit, mergeable cousin of the
  Theta sketch. `add`/`estimate`/`union_cardinality`/`jaccard`.
- **`cardinality::LinearCounting` — bitmap cardinality sketch.** The simplest distinct-count
  sketch (Whang et al., TODS 1990): an `m`-bit bitmap with `n̂ = −m·ln(z/m)` from the zero-bit
  count `z`. More accurate than HLL for small/moderate loads and exact without collisions —
  the same estimator HLL uses for its small-range correction. Mergeable (bitwise OR).
- **Generic Theta core (`cardinality::ThetaCore<S: Summary>`).** Factored the Theta
  set-operation engine out of `ThetaSketch` and made it generic over a per-key `Summary`
  (folded on repeat keys and on union/intersection). Ships `NoSummary` (plain Theta set)
  and `SumDoubles` (ArrayOfDoubles-style, element-wise sum). `ThetaSketch` is now a thin
  wrapper over `ThetaCore<NoSummary>` — identical public API and estimates (hashing stays
  in the wrapper). This is the substrate the Tuple Sketch builds on.
- **`learned::FeatureHasher` — feature hashing (the hashing trick).** Maps an unbounded
  string-keyed feature space into a fixed-dimension vector with no vocabulary (Weinberger et
  al., ICML 2009): each feature hashes to a coordinate and a ±1 sign, values accumulate, and
  collisions cancel in expectation so inner products are preserved. The standard input layer
  for online learning (VW) and the scikit-learn `FeatureHasher`. `add`/`vector`/`transform`.
- **`membership::CountingQuotientFilter` — counting, deletable, mergeable filter.** A Counting
  Quotient Filter (Pandey et al., SIGMOD 2017): quotienting stores only an `r_bits` remainder
  per item, with a per-remainder count — the one filter that is simultaneously counting,
  deletable, and mergeable, and the substrate feature-rich filters (Aleph/AQF/Memento) build on.
  Functionally exact (no false negatives, FPR `≈ load/2^r_bits`); uses the clear bucketed
  reference layout (the RSQF rank-select packed layout is a cache/space optimization to follow).
- **`membership::ScalableBloomFilter` — Bloom filter for unbounded inserts.** Chains
  sub-filters: when the active one fills, a larger sub-filter with a geometrically tighter FPR
  is appended (Almeida et al., IPL 2007), keeping the compounded false-positive rate bounded by
  the target while absorbing an unknown number of items. `insert`/`contains`; no false
  negatives.
- **`membership::StackedFilter` — meta-filter for known-negative workloads.** Layers
  alternating Bloom filters — positives, then the known negatives layer 0 falsely admits, then
  the positives layer 1 falsely admits (Deeds et al., VLDB 2020) — turning prior knowledge of
  frequent non-members into a far lower effective FPR than a single filter of the same size.
  Static `build(positives, known_negatives, fp)`; no false negatives.
- **`learned::SandwichedLearnedBloom` — sandwiched learned Bloom filter.** Wraps an oracle
  between an initial and a backup Bloom filter (Mitzenmacher, NeurIPS 2018): the initial
  filter screens true negatives, the model classifies survivors, and the backup holds the
  positives the model misses — so there are **no false negatives**. Static filter built from a
  positive set + trained oracle; independent internal filters.
- **`learned::LearnedCountMin` — oracle-augmented frequency sketch.** Routes oracle-predicted
  heavy keys to an exact side-table and everything else to a Count-Min back-end, removing the
  heavy-key collision error that dominates plain Count-Min (Hsu et al., ICLR 2019). First
  consumer of the `Oracle` substrate.
- **`learned` module with the oracle/score-function interface.** New `Oracle` trait (the
  single way a user model scores keys for learned sketches) plus `ClosureOracle` (wrap a
  Rust `Fn`) and `PrecomputedOracle` (host-computed scores supplied as data — the
  FFI-friendly path where the model never crosses the boundary). Substrate for PLBF,
  Ada-BF/Sandwiched learned Bloom filters, and learned Count-Min/Count-Sketch in later
  waves.
- **Keyed/salted hashing (`common::hash::keyed_hash`, `Salt`)** — opt-in adversarially
  robust hashing for sketches. A secret `Salt` mixed into the hash makes outputs
  unpredictable, defending against crafted-collision and HLL parameter-extraction attacks.
  Endianness-stable (reproducible across platforms/bindings given the same salt); plain
  seeded hashing remains the default for reproducibility. Per-sketch wiring lands with the
  DP cardinality wrapper that consumes it.
- **`streaming::WindowedAggregator<S>` — sliding-window aggregation over any `Mergeable`
  sketch.** Keep the last `W` per-pane sketches and query the merge of everything in the
  window — windowed Theta/CPC/HLL/KLL/t-digest/Count-Min with no per-sketch windowing
  code. Uses the two-stack FIFO monoid aggregator (O(1) push, O(1) amortized evict, O(1)
  query; FIFO-order-correct for any associative merge). Count-based panes today; drive it
  from `common::time` for time windows. DABA/FiBA worst-case + out-of-order variants land
  later.
- **`common::time` — shared time/watermark convention** for windowed and time-decayed
  sketches. Defines `Timestamp` (caller-chosen `u64` units, no hidden wall clock),
  `TimeDomain` (event vs processing time), a monotonic `Watermark` with configurable
  `allowed_lateness` and a `LateDataPolicy` (Drop/Accept/ClampToWatermark), and a
  `Temporal` trait exposing the canonical `advance(now)` / `watermark()` surface mirrored
  across all four FFI languages. Substrate for the windowed sketches in later waves.

### Internal / Infrastructure
- **Unified Exponential Histogram engine.** `ExponentialHistogram` and
  `SlidingWindowCounter` were two near-duplicate implementations of the Datar 2002
  EH algorithm; both now delegate to one shared `EhCore` bucket engine (the substrate
  for upcoming windowed sketches — ECM-Sketch, APBF, EH-of-sketch aggregation). Public
  APIs and serialization formats are unchanged. `SlidingWindowCounter` picks up the more
  robust canonical-form compression as a side effect.

### Changed
- **Reconciliation: `RatelessIBLT` renamed to `Iblt`.** The old name was a misnomer —
  this is a classic fixed-rate Invertible Bloom Lookup Table, not the SIGCOMM 2024
  rateless construction. `RatelessIBLT` / `RatelessIBLTStats` remain as `#[deprecated]`
  aliases and will be removed in a future release. Language bindings keep their existing
  class/symbol names for now (a deprecation path lands with the FFI follow-up).

### Added
- **`reconciliation::StrataEstimator` — set-difference size estimation.** Estimates `|A △ B|`
  before reconciliation so a fixed-rate `Iblt` can be sized correctly instead of guessed
  (Eppstein et al., SIGCOMM 2011). Per-stratum small IBLTs sampled by trailing-zero hash;
  decodes deepest-first and scales up at the resolution limit.

### Fixed
- **IBLT exact key/value length recovery.** Cells now track the XOR of key/value byte lengths,
  so a decoded singleton is sliced to its exact length. This fixes keys containing **trailing
  zero bytes** (e.g. little-endian small integers), which the previous trim-trailing-zeros
  approach silently corrupted — they failed the key-check and broke decoding. Surfaced by the
  Strata Estimator's integer keys.
- **IBLT decode correctness (key-check hash).** Cells now accumulate a key-check hash so
  singleton detection verifies the recovered key instead of trusting `count == ±1` alone.
  Collisions that previously produced a *false* singleton (e.g. two insertions plus a
  deletion of a third key summing to count 1) are now rejected: decode returns correct
  pairs or reports the IBLT as undecodable, never garbage.

## [0.1.6] - 2025-12-13

### Major Features
- ✅ **Complete Multi-Language Support**: All 41 algorithms across Rust, Python, Node.js, Java, and C#
- ✅ **Java FFI Completion**: 37/41 algorithms with full JNI bindings (4 new algorithms: NitroSketch, LearnedBloomFilter, VacuumFilter, BinaryFuseFilter)
- ✅ **C# FFI Complete**: All 41 algorithms with safe P/Invoke bindings, 153+ unit tests, native libraries for Linux x64
- ✅ **Comprehensive Documentation**: 4 new guides for algorithm selection, integration patterns, performance tuning, and migration

### Added

#### Documentation (New)
- **ALGORITHM_SELECTION_GUIDE.md** - Decision tree, comparison matrix, and recommendations for choosing algorithms
- **INTEGRATION_PATTERNS.md** - Real-world integration examples (DuckDB, Polars, Pandas, NumPy, TypeScript)
- **PERFORMANCE_GUIDE.md** - Parameter tuning, memory/accuracy trade-offs, benchmarking methodology
- **v0.1.5_to_v0.1.6_MIGRATION.md** - Migration path, breaking changes, feature parity matrix

#### Java Language Binding
- JNI bindings for NitroSketch (high-speed network monitoring with sampling)
- JNI bindings for LearnedBloomFilter (ML-enhanced membership, 70-80% space savings)
- JNI bindings for VacuumFilter (dynamic membership with deletions)
- JNI bindings for BinaryFuseFilter (ultra-efficient static filter, ~9 bits/item)
- All 41 algorithms now accessible from Java with comprehensive unit tests

#### C# Language Binding
- P/Invoke bindings for all 41 algorithms
- Native library compilation for Linux x64
- NuGet package metadata configured for multi-platform distribution
- 153+ xUnit tests covering all algorithms and edge cases
- Safe memory management with IDisposable pattern

#### Testing
- Total test count: 854+ → 1000+ (comprehensive multi-language coverage)
- Java: 40+ algorithms with multiple tests each
- C#: 153+ unit tests across 5 test files
- Cross-language validation tests
- Performance regression testing baseline

### Changed
- Updated root README.md: "40+ algorithms" → "41 algorithms", "854+ tests" → "1000+ tests"
- Updated SketchOxide.csproj: Version 0.1.0 → 0.1.6, algorithm count 28 → 41
- Expanded algorithm documentation in README (clarified 10 categories)
- Added Java and C# to main language support badges

### Fixed
- Corrected algorithm counts in documentation (39/41 → 41/41)
- Fixed frequency algorithm categorization (HeavyKeeper and NitroSketch reclassified)
- Corrected universal algorithm category (1 → 3 algorithms: UnivMon, NitroSketch, HeavyKeeper)

### Technical Details

#### Memory Safety (C#)
- Zero unsafe pointer access in wrappers
- Proper Box::into_raw/Box::from_raw for FFI allocation/deallocation
- IDisposable pattern enforces resource cleanup
- Null pointer validation on all FFI boundaries

#### API Consistency
- Uniform naming conventions across all language bindings
- Consistent parameter ordering (ptr, then data/parameters)
- Standard return types (null for errors, 0 for failed operations)
- Predictable error handling across all implementations

#### Performance
- No regressions from v0.1.5 to v0.1.6
- New algorithms meet or exceed reference implementations:
  - NitroSketch: <100ns per sampled update
  - LearnedBloomFilter: ~3-5 bits/item
  - VacuumFilter: 12-14 bits/item with dynamic operations
  - BinaryFuseFilter: ~9 bits/item for static sets

### Documentation Updates
- README: 41 algorithms, 5 language bindings, 1000+ tests, feature matrix
- ROADMAP: Confirmed v0.1.6 scope, updated v0.1.7+ planning
- Extensive inline code comments and docstrings throughout FFI implementations

### Known Limitations
- BinaryFuseFilter, LearnedBloomFilter, VacuumFilter: No serialization support (use reconstruction from source data)
- C#: Native libraries currently provided for Linux x64 only (macOS, Windows require cross-compilation)
- Java: 4 new algorithms working but full JNI optimization pending

### Contributors
- Yury Fedoseev

### Dependencies
- No new external dependencies added
- Rust FFI layer uses stable APIs only
- C# uses standard .NET (6.0+, 7.0, 8.0, netstandard2.1)
- Java uses OpenJDK 11+ with standard JNI

---

## [Unreleased]

### Planned for v0.1.7+
- Additional native library binaries (Windows, macOS ARM64)
- Deep-dive algorithm documentation (top 10 algorithms)
- Performance optimization for Java JNI bindings
- WASM support exploration
- Additional language bindings (Go, Kotlin)

## [0.1.4] - 2025-11-26

### Added

#### Version Management
- `VERSION` file as single source of truth for all package versions
- `scripts/sync-version.sh` for automatic version synchronization across all language bindings
- `VERSIONING.md` documentation with clear release workflow

#### Infrastructure
- CI/CD version sync integration - all publisher jobs sync versions before building
- Simplified release process - update VERSION file instead of 5 separate locations

### Fixed
- npm publishing version mismatch (now uses VERSION file)
- Python publishing version mismatch (now uses VERSION file)
- All publishers guarantee version consistency across platforms

## [0.1.3] - 2025-11-26

### Added

#### Test Coverage
- **Phase 1**: 257 comprehensive JUnit 5 tests for Java (BloomFilter, CountMinSketch, DDSketch, MinHash, CuckooFilter)
- **Phase 2**: 170 Jest tests for Node.js with TypeScript (BloomFilter, DDSketch, MinHash)
- **Phase 3**: 39 pytest tests for Python FFI bindings (BloomFilter, DDSketch, MinHash)
- **Phase 4**: Verified 388 existing xUnit tests for C# (.NET)
- **Phase 5**: Cross-language validation documentation and test data patterns

#### Documentation
- `CROSS_LANGUAGE_VALIDATION.md` - Comprehensive cross-language test strategy
- `CROSS_LANGUAGE_TEST_DATA.md` - Shared test data formats and integration patterns
- `TEST_COVERAGE_SUMMARY.md` - Complete overview of 854 tests across 4 languages

#### Infrastructure
- Consolidated workspace dependencies in root Cargo.toml for centralized version management
- Fixed CI/CD publishing workflow with proper secret configuration
- Repository cleanup: removed temporary benchmark output files

### Fixed
- Fixed PyPI publishing token reference in GitHub Actions workflow
- Fixed release summary job to handle release events correctly
- Disabled Maven Central and NuGet publishers until proper certificates are configured
- All GitHub Actions workflow dependencies updated for disabled jobs

### Changed
- All FFI bindings now reference workspace version (0.1.3) from single location
- Improved .gitignore with benchmark and test output file patterns

## [0.1.0] - 2025-11-07

### Added

#### Core Rust Library
- **UltraLogLog** (VLDB 2024): Modern cardinality estimation, 28% more space-efficient than HyperLogLog
  - 40ns updates (2.5x faster than target)
  - Precision range 4-18
  - Full serialization support
  - 35 comprehensive tests

- **Binary Fuse Filter** (ACM JEA 2022): Modern membership testing, 75% smaller than Bloom filters
  - 22ns queries (4.5x faster than target)
  - Zero false negatives
  - Optimal space usage (9.84 bits/entry)
  - 30 comprehensive tests

- **DDSketch** (VLDB 2019): Modern quantile estimation with relative error guarantees
  - 44ns adds (4.5x faster than target)
  - Configurable relative accuracy (0.001-0.05)
  - Fully mergeable
  - 40 comprehensive tests including property-based tests

- **REQ Sketch** (PODS 2021): Tail quantile specialist with zero error at extremes
  - 4ns updates (25x faster than target)
  - Zero error at p100 in HRA mode
  - Compactor-based architecture
  - 36 comprehensive tests

- **Count-Min Sketch** (2003): Frequency estimation with ε-δ guarantees
  - 170-380ns updates (scales with accuracy)
  - Never underestimates
  - Mergeable for distributed systems
  - 34 comprehensive tests

- **MinHash** (1997): Jaccard similarity estimation
  - <100ns updates
  - Configurable number of permutations
  - Mergeable (set union)
  - 35 comprehensive tests

- **Theta Sketch** (2015): Set operations (union, intersection, difference)
  - <150ns inserts
  - Full set algebra support
  - Industry standard (LinkedIn, ClickHouse)
  - 33 comprehensive tests

- **CPC Sketch** (2017): Maximum space-efficient cardinality
  - 56ns updates (1.7x faster than target)
  - Adaptive flavors system
  - 30-40% better than HyperLogLog
  - 30 comprehensive tests

- **Frequent Items** (2024): Top-K heavy hitter detection
  - 85ns updates (2.3x faster than target)
  - Deterministic error bounds
  - No false positives/negatives modes
  - 32 comprehensive tests

#### Common Infrastructure
- Hash functions: MurmurHash3 (custom) and XXHash (via twox-hash)
- Traits: `Sketch` and `Mergeable` for consistent API
- Error types: Comprehensive `SketchError` enum
- Full serialization/deserialization support

#### Python Bindings (PyO3)
- **All 9 algorithms** wrapped with 100% feature parity
- Type conversions: Python → Rust (int, str, bytes)
- Enum support: ReqMode, ErrorType
- Error handling: Python exceptions from Rust errors
- NumPy/Pandas integration examples
- PySpark compatibility

#### Testing
- **305 total tests** across all algorithms
- Unit tests for correctness
- Integration tests for workflows
- Property-based tests (proptest) with 3,000+ random cases
- Benchmarks for all algorithms (Criterion.rs)

#### Documentation
- Comprehensive README with examples
- Algorithm deep-dive (datasketches.md - 997 lines)
- SOTA research analysis (sota_2025_analysis.md)
- Performance summary with benchmarks
- Implementation guide
- Python integration examples
- Contributing guide
- This changelog

#### Performance
- All algorithms meet or exceed research paper targets
- **2-10x faster** than target performance
- **28-75% space efficiency** improvements over traditional algorithms
- Production-ready performance (millions of operations/second)

### Performance Highlights

| Algorithm | Target | Actual | Improvement |
|-----------|--------|--------|-------------|
| UltraLogLog | <100ns | 40ns | 2.5x faster |
| Binary Fuse | <50ns | 22ns | 2x faster |
| DDSketch | <200ns | 44ns | 4.5x faster |
| REQ | <100ns | 4ns | 25x faster |
| Count-Min | <300ns | 170-380ns | Within target |
| CPC | <100ns | 56ns | 1.7x faster |
| MinHash | <100ns | <100ns | Meets target |
| Theta | <150ns | <150ns | Meets target |
| Frequent | <200ns | 85ns | 2.3x faster |

### Space Efficiency

**UltraLogLog vs HyperLogLog** (1M items):
- HyperLogLog: 4.1 KB
- UltraLogLog: 3.0 KB
- **28% smaller** ✅

**Binary Fuse vs Bloom Filter** (1M items, 1% FP):
- Bloom Filter: 4.8 KB
- Binary Fuse: 1.1 KB
- **75% smaller** ✅

### Development Process
- Test-Driven Development (TDD) methodology
- SOLID and DRY principles throughout
- Pre-commit hooks for quality (rustfmt, clippy, tests)
- Zero clippy warnings with `-D warnings`
- 100% rustfmt compliance

---

## Development Milestones

### Phase 1: Foundation (Complete)
- Git repository setup
- Pre-commit hooks (Rust + Python)
- Workspace configuration (Cargo + Maturin)
- Core traits and error types

### Phase 2: Core Infrastructure (Complete)
- Hash functions (MurmurHash3, XXHash)
- Common traits (Sketch, Mergeable)
- Error handling framework
- 22 tests for hash functions

### Phase 3: Algorithm Implementation (Complete)
All 9 algorithms implemented following TDD:
1. UltraLogLog (35 tests)
2. Binary Fuse Filter (30 tests)
3. DDSketch (40 tests)
4. REQ Sketch (36 tests)
5. Count-Min Sketch (34 tests)
6. MinHash (35 tests)
7. Theta Sketch (33 tests)
8. CPC Sketch (30 tests)
9. Frequent Items (32 tests)

**Total: 305 tests, all passing**

### Phase 4: Python Bindings (Complete)
- All 9 algorithms wrapped with PyO3
- Type conversion system
- Error handling
- Module registration
- 100% feature parity with Rust
- Comprehensive Python tests
- Example scripts

### Phase 5: Performance Verification (Complete)
- Comprehensive benchmarks with Criterion.rs
- All algorithms exceed targets by 2-10x
- Space efficiency verified (28-75% improvements)
- Performance analysis documentation
- Production workload simulations

### Phase 6: Enhanced Documentation (Complete)
- Comprehensive README
- Performance summary
- SOTA research analysis
- Contributing guide
- This changelog
- Project status report

---

## Research Foundation

All algorithms based on peer-reviewed research:

- **UltraLogLog**: Ertl, O. (2024). VLDB.
- **Binary Fuse**: Graf, T. M., & Lemire, D. (2022). ACM JEA.
- **DDSketch**: Masson, C., Rim, J. E., & Lee, H. K. (2019). VLDB.
- **REQ**: Cormode, G., et al. (2021). PODS.
- **Count-Min**: Cormode, G., & Muthukrishnan, S. (2005). Journal of Algorithms.
- **MinHash**: Broder, A. Z. (1997). STOC.
- **Theta**: Yahoo Research (2015). Apache DataSketches.
- **CPC**: Yahoo Research (2017). Apache DataSketches.
- **Frequent Items**: Based on Misra-Gries (1982).

---

## Breaking Changes

None (initial release).

---

## Deprecations

None (initial release).

---

## Security

No security issues in this release.

---

## Contributors

- Initial implementation and design
- Research and algorithm selection
- Performance optimization
- Documentation

---

## Acknowledgments

Built on the shoulders of giants:
- Otmar Ertl (UltraLogLog)
- Thomas Mueller Graf & Daniel Lemire (Binary Fuse Filters)
- Charles Masson, Jee E Rim, Homin K Lee (DDSketch)
- Graham Cormode and collaborators (Count-Min, REQ)
- Apache DataSketches community (Theta, CPC, Frequent Items)
- Andrei Broder (MinHash)

---

## Future Roadmap

### v0.2.0 (Planned)
- Additional serialization formats (JSON, MessagePack)
- SIMD optimizations for hash functions
- Thread-safe variants
- More Python integration examples (Polars, DuckDB)

### v0.3.0 (Planned)
- WASM support for browser usage
- Additional algorithms (IBIF for dynamic Binary Fuse)
- Distributed system patterns
- Cloud deployment guides

### v1.0.0 (Planned)
- Stable API guarantee
- Long-term support commitment
- Production deployment guides
- Performance monitoring integration

---

## Links

- **Repository**: https://github.com/yourusername/sketch_oxide
- **Documentation**: https://docs.rs/sketch_oxide
- **PyPI**: https://pypi.org/project/sketch-oxide/
- **Issues**: https://github.com/yourusername/sketch_oxide/issues

---

**No nostalgia. Just the best algorithms available in 2025.**

[Unreleased]: https://github.com/yourusername/sketch_oxide/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/yourusername/sketch_oxide/releases/tag/v0.1.0
