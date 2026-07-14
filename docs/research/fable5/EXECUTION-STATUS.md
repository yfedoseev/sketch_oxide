# fable5 Execution Status

> Progress on executing the `docs/research/fable5/` improvement plan, with tests.
> Branch `releases/v0.2.0`. Each item notes verification status.

## Completed & verified (Rust core)

### Phase A — stop-the-bleeding
- **Deserialization DoS fixes** (doc 05 #1):
  - `spline_sketch::deserialize` — checked arithmetic; a crafted `sample_count`
    near `u64::MAX` no longer overflows past the length check into a giant
    `Vec::with_capacity`. Tests: overflow-rejected, truncation-rejected, round-trip.
  - `binary_fuse::deserialize` — checked `segment_length * segment_count`;
    dropped the `size > 0` escape hatch that let mismatched arrays through.
    Tests: wrapping-dims-rejected, empty + populated round-trip.
  - `ddsketch::deserialize` and `kll::from_bytes` — rewritten on the panic-free
    `ReadCursor` (below); attacker-controlled bin/level counts validated against
    remaining bytes. Tests: oversized-count-rejected, truncation-rejected, round-trip.
  - `count_min::deserialize` — width/depth validated on the full `usize` value
    *before* the lossy `as u32` cast (a `2^32+5` width truncated to a small u32
    that passed validation, then drove a huge table size); table size via
    `checked_mul`. Test: truncating-dims-rejected + truncated-header.
- **Release profile** (doc 02 #1, doc 05 #1): workspace `[profile.release]`
  `lto = "fat"`, `codegen-units = 1`, `overflow-checks = true`; matching `[profile.bench]`.
- **SpaceSaving serialize honesty** (doc 01 F2): removed references to a
  nonexistent "serde feature"; `deserialize` of populated counters now returns a
  typed `Unsupported` error (full encoding lands with B1 canonical bytes).
- **Error enum hardening** (doc 01 F5/F8): `SketchError` is `#[non_exhaustive]`
  with new `Unsupported { op }` and `InvalidState { reason }` variants.
- **NaN-safe sorts** (doc 01 F5): KLL sorts use `f64::total_cmp` (update already
  rejects non-finite input).
- **Hash shim** (doc 01 F1): `murmur3_hash64` (which was never MurmurHash3)
  `#[deprecated]` toward `hash_64`/`xxhash`; honest docs.
- **Enum/API hygiene** (doc 01 F8): `#[non_exhaustive]` on `TimeDomain` /
  `LateDataPolicy`; `#[must_use]` on `Sketch::{estimate,is_empty,serialize}`.
- **Doc drift** (doc 03): `lib.rs` header 2025→2026; documented the two distinct
  `ExponentialHistogram` types (DGIM vs OTel); marked OTel histogram done in the
  master gap analysis.
- **Stale root re-exports** (doc 01 F8): added `DDSketch`/`ReqSketch` and
  representative types from the 2026 modules (privacy/learned/matrix/statistics).

### Phase B foundations
- **B2 serialization framing + panic-free cursor — COMPLETE across all deserializers**:
  new `common::cursor` — `ReadCursor` (bounds-checked reads that error instead of
  panicking), `WriteBuf`, `Framing` (`[magic:2][sketch-id:1][version:1]`),
  `SketchId` registry. **Every hand-rolled `deserialize`/`from_bytes` in the crate
  is now panic/OOM/abort-safe against malicious bytes** — checked size arithmetic,
  dimension/precision bounds before shifts and multiplies, and count-vs-remaining
  validation before every `with_capacity`. Hardened: ddsketch, kll, spline,
  binary_fuse, count_min, conservative_count_min, count_sketch, elastic_sketch,
  otel_histogram, ribbon, qsketch, cpc, ultraloglog, minhash, ams, tcm,
  udd_sketch, bloom, cuckoo, counting_bloom, stable_bloom, exponential_histogram,
  sliding_window (already-safe verified: nitrosketch, sliding_hll, tdigest,
  univmon). Extra bugs caught & fixed along the way: `cpc` shift-overflow abort
  (`1<<lg_k`, lg_k≤255), `stable_bloom` division-by-zero (`counter_bits=0`),
  `udd_sketch` unbounded collapse-loop CPU-DoS + OOB bucket read, and a
  tiny-epsilon `count_sketch::new` overflow (`3/ε²`→usize::MAX→panic). Each
  hardened file has a malformed-input regression test + a round-trip test. Total
  suite now 1327 lib tests, all green.
- **B1 canonical hash substrate** (doc 01 F1): new `common::canonical` —
  `CanonicalEncode` trait (LE fixed-width ints, raw-UTF-8 strings, verbatim
  bytes), `stable_hash` at a fixed seed. Golden-vector + golden-hash tests; the
  headline invariant `stable_hash("abc") == stable_hash("abc".as_bytes())` is
  asserted (the exact cross-language double-count case).
- **B4 statistical CI harness** (doc 05 #3): `tests/statistical_validation.rs` —
  N seeded trials asserting HLL RMS relative error ≤ 1.4× theory and near-zero
  bias, Bloom measured FPP ≤ 1.6× configured, DDSketch relative error ≤ 2× alpha.
  Far tighter than the old 5–15% single-seed smoke checks.
- **B5 hot paths**:
  - HLL `2^-r` via direct IEEE-754 exponent construction (`inv_pow2`), replacing
    per-register `powi` libm calls; bit-identical to `powi` (verified by test).
  - BlockedBloom: hash-once + Kirsch–Mitzenmacher bit derivation + Lemire
    fast-range block index; removed the per-insert `Vec<usize>` allocation and
    the k+1 re-hashes.
  - CountMin: replaced the serial xxh64-finalizer chain with K–M double hashing
    (hash once). Never-underestimate guarantee + round-trip verified by test.
  - MinHash: hash the item ONCE (streaming, zero-alloc) then derive all `num_perm`
    values via the murmur3 `fmix64` bijection, replacing 128 heap allocations +
    128 full-key hashes per `update`. Wire format unchanged; Jaccard accuracy
    preserved (verified by a known-overlap test). *(doc 02 #4, the 20–100× win.)*
- **Batch ingest** (doc 01 F8/F9, doc 02 #5): `common::batch` adds `Extend` for
  HLL/CountMin/MinHash/DDSketch/KLL (the crate previously had zero `Extend` impls),
  giving idiomatic bulk construction and a single bottom-out point for a binding
  batch fast path.

### Phase B tooling
- **MSRV**: `rust-version = "1.87"` (workspace + member opt-in); CI MSRV build job.

## Completed & verified (CI / packaging)
- **A3 trusted publishing** (doc 04 #1, doc 05 #4): `publish.yml` rewritten — a
  `test-gate` job (fmt+clippy+tests) that every publish job `needs:`; crates.io
  OIDC (`crates-io-auth-action`), PyPI trusted publishing + attestations
  (`gh-action-pypi-publish`), npm OIDC + `--provenance`. maturin-action wheels
  with manylinux_2_28 + aarch64 + musllinux + sdist. Maven/NuGet left disabled
  with migration notes (OSSRH sunset, `[LibraryImport]`).
- **A5 CI hygiene** (doc 05): `releases/**` triggers; `actions/cache@v3→v4`;
  `setup-python@v4→v5`; un-soft-failed mypy and `tsc`; added a
  release+debug-assertions+overflow-checks test lane.
- **Supply chain**: OSSF Scorecard workflow; `cargo-semver-checks` + MSRV jobs.
- **Fuzzing** (doc 05 #4): `sketch_oxide/fuzz/` with libfuzzer targets for the
  DDSketch / spline / binary-fuse / KLL deserializers.

## Also done — B3 capability-trait layer + FULL adoption
- **B3 capability traits** (doc 01 F3): new `common::capabilities` —
  `Update<T>`, `CardinalityEstimate`, `PointQuery<T>`, `Filter<T>`,
  `QuantileQuery`, and a *fallible* `Serializable`. Additive (existing
  `Sketch`/`Mergeable` untouched). Resolves the concrete misfits: Count-Min
  exposes a real `PointQuery` (not the `estimate()→0.0` placeholder); Binary Fuse
  implements `Filter` but **not** `Update` (immutability = compile error, not a
  runtime panic); `SpaceSaving` gets an honest fallible `Serializable`.
- **Full adoption** across the cardinality / frequency / quantiles / membership /
  similarity / streaming / universal families (~90 impls over ~80 files, driven by
  5 parallel agents), each delegating to existing methods with per-module smoke
  tests. Types whose semantics don't fit a capability (keyed/weighted/`&mut`-query)
  were deliberately skipped with comments. Integration needed only 3 fixes
  (a `to_bytes` name-collision, a `Sketch`/`Update` call ambiguity, and a
  `blocked_bloom::from_bytes` overflow found along the way).

## Also done — features + more hot paths + Phase-C increments
- **Feature flags** (doc 01 F6): `[features]` `std`/`simd`/`serde` (the crate had
  none). `serde` derives + `serde_json` round-trip test on the `common` DTO types.
- **C4 SIMD** (doc 02 #2): `common::simd::count_zero_bytes` — an AVX2 kernel with
  runtime `is_x86_feature_detected` dispatch + scalar fallback, wired into HLL's
  empty-register count; an equivalence test proves SIMD == scalar (AVX2 path
  exercised on this host).
- **C3 batch ingest** (doc 01 F8/F9): `Extend` for HLL/CountMin/MinHash/DDSketch/KLL.
- **C5 LLM-dedup presets** (doc 03 C.4): `MinHash::datatrove()` (112-perm
  FineWeb/datatrove config) + `to_binary_vector()` signature export for Milvus
  BYO-signature / LSHBloom pipelines.
- **C2 Extended RaBitQ** (doc 03 B.1, the "strongest new candidate"): new
  `vector::RaBitQ` — 1-bit vector quantization with the unbiased inner-product
  estimator (random orthonormal rotation → per-dimension sign code + debias
  factor). Statistically validated: over 400 random 256-dim pairs the estimated
  inner product tracks truth at RMSE < 0.12 (theory ~1/√256 ≈ 0.06) with < 0.03
  bias, and L2 relative RMSE < 0.12 — a miscoded estimator would show O(1) error
  and fail. Dense-rotation reference impl; the paper's Fast-Hadamard transform is
  a drop-in speed follow-on that doesn't change the estimator.

## Remaining — each a standalone multi-day project (not one-session, verified)
These are breaking crate-wide migrations, new algorithms, or interop that can't be
verified in this environment. The config-level parts of B6 are DONE (above); what
remains is the code migration itself.
- **B6 PyO3 0.22→0.29 — DONE & runtime-verified.** Migrated all **184 bound
  classes** (163 → 0 compile errors via a handful of mechanical pattern fixes:
  `downcast`→`cast`, `*::new_bound`→`*::new`, `Python::with_gil`→`attach`,
  `PyObject`→`Py<PyAny>`, `assume_gil_acquired`→a `py` param, `PyList/PyTuple::new`
  now returning `Result`, `#[pyclass(from_py_object)]`). The wheel builds
  (abi3 ≥3.10, manylinux_2_34), imports, and **all 287 Python tests pass** at
  runtime on Python 3.14. numpy bumped 0.22→0.29 in lockstep.
- **B6 napi v2→v3 — DONE & runtime-verified.** Bumped `napi`/`napi-derive` to 3.10
  and `@napi-rs/cli` to v3; the `#[napi]`-style code needed **no source changes**
  (the only issue was a napi/napi-derive version skew, fixed by pinning napi 3.10.5).
  The `.node` addon builds (release) and **all 216 Node/jest tests pass** on Node 22.
- **B6 remainder** — generated Python type stubs (pyo3-stub-gen) + per-platform npm
  packages (the CI matrix from `publish.yml` already targets them); both are
  packaging polish, not code.
- **Phase C differentiation** (doc 00) — the still-remaining net-new / spec-gated
  items: DataSketches Theta **binary** compat (reverse-implement an external byte
  format — can't verify real interop here), DataFusion UDAF pack (new `datafusion`
  dep), no_std/eBPF (crate-wide, all-or-nothing). The **breaking** part of the
  API-convention sweep (200+ inherent renames + deprecation aliases) also remains;
  its non-breaking parts are done (capability-trait interface unification +
  `try_new`/`try_merge` `Result`-returning constructors/merge, doc 01 F4/F5).
  *(Done from Phase C: **RaBitQ (C2)**, batch `Extend` (C3), a SIMD kernel (C4),
  MinHash presets (C5), **OTLP protobuf + Prometheus (C6)**.)*
- **Full CanonicalEncode routing** across all sketches + 4 bindings — breaking,
  needs coordinated binding changes + a wire-format version bump. Substrate is done.

## Also done — C6 OTLP + Prometheus, B6 config, edition 2024
- **C6 OTLP protobuf + Prometheus converter** (doc 03 C.2): new `quantiles::otlp`
  — a hand-rolled OTLP `ExponentialHistogramDataPoint` protobuf **encoder + decoder**
  (no `prost`/`protoc` dep) using the standardized field numbers, proven correct by
  a lossless encode→decode round-trip + malformed-input rejection; plus
  `to_prometheus_native()` (spans + delta-encoded counts) proven lossless by
  reconstruction and count-preservation tests.
- **B6 config-level items** (doc 04, the parts not requiring the 184-class code
  migration): Python `requires-python` 3.8→**3.10** + classifiers, PyO3
  `abi3-py38`→**abi3-py310** (Python binding still builds), `py.typed` marker,
  Node `engines: {node: ">=20"}`.
- **Edition 2024** (doc 04 #5): migrated the core crate to `edition = "2024"` via
  `cargo fix --edition` + manual fixes (unsafe-op-in-unsafe-fn block in the SIMD
  kernel, a 2024 pattern-binding change); full suite green, clippy/fmt clean.

## Also done — ThetaSketch serialization (doc 01 F2)
- **ThetaSketch had no serialization at all** despite union/intersect/difference
  being its flagship feature. Added `to_bytes`/`from_bytes` (framed on the
  panic-free cursor) + the `Serializable` capability impl, plus a
  `ThetaCore::from_raw_parts` reconstruction path that restores `theta` exactly
  (so an at-capacity sketch round-trips to the same estimate). Tests cover the
  canonical serialize→deserialize→**union** workflow and malformed-input
  rejection. *(The DataSketches-compatible **binary** format (C1) — a different,
  externally-specified layout — remains; this closes the "Theta can't persist" gap.)*

## Final verification
1359 lib tests + doctests + integration binaries + the statistical / RaBitQ / OTLP /
Prometheus harnesses, all passing under `--all-features` on **edition 2024**;
`cargo clippy --lib --all-features` and `cargo fmt --check` clean; the Python binding
still builds on `abi3-py310`. ~90 capability adoptions + 12 new modules/test files.
Nothing committed (per the repo's git conventions).
