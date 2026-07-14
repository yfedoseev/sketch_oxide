# sketch_oxide → SOTA 2026: Executive Summary & Prioritized Improvement Plan

> `docs/research/fable5/` — improvement research, compiled 2026-07-01 on branch
> `releases/v0.2.0` (HEAD `51cba1d`). Produced by five parallel deep audits (two
> web-research, three codebase); each has its own detailed document:
>
> | Doc | Scope |
> |---|---|
> | [01-architecture-api-design.md](01-architecture-api-design.md) | Rust core architecture, trait design, API consistency, error handling |
> | [02-performance-simd.md](02-performance-simd.md) | Hot paths, SIMD, hashing, memory layout, FFI batch APIs, bench gates |
> | [03-sota-algorithms-2026.md](03-sota-algorithms-2026.md) | New algorithms/trends beyond the internal roadmap; competitive landscape |
> | [04-bindings-packaging-supply-chain.md](04-bindings-packaging-supply-chain.md) | PyO3/napi/JNI/.NET modernization, wheels, trusted publishing, MSRV/edition |
> | [05-testing-ci-correctness.md](05-testing-ci-correctness.md) | Fuzzing, statistical validation, cross-language conformance, CI/publish pipeline |
>
> **Relationship to prior research:** `docs/sandbox/research/` (2026-06-09) already
> answers "*which algorithms* are missing" (~165 items, phased roadmap). This series
> answers the complementary question: "*is the engineering underneath* those algorithms
> at a 2026 state-of-the-art bar?" Short answer: not yet — and several of the gaps below
> undermine the algorithm roadmap itself if not fixed first.

---

## Where the project stands

**Genuinely strong:** ~190 algorithms across 16 modules (broadest coverage in any
language ecosystem), full Python binding coverage (184 classes), clippy-clean with a
curated lint deny-list, `missing_docs` enforced, ~2,400 tests + gated doctests, 30
Criterion bench files including competitor comparisons, disciplined `// SAFETY:`
comments, an excellent internal algorithm-research corpus, and — per the external
survey — the **only maintained KLL/REQ in Rust** and a real head start on temporal /
reconciliation / privacy / range-filter domains no competitor touches.

**The competitive clock:** Apache DataSketches went polyglot in 2025–26 (official Rust
port at 0.3.0, Go port, Spark/Snowflake/BigQuery integrations). Its Rust port is still
narrow (no KLL/REQ/Tuple/sampling). sketch_oxide's window is to convert breadth +
bindings + interop into adoption **before datasketches-rust matures**.

---

## The five findings that matter most (cross-cutting verdict)

These emerged independently from multiple audits and gate everything else:

### 1. Cross-language hash incompatibility — the headline promise is currently broken

The same logical item hashes differently depending on which binding ingested it
(Rust `str::Hash` appends 0xFF; `[u8]::Hash` prepends a native-endian length; Python
tries `i64→u64→String→bytes` while Java/Node always send bytes). **Merging a
Python-built and Java-built sketch over identical data silently double-counts.** No
conformance test catches it because none exists. Fix: a documented canonical item
encoding + stable hash spec, adopted by core and all four bindings, locked by golden
hash vectors in CI. *(Details: doc 01 F1; test gap: doc 05 §2.)*

### 2. Serialization is unsafe, unversioned, and 84% absent

Only 31 of 188 algorithm files serialize at all (ThetaSketch — the set-ops flagship —
cannot). No magic bytes, no version byte, no endianness spec; `usize` in wire formats.
Concrete attacker-triggerable bugs found: integer-overflow length-check bypass in
`spline_sketch.rs` deserialize (remote DoS through PyO3/napi), unchecked-slice panics in
DDSketch/KLL, wrap-around in BinaryFuse, and `SpaceSaving::serialize` that **silently
drops all counters** referencing a serde feature that doesn't exist. Fix: shared
`[magic][id][version]` framing + panic-free cursor in `common`, checked arithmetic,
cargo-fuzz targets per deserializer, `overflow-checks` in release. *(Docs 01 F2, 05 §1–2.)*

### 3. The `Sketch`/`Mergeable` trait backbone covers ~10% of the library

21 of 211 public structs implement `Sketch`; 20 implement `Mergeable` — while 32 types
have ad-hoc inherent `merge()`. The trait itself misfits (placeholder `estimate() → 0.0`
in CountMin, unconditional panic in BinaryFuse's `update`, full-clone-per-read in KLL).
This directly blocks the internal roadmap's flagship substrate
(`WindowedAggregator<S: Mergeable>`, DP wrappers, decay frameworks). Fix: split into
capability traits (Update/Estimate/Query/Filter/Serializable), then drive adoption
across all 16 modules — **before** the Phase-1 substrate wave, not after. *(Doc 01 F3.)*

### 4. Statistical guarantees are asserted, not tested

Accuracy tests are single fixed-seed runs with tolerances 3–10× looser than theory
(HLL p=12 tested at 5–15% error where theoretical SE is 1.6%). A subtle estimator-bias
regression would pass CI. For a library whose product *is* error bounds, a Monte-Carlo
statistical harness (N seeded trials, assert error distribution vs paper bound) is the
core credibility feature — and the internal roadmap already specifies it (testing
item 2) but nothing implements it. *(Doc 05 §2–3.)*

### 5. The release pipeline is running on dead or dying infrastructure

npm classic tokens were revoked Dec 2025 (post-Shai-Hulud) — **the next npm publish may
simply fail**; the Maven job targets OSSRH, which shut down June 2025; PyPI uses a raw
token with no attestations; publish jobs don't depend on tests being green; Linux wheels
are likely not manylinux-tagged; no aarch64/musl wheels; npm ships no prebuilt binaries
for consumers off the publisher's platform. PyO3 is 7 versions behind (0.22 vs 0.29) and
the Python 3.8 floor is un-buildable on modern PyO3. *(Doc 04.)*

---

## Prioritized plan

### Phase A — Stop-the-bleeding (days, all S effort)

| # | Action | Source |
|---|---|---|
| A1 | Fix `spline_sketch`/`binary_fuse` deserialize overflow bugs; add `overflow-checks = true` | 05 |
| A2 | Fix `SpaceSaving::serialize` silent data loss; remove phantom serde-feature references | 01 |
| A3 | Switch all registries to trusted publishing (OIDC); make publish depend on green tests | 04 |
| A4 | Add `[profile.release]` `lto="fat"`, `codegen-units=1` (5–20% free perf) | 02 |
| A5 | CI: add `releases/**` triggers; un-soft-fail mypy/tsc; cache@v3→v4; debug-assertions test lane | 05 |
| A6 | `#[non_exhaustive]` on `SketchError` + enums; `total_cmp` for NaN sorts in KLL/DDSketch | 01 |
| A7 | Fix doc drift: rename `murmur3_hash64` shim; mark OTel-histogram P0 done in gap analysis; document the two ExponentialHistogram types | 01, 03 |

### Phase B — Foundations that gate the algorithm roadmap (weeks, M)

| # | Action | Source |
|---|---|---|
| B1 | **Canonical item-encoding + stable hash spec** across core + 4 bindings, golden hash vectors | 01 |
| B2 | **Serialization framing layer** (`magic/id/version` + panic-free cursor); migrate 31 formats; cargo-fuzz per deserializer in CI | 01, 05 |
| B3 | **Capability-trait split** and library-wide Mergeable adoption (unblocks WindowedAggregator/DP/decay waves) | 01 |
| B4 | **Statistical CI harness** (multi-seed error-bound validation vs theory) + golden-file cross-language conformance suite | 05 |
| B5 | Hot-path fixes: Count-Min K-M derivation + prefetch; MinHash hash-once (kills 128 allocs/update); BlockedBloom hash-once; HLL `2^-r` LUT | 02 |
| B6 | PyO3 0.22→0.29, Python floor →3.10, manylinux_2_28 + aarch64/musl wheels + sdist; real type stubs (pyo3-stub-gen); napi-rs v3 + per-platform npm packages + WASM fallback | 04 |
| B7 | MSRV + edition 2024 + `[features]` (`std`/`rand`/`serde`/`simd`) + cargo-semver-checks | 01, 04 |

### Phase C — Differentiation (quarters, M–L)

| # | Action | Source |
|---|---|---|
| C1 | **DataSketches binary compat for Theta (Puffin `apache-datasketches-theta-v1`)** + iceberg-rust example — time-sensitive land-grab | 03 |
| C2 | **Extended RaBitQ** vector-quantization module (strongest new algorithm candidate; no Rust crate exists) | 03 |
| C3 | FFI batch APIs everywhere (numpy arrays + `allow_threads`, typed arrays in Node) + core `update_batch`/`Extend` | 02, 01 |
| C4 | SIMD kernels behind a `simd` feature (Bloom probes, MinHash mins, HLL scan, VQF) + optional `rayon`/concurrent sketches | 02 |
| C5 | LLM-dedup product story: datatrove-compatible MinHash preset, Milvus BYO-signature export, LSHBloom, BFF-compatible Bloom | 03 |
| C6 | OTLP protobuf encoding + Prometheus native-histogram converter; DataFusion UDAF pack for Arroyo/RisingWave-class engines | 03 |
| C7 | `no_std`/eBPF (Aya-safe) core feature — first-mover, serves edge + LDP clients | 03, 01 |
| C8 | API-convention sweep (verbs, Result constructors, unified quantile signature) with deprecation aliases; shared C-ABI layer for Java/.NET revival (floor 17 / net8.0, `[LibraryImport]`, Central Portal publish) | 01, 04 |

### Adjustments to the existing algorithm roadmap (from the external survey)

- Promote **Sublime** (SIGMOD 2026) from P2-watch; evaluate **Breadcrumb Filters**
  before committing the CQF build and **Aeris** before the Memento/Diva sequencing.
- Fold **banded matrix-factorization DP continual counting** and **DP Misra-Gries**
  into the Phase-1 privacy wave (production-proven at Google; ~1–2/5 effort each).
- ExaLogLog remains unshipped anywhere — a Rust implementation would be the world's
  first production one.

---

## Definition of "SOTA 2026" for this project

The internal roadmap's Definition of Done covers algorithm breadth. This series adds
the engineering half; the library can credibly claim SOTA when:

1. One documented hash/encoding spec, provably identical across 5 languages (golden
   vectors in CI).
2. Every serializable sketch has a versioned, fuzzed, panic-free wire format — and
   every *mergeable* sketch is serializable.
3. Error bounds are continuously validated against theory in CI, not asserted in
   README badges.
4. Trusted publishing + attestations on all registries; wheels/prebuilds for the real
   2026 platform matrix; PyO3/napi on current majors.
5. The capability-trait layer covers the whole library, so windowed/DP/decay/learned
   wrappers compose over all of it.
6. Hot paths are within ~2× of the best native implementations (SIMD where it counts,
   zero allocations per update, batch FFI), enforced by a benchmark regression gate.
