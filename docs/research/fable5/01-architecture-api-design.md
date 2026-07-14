# Rust Core — Architecture & API-Design Audit (2026-07-01)

> Part of the `docs/research/fable5/` improvement research series.
> Scope: `sketch_oxide/src` (212 files, ~66k LOC, 211 public structs, 16 modules +
> `common`). This deliberately excludes algorithm-coverage gaps already documented in
> `docs/sandbox/research/00-master-gap-analysis.md` / `00-roadmap-2026.md`. Findings are
> ranked by impact.

---

## F1. Cross-language / cross-type hash incompatibility (silent correctness bug) — **Highest impact**

**Current state.** Core sketches hash via Rust's `std::hash::Hash` fed into
`XxHash64::with_seed(0)`:

- `sketch_oxide/src/cardinality/hyperloglog.rs:218-230` — `update<T: Hash>` →
  `item.hash(&mut XxHash64)`.
- `sketch_oxide/src/frequency/count_min.rs:160-185` — same pattern.

The FFI layers then feed *different Rust types* for the same logical item:

- Python (`python/src/common.rs:97-116`, `with_python_item!`): tries `i64`, then `u64`,
  then `String`, then `&[u8]`.
- Java (`java/src/lib.rs:65-80`): always `Vec<u8>` (`hll.update(&bytes)`).
- Node (`nodejs/src/lib.rs:96-101`): always `Vec<u8>` from a Buffer.

`str::hash` writes UTF-8 bytes **plus a 0xFF terminator**; `Vec<u8>/[u8]::hash` writes a
**native-endian `usize` length prefix** plus bytes; integers hash via `to_ne_bytes`.
Consequences:

1. `hll.update("abc")` from Python and `hll.update("abc".getBytes())` from Java hit
   **different registers**. Merging a Python-built and a Java-built sketch over
   identical data silently double-counts — the exact "merge-after-deserialize" workflow
   the roadmap calls canonical. The conformance script
   `tests/cross_language_validation.py` only tests Python, so this is undetected.
2. Native-endian length prefixes and integer bytes mean serialized sketches are not
   portable across endianness/pointer width even within Rust.
3. `Salt`/`keyed_hash` (`common/hash.rs:42-127`) is well-designed (redacted `Debug`,
   SipHash-1-3, documented threat model) but **adopted by zero sketches** —
   `grep -rln keyed_hash src | grep -v common` → 0. The adversarial-robustness story
   exists only in `common`.
4. Misleading shim: `murmur3_hash64` (`common/hash.rs:237-240`) is documented as
   "MurmurHash3 64-bit implementation" but silently calls xxhash.

**Why it matters.** A cross-language sketch library's single most important invariant is
"same logical item → same hash everywhere." DataSketches solves this by specifying
canonical byte encodings (e.g., strings hash as UTF-8, longs as 8 LE bytes) with a fixed
seed (9001). sketch_oxide has no such specification; correctness currently depends on
every binding accidentally choosing the same Rust type.

**Recommendation (L, but staged).** Define a documented *canonical item encoding*
(`trait SketchHashable { fn canonical_bytes(&self) / fn stable_hash(seed, salt) }`) with
fixed little-endian integer encoding and unprefixed UTF-8 for strings; route all
sketches and all four bindings through it; add cross-language golden hash vectors to CI.
Also delete or honestly rename `murmur3_hash64` (S), and wire `Salt` into at least the
cardinality/frequency constructors (`with_salt(...)`) (M).

---

## F2. Serialization: 84% of algorithms have none; no versioning; panics and silent data loss

**Current state.**

- **Coverage:** only 31 of 188 algorithm files implement any `to_bytes`/`serialize`;
  157 do not — including flagships `ThetaSketch` (`cardinality/theta.rs` — no serialize
  at all despite union/intersect/difference being its selling point), REQ, GK, all of
  `membership/` except Bloom/BinaryFuse-adjacent files, all quotient filters,
  `moments_sketch`, most of `privacy/`, `learned/`, `graph/`, `net/`.
- **No format discipline:** no magic bytes, no format-version byte, no endianness
  statement anywhere (`grep -rn "MAGIC\|FORMAT_VERSION" src` → 0). E.g. HLL format is
  ad-hoc `[precision:1][registers...]` (`cardinality/hyperloglog.rs`, `to_bytes`), and
  deserialization explicitly **loses HIP estimator state** ("HIP history is lost"
  comment in `from_bytes`).
- **Panics on malformed input:** `DDSketch::deserialize`
  (`quantiles/ddsketch.rs:473-525`) validates a 104-byte header, then reads
  attacker-controlled `pos_bins_len` and slices `bytes[pos..pos+8].try_into().unwrap()`
  in a loop with **no bounds checks** → index-out-of-range panic on truncated input.
  Similar unchecked-slice unwraps in `quantiles/kll.rs:431-459`.
  `CountMinSketch::deserialize` computes `depth * width * 8` without overflow checks
  after a truncating `width as u32` cast.
- **Silent data loss:** `SpaceSaving::serialize` (`frequency/space_saving.rs:535-551`)
  writes only the header and **drops all counters**, with a comment saying "only works
  when items HashMap is empty… use serde feature" — but **no serde feature exists
  anywhere in the workspace** (`sketch_oxide/Cargo.toml` has no `[features]` section, no
  serde dependency). Deserialize then errors if `num_items > 0`.
- **`usize::to_le_bytes` in wire formats** (space_saving, count_min) makes formats
  depend on pointer width.
- No serde support, no Apache DataSketches binary compatibility (roadmap flags DS-compat
  as future work, but the *internal* format-stability substrate it would build on
  doesn't exist either).

**Why it matters.** For 2026 SOTA, sketches are long-lived artifacts (stored in
Parquet/DB columns, shipped between services). Unversioned, panicking, partially-lossy
serialization is disqualifying for 1.0; `Sketch::serialize` returning `Vec<u8>`
infallibly while an impl silently discards data is an API-contract violation today.

**Recommendation.**

- (S) Fix `SpaceSaving::serialize` to error or actually serialize; remove references to
  the nonexistent serde feature.
- (M) Introduce a tiny shared framing layer in `common`:
  `[magic:2][sketch-id:1][format-version:1][payload]` + a `ReadCursor` helper that
  returns `Err(Deserialization)` instead of panicking; migrate the 31 existing formats;
  fuzz `deserialize` (cargo-fuzz) as a CI gate.
- (M) Add an optional `serde` feature (derive on the state structs).
- (L) DataSketches-compatible codecs for Theta/KLL/CPC behind a `datasketches-compat`
  feature (already on roadmap; note it's blocked on F1's canonical hashing).

---

## F3. The `Sketch` trait doesn't fit most sketches — and most sketches don't implement it

**Current state.** `common/traits.rs:16-62` defines
`Sketch { type Item; update(&mut self, &Item); estimate(&self) -> f64; is_empty(); serialize() -> Vec<u8>; deserialize(...) }`,
with `Mergeable: Sketch` (`traits.rs:73-89`). Evidence of misfit:

- Only **21 of 211 public structs** implement `Sketch`; 20 implement `Mergeable`; whole
  modules (`privacy`, `learned`, `membership` filters, `sampling`, `reconciliation`,
  most of `net`/`statistics`/`matrix`) implement neither. Meanwhile 32 files have an
  *inherent* `pub fn merge` outside the trait — so
  `WindowedAggregator<S: Mergeable + Clone>` (`streaming/windowed_aggregator.rs:84`),
  the roadmap's flagship substrate, can only wrap ~20 types.
- `estimate(&self) -> f64` is meaningless for point-query sketches:
  `impl Sketch for CountMinSketch` returns a documented placeholder —
  `fn estimate(&self) -> f64 { 0.0 }` ("This is a placeholder to satisfy the Sketch
  trait", `frequency/count_min.rs:~300`).
- `impl Sketch for BinaryFuseFilter::update` **panics unconditionally** ("BinaryFuseFilter
  is immutable", `membership/binary_fuse.rs:406-410`) — a Liskov violation inside a
  trait whose doc block advertises Liskov substitution.
- `impl Sketch for KllSketch` (`quantiles/kll.rs:484-505`) **clones the entire sketch on
  every `estimate()` and `serialize()`** because `quantile()`/`to_bytes()` take
  `&mut self` (lazy compaction). Hidden O(n) allocation on a read path.
- `type Item = u64` on HLL/CountMin while the real inherent API is `update<T: Hash>` —
  the trait erases the library's actual genericity.
- Bundling `serialize`/`deserialize` into `Sketch` forces types without a format (or
  with fallible/lossy formats) to lie (F2).

**Why it matters.** The trait is the library's composition backbone (windowed
aggregation, NitroSketch<S: Sketch>, future DP wrappers per the roadmap). Today it is
simultaneously too narrow (one `f64` estimate), too broad (mandatory infallible
serialization), and under-adopted (10%), so generic infrastructure only benefits a tenth
of the library.

**Recommendation (M–L, pre-1.0 breaking window).** Split into capability traits,
DataSketches-style:

- `Sketch` (marker + `is_empty`), `Update<T>` (or `StreamUpdate { type Item }`),
  `CardinalityEstimate`, `PointQuery<T, Output>`, `QuantileQuery`, `Filter`
  (contains/insert), `Serializable { fn serialize(&self) -> Result<Vec<u8>>; }` — all
  separate; keep `Mergeable` but require it not extend the full `Sketch`.
- Fix `&mut self`-on-read by finalizing internally (interior compaction flag or a
  documented `finalize(&mut self)` API) rather than cloning.
- Then implement the applicable traits across all 16 modules and add trait-driven
  property tests (merge associativity etc., as roadmap cross-cutting item 1 already
  envisions — but it needs this refactor first to be meaningful).

---

## F4. API-naming and signature inconsistency across (and within) modules

**Current state.**

- Ingest verb counts across `src`: `update` (74), `add` (34), `insert` (59), `push` (2).
  Even *within `cardinality/`*: `HyperLogLog::update<T: Hash>`, `HllPlus::add(&[u8])`,
  `KMV::add<T: Hash>`, `LinearCounting::add(u32) -> Result<()>`,
  `ThetaSketch::update<T: Hash>`.
- Query verbs: `estimate` (86), `query` (15), `count` (35), `frequency` (3),
  `observe`/`submit`/`release`/`privatize` in `privacy/`.
- Quantile API is a zoo (`grep pub fn quantile quantiles/*.rs`): returns `Option<f64>`
  (gk, otel, ddsketch, req, udd, moments), bare `f64` (tdigest — and takes `&mut self`),
  `u64` (dyadic_count_sketch; spline_sketch names it `query`), `Option<u64>` (q_digest,
  `&mut self`). `KllSketch::quantile(&mut self, rank: f64)` names the argument `rank`
  while others use `phi`/`q`.
- Item-type inconsistency: `&[u8]` keys (BloomFilter `membership/bloom.rs:123`),
  `T: Hash` (CountMin/HLL/Theta), `f64` only (KLL — not generic over ordered `T` unlike
  DataSketches KLL<T>), `u32` (LinearCounting).
- Constructor fallibility is inconsistent: 149 of 179 `pub fn new(` return `Result`; the
  rest return `Self` (e.g., `BloomFilter::new` — no validation path), and
  `BloomFilter::from_bytes` returns `Result<Self, &'static str>`
  (`membership/bloom.rs:210`) — a different error type than everything else.
- Merge inconsistency: `BloomFilter::merge` **panics** on size mismatch via `assert_eq!`
  (`membership/bloom.rs:266-272`) while `Mergeable::merge` returns
  `Result<(), SketchError>`; Theta uses out-of-place
  `union/intersect/difference -> Result<Self>`.

**Why it matters.** Users (and the 4 FFI layers) must memorize per-algorithm APIs;
binding code multiplies each inconsistency by four languages. This is the #1 "polish"
gap vs DataSketches, whose family-level API uniformity is a key adoption driver.

**Recommendation (M).** Publish an API-convention document and enforce it in a sweep
before 1.0: `update` for streaming ingest everywhere (keep `insert`/`contains` for
filters only), `estimate`/`quantile(rank: f64) -> Option<f64>` uniformly, all
constructors `-> Result<Self, SketchError>`, all merges `-> Result<(), SketchError>`,
`SketchError` as the sole error type. Deprecation aliases for one 0.x cycle (pattern
already proven with `RatelessIBLT`).

---

## F5. Error handling: 149 non-test `unwrap()`s, panicking merges, and a String-only error enum

**Current state.**

- Non-test, non-comment counts: **149 `.unwrap()`, 19 `.expect()`, 1 `panic!`**.
  Hotspots: `quantiles/ddsketch.rs` (17), `quantiles/kll.rs` (12), mostly (a)
  unchecked-slice deserialization (F2) and (b)
  `sort_by(|a,b| a.partial_cmp(b).unwrap())` on `f64` (`kll.rs:226,255,310,383`) — the
  latter panics if a NaN was ever ingested, and `KllSketch::update(f64)` does not reject
  NaN.
- The intentional panic and panicking `assert_eq!` merge noted in F3/F4.
- `SketchError` (`common/error.rs:6-35`): reasonable shape, but (a) **not
  `#[non_exhaustive]`** — adding any variant post-1.0 is semver-breaking; (b) String
  payloads allocate on every error and are awkward to map to stable FFI error codes;
  (c) no `Unsupported`/`InvalidState` variants, which is why Bloom fell back to
  `&'static str` and BinaryFuse to `panic!`.

**Why it matters.** Panics in a library embedded via JNI/PyO3/napi abort or throw opaque
errors across the FFI boundary; NaN-triggered panics deep inside compaction are
effectively data-dependent crashes.

**Recommendation.** (S) `#[non_exhaustive]` on `SketchError` + add
`Unsupported { op: &'static str }` and use it for BinaryFuse/SpaceSaving; (S) replace
`partial_cmp().unwrap()` with `f64::total_cmp` and either reject or document NaN at
`update`; (M) audit the 149 unwraps — the deserialization ones fall out of the F2
cursor helper; (S) fix `BloomFilter::from_bytes` and `merge` signatures.

---

## F6. No feature flags, no `no_std`, no MSRV, no semver tooling

**Current state.** `sketch_oxide/Cargo.toml` has **no `[features]` section at all**;
`twox-hash`, `xxhash-rust`, `siphasher`, `rand` are all unconditional. No `rust-version`
in either Cargo.toml; `rust-toolchain.toml` pins `channel = "stable"` (a moving target —
builds are not reproducible over time and there is no MSRV promise). No `#![no_std]`
anywhere (`std::collections::HashMap` used freely, e.g. DDSketch bins). CI has no
cargo-semver-checks, no MSRV job, no public-API diff. Positives worth keeping: workspace
lint table denies `todo!/unimplemented!/dbg!`, clippy is **clean (0 warnings)**,
`#![warn(missing_docs)]` is on and `cargo doc --no-deps` emits only 23 cosmetic warnings
(bare URLs, e.g. `universal/univmon.rs:47`), and `deny.toml`/`clippy.toml`/`rustfmt.toml`
exist.

**Why it matters.** 2026-standard Rust libraries declare MSRV, gate deps behind features
(a sketch library's core is `alloc`-friendly by nature — HLL/KLL/Bloom need no `std`),
and the roadmap itself lists `no_std` for LDP clients/edge as a requirement for *new*
code — but there's no feature scaffolding for new code to slot into. `rand` is only
needed by sampling/privacy; SIMD work (roadmap item 7) will also need feature flags.

**Recommendation.** (S) Add `rust-version` + MSRV CI job + cargo-semver-checks job.
(M) Introduce `default = ["std"]`, `std`, `rand` (sampling/privacy), later `serde`,
`simd`; make `common` + hash-only sketches `no_std + alloc` clean as the pilot.

---

## F7. `unsafe`: small and localized — acceptable, one policy inconsistency

**Current state.** 11 `unsafe` sites in 5 files (`ultraloglog.rs:276,284`,
`bloom.rs:135,160`, `count_sketch.rs:191,258,317`, `count_min.rs:176,234`,
`removable_sketch.rs:111`) — all `get_unchecked` after power-of-two masking, each with a
`// SAFETY:` comment. The workspace sets `unsafe_code = "warn"` yet these compile
(presumably `#[allow]`ed locally). FFI crates (`java/src/lib.rs` raw-pointer deref) are
inherently unsafe and out of scope.

**Why it matters.** Low risk, but "warn-and-allow" is not a policy. The masks make these
provably in-bounds; the interesting question is whether `get_unchecked` even wins after
the bounds check is eliminated by the mask.

**Recommendation.** (S) Either demonstrate the win with a criterion diff and document a
`SAFETY` policy (`#![deny(unsafe_op_in_unsafe_fn)]`, forbid in all modules except a
listed few), or drop to safe indexing. Add `#![forbid(unsafe_code)]` on modules that
need none.

---

## F8. Public-API hygiene gaps for 1.0

**Current state (grep evidence).**

- `#[non_exhaustive]`: **0 uses** (error enum, `Stats` structs,
  `TimeDomain`/`LateDataPolicy` enums in `common/time.rs` all evolvable).
- `#[must_use]`: **0 uses** — `estimate()`, `quantile()`, `contains()`,
  `merge()->Result` all silently droppable.
- **63 public fields** on ~15 structs — mostly `*Stats` snapshots (acceptable) but
  including `vacuum_filter.rs:199-207` and `nitrosketch.rs:71-73` where fields mirror
  live state.
- No sealed traits: `Summary` (`cardinality/theta_core.rs:22`), `Sketch`, `Mergeable`
  are all open — any future added method breaks downstream impls.
- No prelude; `lib.rs:39-53` re-exports a hand-picked, **stale** subset (none of the
  2026 modules — `privacy`, `learned`, `matrix`, `statistics`, `net` types are absent
  from root re-exports; `KllSketch` is exported but `ReqSketch`/`DDSketch` are not,
  despite DDSketch being in the crate description).
- `impl Extend`/`FromIterator`: 0 — no idiomatic bulk construction
  (`BinaryFuseFilter::from_items` exists ad hoc).
- Send/Sync: fine by construction (zero `Rc`/`RefCell`/`Mutex`; `SmallRng` fields in
  `cvm.rs:43`, `qsketch.rs:113` are `Send`). Not documented or statically asserted
  anywhere — one `static_assertions` test would lock it in.
- Doc-comment claims drift: `lib.rs:1-4` says "State-of-the-Art (2025)"; space_saving
  references a nonexistent serde feature (F2).

**Recommendation.** (S) `#[non_exhaustive]` on `SketchError` + enums + Stats structs;
`#[must_use]` on query/merge methods (one clippy-assisted sweep); static assertions for
`Send + Sync` on the top-20 types. (S) Refresh root re-exports/prelude to cover all 16
modules. (M) Seal `Summary` and any trait not meant for user impls.

---

## F9. Merge & construction ergonomics: no builders, no batch APIs, runtime-only params

**Current state.**

- **Builders: 0** (`grep "struct.*Builder"` → nothing). Multi-parameter sketches use
  positional `new(a, b, c)` (e.g. `Iblt::new(100, 32)`,
  `BloomFilter::with_params(n, m, k)`); adding a `Salt` or HIP toggle to constructors
  (F1) will make positional args worse.
- **Batch update APIs in core: 1** hit for `update_batch|update_many|extend` — batching
  exists only in the Python binding (`python/src/count_min.rs:163`), meaning every
  binding re-implements batching by looping over FFI calls; the core has no
  `update_batch(&mut self, items: impl IntoIterator)` for amortizing hashing/dispatch.
- **Const generics: 0** — everything is runtime-parameterized (fine as a default for
  FFI, but there's no zero-cost path for embedded/hot-loop users, e.g.
  `BloomFilter<const K: usize>`).
- Mergeable coverage: 20 trait impls vs 32 inherent `merge` fns vs ~180 algorithm types;
  anything without `Mergeable` is invisible to `WindowedAggregator` and to future
  DP/decay wrappers (see F3).

**Recommendation.** (M) Add `update_batch`/`Extend` on the hot sketches (also unlocks a
real napi/pyo3 batch fast path); (M) a light builder pattern only where ≥3 params or
optional salt/HIP/seed exist
(`HyperLogLog::builder().precision(12).salt(s).hip(true).build()?`); (L, optional)
const-generic variants only if embedded targets become real.

---

## F10. FFI architecture asymmetry (observation, briefly)

Java is one hand-written 4,110-line `lib.rs`, Node one 6,461-line `lib.rs`, Python 184
modular files — three divergent hand-maintained surfaces plus .NET, with per-binding
item-conversion policies (root cause of F1). Java errors are silently swallowed
(`if let Ok(bytes) = env.convert_byte_array(arr)` — `java/src/lib.rs:78`, dropped on
error). The roadmap already proposes a shared C-ABI surface; this audit's data (naming
inconsistency ×4 languages, hash policy divergence) quantifies why that should be pulled
*earlier* than "follows by one wave." Effort: L.

---

## Ranked recommendation summary

| # | Recommendation | Fixes | Effort | Impact |
|---|---|---|---|---|
| 1 | Canonical item-encoding + stable hash spec, adopted by core + all 4 bindings; golden hash vectors in CI | F1 | M–L | Correctness of the library's headline promise (cross-language merge) |
| 2 | Serialization substrate: magic/version framing, panic-free cursor, fix SpaceSaving silent loss, fuzz deserializers | F2, F5 | M | Data safety, untrusted-input robustness, 1.0 format stability |
| 3 | Trait split (`Update`/`Estimate`/`Query`/`Serializable` capabilities), retire placeholder/panicking impls, then drive adoption from 21→all types | F3 | L | Unlocks windowed/DP/decay wrappers for the whole library — prerequisite for roadmap Wave 1.0 items |
| 4 | API-convention sweep (verbs, `Result` constructors, quantile signature, single error type) with deprecation aliases | F4 | M | Usability, FFI simplification, DataSketches-grade polish |
| 5 | Error hardening: `#[non_exhaustive]`, `total_cmp` for NaN, unwrap audit, `Unsupported` variant | F5, F8 | S | Crash-freedom in embedded/FFI contexts |
| 6 | MSRV + `[features]` (`std`, `rand`, `serde`, `simd`) + cargo-semver-checks CI; pilot `no_std+alloc` for `common` | F6 | S–M | 2026 ecosystem table stakes |
| 7 | Hygiene sweep: `#[must_use]`, sealed `Summary`, Send/Sync assertions, refresh stale root re-exports (2026 modules missing) | F8 | S | Semver readiness |
| 8 | Core `update_batch`/`Extend` + selective builders (salt/HIP-ready) | F9 | M | Throughput via FFI, ergonomics |
| 9 | Document-or-drop `get_unchecked` policy | F7 | S | Auditability |
| 10 | Accelerate shared C-ABI binding layer | F10 | L | Maintenance of 4 bindings |

**Positives to preserve:** clippy-clean with a curated deny-list, `missing_docs`
enforced (~23 cosmetic doc warnings only), zero interior-mutability (auto Send/Sync),
`deny.toml` supply-chain config, disciplined `// SAFETY:` comments, the
well-designed-but-unused `Salt` API, `common/time.rs` watermark/late-data types, and the
`RatelessIBLT` deprecation-alias pattern as the template for the F4 renames.
