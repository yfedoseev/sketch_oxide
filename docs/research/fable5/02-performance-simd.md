# Performance & SIMD Audit — Path to SOTA 2026

> Part of the `docs/research/fable5/` improvement research series (2026-07-01).
> Scope: raw-performance engineering of the Rust core and FFI hot paths, complementing
> the algorithm-coverage roadmap in `docs/sandbox/research/00-roadmap-2026.md`.

## Executive summary

The codebase is clean, correct-first Rust with some good micro-optimizations already in
place (Kirsch–Mitzenmacher + Lemire fast-range in Bloom, flat table + power-of-2 mask in
Count-Min, HIP estimator in HLL, cache-line-blocked Bloom). But it is **entirely scalar**
(zero SIMD anywhere), ships **no `[profile.release]` tuning at all**, has **no
parallel/concurrent story** (no rayon, no atomics), several **hot loops re-hash the full
key k times or allocate per insert**, and **batch APIs cover only ~14 of 184 Python
binding modules**, with zero GIL release (`allow_threads` count: 0) and essentially no
batch support in Node. Benchmarks are extensive (30 Criterion bench files incl. "us vs
them") but CI only runs `cargo bench --no-run` — no regression gate.

---

## Ranked findings

### 1. No `[profile.release]` / LTO / codegen tuning anywhere — HIGH impact, S effort

**Evidence:** `grep '\[profile' **/Cargo.toml` returns nothing; the workspace root
`Cargo.toml` has no profile section. All benches and all four binding crates ship with
the default release profile (16 codegen units, no LTO, panic=unwind).

**Why it matters:** For a hash-heavy, tiny-hot-loop library, `lto = "fat"`,
`codegen-units = 1`, and `panic = "abort"` (for the cdylib bindings) routinely yield
5–20% on exactly these workloads, and better cross-crate inlining of
`twox_hash`/`xxhash_rust` into hot loops. This is free performance being left on the
table on every published wheel/npm/jar.

**Recommendation:** Add to workspace root:

```toml
[profile.release]
lto = "fat"
codegen-units = 1
```

plus `panic = "abort"` in binding crates; consider a `[profile.bench]` inheriting it.
Also document/ship `RUSTFLAGS="-C target-cpu=native"` guidance and consider
`target-feature=+avx2` variant wheels.

**Expected gain:** 5–20% across the board. **Effort: S.**

### 2. Zero SIMD in any hot path — HIGH impact, M–L effort

**Evidence:** `grep -r 'simd|std::arch|target_feature|portable_simd'` over
`sketch_oxide/src` hits only a *comment* in
`src/membership/vector_quotient_filter.rs:8-9` ("the SIMD... this implementation is the
portable scalar version"). No `std::arch`, no `wide`, no `portable_simd` dependency.

SIMD-amenable scalar paths found:

- **HLL estimate/count_zeros:** `src/cardinality/hyperloglog.rs:284-298, 312-314` —
  per-register `2.0f64.powi(-r)` + `.sum()`, and a scalar zero-count. DataSketches C++
  vectorizes this with an AVX2 lookup-table harmonic sum (registers ≤ 64 → precomputed
  `2^-r` table indexed via gather/shuffle); even scalar, replacing `powi` with a 64-entry
  LUT is a big win.
- **Bloom `contains`/`insert` k-probe loop:** `src/membership/bloom.rs:127-165` — probes
  are independent; AVX2 can compute 4–8 probe indices at once (fastfilter-style).
- **Count-Min update/estimate rows:** `src/frequency/count_min.rs:170-184, 228-239` —
  currently *serially dependent* (see finding 3), which also blocks SIMD/ILP.
- **MinHash `update`:** `src/similarity/minhash.rs:174-185` — 128 independent
  min-updates; textbook AVX2 `vpminuq`-style loop once permutations are multiply-shift
  (finding 4).
- **VQF block scan** (`vector_quotient_filter.rs`) — the paper's whole point is SIMD
  block probing.

**Recommendation:** Start with safe autovectorization-friendly rewrites (LUT for `2^-r`,
chunked `count_zeros` via `bytecount`-style tricks), then add `#[cfg(target_arch)]`
AVX2/NEON kernels with runtime `is_x86_feature_detected!` dispatch for Bloom probes,
MinHash mins, and the HLL register scan.

**Expected gain:** 2–4x on HLL estimate, 1.5–3x on MinHash update, 1.3–2x on Bloom batch
queries. **Effort: M (LUT/autovec) to L (explicit intrinsics + dispatch).**

### 3. Count-Min derives rows via serial re-finalization of a stateful hasher — HIGH impact, S effort

**Evidence:** `src/frequency/count_min.rs:160-184` — per row: `hasher.finish()` then
`hasher.write(&[0x7B])`. Each row's hash depends on the previous (`finish` re-runs xxh64
finalization each time), creating a serial dependency chain that defeats ILP and
prefetching, and `write(&[0x7B])` on XxHash64 is not a cheap mixer.

**Recommendation:** Hash once to 128 bits (or two 64-bit seeds) and derive rows with
Kirsch–Mitzenmacher `h1 + i*h2` — exactly what `bloom.rs` already does — or a cheap
per-row multiply-mix. All d table indices then become computable up front → prefetch all
d cache lines before touching memory. Same fix applies to `estimate`.

**Expected gain:** 1.5–2.5x on CM update/query (d=5 typical; memory-latency bound,
prefetch is the multiplier). **Effort: S.**

### 4. MinHash rehashes the full item per permutation AND allocates a `Vec<u8>` per hash — HIGH impact, S effort

**Evidence:** `src/similarity/minhash.rs:174-185` calls `hash_with_seed` `num_perm`
(default 128) times; `hash_with_seed` (`minhash.rs:275-298`) constructs a
`ByteHasher { bytes: Vec::new() }`, serializes the item into a heap `Vec<u8>`, then runs
full xxh64 — so one `update()` = **128 heap allocations + 128 full-key hashes**.

**Why it matters:** State of the art (datasketch, DataSketches) hashes once and applies
k *universal permutations* `(a_i * h + b_i) >> shift` — O(1) hashing + k multiply-adds,
SIMD-friendly.

**Recommendation:** Hash the item once (single xxh64, no ByteHasher allocation), then
derive the 128 values with precomputed multiply-shift constants. Keeps the same accuracy
class; note it changes signatures (version the serialization).

**Expected gain:** 20–100x on MinHash update for non-trivial keys. **Effort: S–M** (M if
signature compatibility must be preserved).

### 5. FFI: per-item crossing dominates; batch coverage is ~8% and GIL is never released — HIGH impact for bindings users, M effort

**Evidence:**

- Python: only 14 of 184 modules in `python/src/` mention
  `update_batch`/`insert_batch`; `grep -r allow_threads python/src | wc -l` → **0**.
  numpy is a dependency but used in exactly one file (`python/src/ddsketch.rs:75`
  `PyReadonlyArray1<f64>` — the right pattern).
- Batch methods that do exist still walk a `PyList` item-by-item with per-item type
  dispatch: `python/src/hyperloglog.rs:164-187` (`update_batch_hashes` extracts each int
  individually rather than accepting `PyReadonlyArray1<u64>`); `python/src/common.rs:40-57`
  does a 5-way `extract` type probe per item.
- Node: `nodejs/src/lib.rs` (6461 lines) has `updateBatch(values: Vec<f64>)` only for
  two quantile sketches (lines 2711, 2811); everything else is per-item `Buffer` — and
  line 98 even has a comment "Future optimization: Add async_batch()".

**Recommendation:** (a) Add `update_batch(PyReadonlyArray1<u64>)` /
`contains_batch → PyArray1<bool>` numpy overloads for the top-10 sketches (HLL, ULL,
Bloom, BinaryFuse, CM, DDSketch, Theta, MinHash, KLL, Cuckoo); (b) wrap batch loops in
`py.allow_threads(...)`; (c) Node: accept `BigUint64Array`/`Float64Array` and pre-hashed
batches. Per-item PyO3 call overhead is ~100–200ns vs ~5ns core update.

**Expected gain:** 10–50x for Python/Node bulk workloads. **Effort: M** (mechanical but
wide).

### 6. BlockedBloom allocates a `Vec<usize>` per insert and hashes the key k+1 times — MEDIUM-HIGH, S effort

**Evidence:** `src/membership/blocked_bloom.rs:114` —
`let bit_indices: Vec<usize> = (0..k).map(...).collect();` heap allocation on *every
insert* ("to avoid borrow checker issues"). And `hash_block` (line 268-272) +
`hash_within_block` (line 276+) each call `xxh64(key, seed)` — the full key is hashed
**k+1 times** per op, plus `% self.num_blocks` (slow modulo) instead of the Lemire
fast-range already used in `bloom.rs:112-116`.

**Recommendation:** Hash once → split into block index (fast-range) + derive k bit
positions from the two hash halves via K-M double hashing (each position is just 9 bits
within a 512-bit block — one 64-bit hash yields 7 positions directly). No allocation.
This is the fastfilter "register-blocked" pattern; with one hash you can also set all k
bits via a computed 8×u64 mask and OR it in — fully branchless and autovectorizable.

**Expected gain:** 3–8x on BlockedBloom insert/query (currently likely *slower* than
plain `BloomFilter`, defeating its purpose). **Effort: S.**

### 7. HLL registers are `Vec<u8>` (8 bits/register) and estimate uses `powi` per register — MEDIUM, S–M effort

**Evidence:** `src/cardinality/hyperloglog.rs:84` (`registers: Vec<u8>`),
`hyperloglog.rs:155,170,247,291` — `2.0_f64.powi(-(r as i32))` in `from_registers`,
`recompute_kxq`, `update_hash`, and `raw_estimate`.

**Why it matters:** (a) 8-bit registers waste 25% memory vs 6-bit packing (Redis/
DataSketches pack 6-bit; the packing code already exists in `to_redis_bytes`,
lines 461-475); at p=18 that's 256KB vs 192KB — cache footprint matters for
merge/estimate. (b) `powi` is a function call; register values are 0..=64, so a 65-entry
`static [f64; 65]` LUT makes the estimate loop a pure gather+add that autovectorizes.

**Recommendation:** LUT for `2^-r` (S, big win on estimate/merge); optionally
6-bit-packed dense mode as a variant (M — keep u8 as the fast default like DataSketches
HLL_8 vs HLL_6, so this is a memory/speed knob, not a mandate).

**Expected gain:** 3–10x on `estimate()`/`recompute_kxq`; minor on update.
**Effort: S (LUT), M (packing).**

### 8. No parallelism / concurrent-update story at all — MEDIUM, M effort

**Evidence:** `grep -r 'rayon|AtomicU|std::thread' sketch_oxide/src` → zero hits in
source. No striped/atomic-register HLL, no parallel batch build for
BinaryFuse/Ribbon construction (BinaryFuse `try_construct`, `binary_fuse.rs:171+`, is a
serial peeling build).

**Why it matters:** 2026 state of the art (DataSketches, ClickHouse) offers thread-safe
concurrent HLL/theta updaters; filter construction over 10M+ keys is embarrassingly
parallelizable in stages (hash phase).

**Recommendation:** Optional `rayon` feature: parallel hash phase for
BinaryFuse/Xor/Ribbon construction; `AtomicU8`-register `ConcurrentHyperLogLog` (relaxed
max via CAS loop) and striped Count-Min. Keep it feature-gated to preserve zero-dep core.
This also aligns with the roadmap's cross-cutting item 9 (concurrency story:
OctoSketch-style per-core aggregation).

**Expected gain:** near-linear scaling for filter builds; enables multi-threaded ingest
use cases now impossible. **Effort: M.**

### 9. Hashing strategy: two xxhash deps, no modern-hash evaluation — LOW-MEDIUM, S effort

**Evidence:** Workspace `Cargo.toml` pulls **both** `twox-hash = "2.0"` and
`xxhash-rust = "0.8"` (duplicate xxh64 implementations); `siphasher` for keyed hashing
(justified, opt-in — `src/common/hash.rs` design is good). `bloom.rs:102-107` computes
**two full xxh64 passes** (seeds 0 and 1) where one **xxh3-128** call (available in
`xxhash-rust` behind the `xxh3` feature) yields both halves in one pass — xxh3 is also
~2x faster than xxh64 on short keys. No wyhash/rapidhash/komihash anywhere;
`hash_benchmarks.rs` exists but only covers current functions.

**Recommendation:** Consolidate on `xxhash-rust` (drop `twox-hash`, or vice versa);
switch Bloom/BlockedBloom base hashes to one xxh3-128 call; benchmark
rapidhash/komihash for short-key (u64) workloads — for 8-byte keys a single multiply-mix
(wyhash-style) is ~3x faster than xxh64 and quality-sufficient for sketches. Note:
changing hashes breaks serialized-sketch compatibility → gate behind a version bump /
algorithm-id byte. Ties into roadmap cross-cutting item 8 (hashing policy).

**Expected gain:** ~2x base-hash cost on Bloom-family; dependency hygiene. **Effort: S.**

### 10. Benchmarks: excellent coverage, no CI regression gate — MEDIUM, S effort

**Evidence:** 30 Criterion bench files in `sketch_oxide/benches/` including
`comparison_benchmarks.rs` vs pdatastructs/probabilistic-collections/
streaming_algorithms (good!). But `.github/workflows/test.yml:76-78` runs only
`cargo bench --no-run` (compile check), and Python benches are
`continue-on-error: true` (line 144). No `criterion` baseline comparison, no
`bencher`/`codspeed`/`iai-callgrind`. Not benchmarked against: the `datasketches` crate,
`amadeus-streaming`, or C++ baselines. `benchmark_results.txt` is a stale checked-in
snapshot.

**Recommendation:** Add a CodSpeed or `criterion --save-baseline` + `critcmp` PR gate on
5–6 canonical hot paths (HLL update/estimate, Bloom insert/contains, CM update,
BinaryFuse contains, DDSketch add); consider `iai-callgrind` for instruction-count
determinism in CI.

**Expected gain:** prevents regressions; no direct speedup. **Effort: S.**

### 11. Misc smaller items

- **`#[cold]` count: 0** across `sketch_oxide/src` (489 `#[inline]` uses though). Keep
  error/validation paths `#[cold]`/outlined where they sit near hot loops.
  Effort: S, gain: marginal.
- **Serialization does per-word `extend_from_slice` loops** (`bloom.rs:202-204`,
  `count_min.rs:325-327`; deserialize `count_min.rs:388-397` pushes u64s one at a time).
  Use `bytemuck`/chunked copies: 10–50x on serialize/deserialize of large sketches.
  Effort: S.
- **`Vec<Vec<f64>>` matrix layout** in `src/matrix/frequent_directions.rs:39` plus
  hand-rolled Jacobi eigendecomposition (line 165) — row-pointer chasing kills cache
  behavior; flat row-major `Vec<f64>` + `faer`/`nalgebra` (feature-gated) SVD would be
  5–20x on `condense()`. Effort: M.
- **`Vec<bool>` used in 4 files** (`morton_filter.rs`, `bloom_rf.rs`, `dpsw_sketch.rs`,
  `sketch_polymer.rs`) — 8x memory waste vs bitset. Effort: S.
- **HLL `update<T: Hash>` via `XxHash64` streaming Hasher** (`hyperloglog.rs:224-229`):
  for `&u64` the `Hash` impl feeds 8 bytes through the streaming path; a specialized
  `update_u64` using xxh3/multiply-mix would help the most common benchmark shape.
  Effort: S.

---

## Priority matrix

| # | Finding | Impact | Effort |
|---|---------|--------|--------|
| 1 | Add LTO/codegen-units release profile | 5–20% everything | S |
| 3 | Count-Min: K-M derivation + prefetch (kill serial hasher chain) | 1.5–2.5x CM | S |
| 4 | MinHash: hash-once + multiply-shift permutations (kill 128 allocs) | 20–100x MinHash | S–M |
| 6 | BlockedBloom: hash-once, no per-insert Vec, fast-range | 3–8x BlockedBloom | S |
| 5 | Batch + numpy + allow_threads across bindings | 10–50x FFI bulk | M |
| 7 | HLL `2^-r` LUT (+ optional 6-bit packing) | 3–10x estimate | S–M |
| 2 | SIMD kernels (Bloom probes, MinHash, HLL scan, VQF) | 1.5–4x hot paths | M–L |
| 9 | Hash consolidation, xxh3-128, short-key fast hash | ~2x base hashing | S |
| 10 | CI benchmark regression gate | guardrail | S |
| 8 | rayon feature: parallel builds, concurrent HLL | new capability | M |
| 11 | Serialization memcpy, matrix layout, Vec<bool>, #[cold] | localized | S–M |

Key files referenced: workspace `Cargo.toml`, `sketch_oxide/src/membership/bloom.rs`,
`sketch_oxide/src/membership/blocked_bloom.rs`, `sketch_oxide/src/frequency/count_min.rs`,
`sketch_oxide/src/cardinality/hyperloglog.rs`, `sketch_oxide/src/similarity/minhash.rs`,
`sketch_oxide/src/common/hash.rs`, `python/src/common.rs`, `python/src/hyperloglog.rs`,
`python/src/ddsketch.rs`, `nodejs/src/lib.rs`, `.github/workflows/test.yml`.
