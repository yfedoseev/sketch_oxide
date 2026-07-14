# Testing / CI / Correctness Audit — Path to SOTA 2026

> Part of the `docs/research/fable5/` improvement research series (2026-07-01).
> All paths relative to the repository root.

**Bottom line:** the project has unusually broad example-based coverage and doctests are
properly gated, but it is missing all three pillars a 2026-grade probabilistic library
needs — **fuzzed untrusted deserialization** (one exploitable-for-DoS bug found during
this audit), **cross-language golden-file conformance** (currently zero), and
**statistical-bound testing** (currently single-trial smoke checks).

---

## 1. Current State (with evidence)

### CI on PR — `.github/workflows/test.yml`

- **Triggers:** push/PR to `main` and `develop` only (lines 3–7). The current release
  branch `releases/v0.2.0` gets **no CI on push**.
- **Rust job:** matrix = ubuntu(stable+beta), macos(stable), windows(stable). Runs
  `cargo test --verbose --all-features` in `./sketch_oxide` (line 65),
  `cargo test --doc` (line 74), and **benches are gated to compile-only**:
  `cargo bench --no-run` (line 78). No bench execution / perf regression tracking.
- **Python job:** 3 OS × Python 3.10/3.12/3.13; maturin wheel build +
  `pytest python/tests/`; benchmarks `continue-on-error: true`.
- **Node job:** 3 OS × Node 20/22; `npm test` + `tsc --noEmit`.
- **Java/C#: disabled** — comments at lines 185–189 ("Java FFI not actively
  maintained"), though `java/src/test` and `dotnet/SketchOxide.Tests/*.cs` exist and rot
  untested.
- **Quality job:** `cargo fmt --check`,
  `cargo clippy --lib --all-features -- -D warnings` (sketch_oxide crate only — the 4
  binding crates in the workspace are never clipped), `cargo doc` with
  `RUSTDOCFLAGS=-D warnings`, cargo-deny action (`deny.toml` present), black/ruff.
  **mypy and repeated tsc are soft-failed** with `|| true` (lines 237, 248).
- **Caching:** cargo registry/git/target cached, but inconsistently `actions/cache@v3`
  in the rust job vs `@v4` in python; target-dir cache key has no toolchain/job
  discriminator (cache thrash between stable/beta).
- **No** fuzz job, Miri, sanitizers, coverage, semver-check, cross-language conformance
  job, or scheduled (cron) workflow.

### Publishing — `.github/workflows/publish.yml`

- crates.io + PyPI (4 wheel targets: linux x86_64, mac x86_64/arm64, win x86_64 —
  **no aarch64-linux, no musl**) + npm. Maven/NuGet hard-disabled (`if: false`,
  lines 165, 199).
- **Publish jobs have no dependency on the test workflow** — a release publishes even if
  tests were never green on that ref. PyPI uses a raw token (no Trusted
  Publishing/OIDC), no artifact attestation. `npm publish` runs on ubuntu only
  (single-platform native module — napi packages normally need per-platform prebuilds).
  "Verify" job is cosmetic (`cargo search`, `pip index ... || echo`).

### Test quality (Rust core)

- Volume: 2,424 `#[test]` occurrences in `sketch_oxide/{src,tests}`; 31 integration test
  files.
- **Property-based:** `proptest = "1.4"` (workspace `Cargo.toml:63`); `proptest!`
  appears in 16 of 31 test files, but only **21 blocks total** — a tiny fraction (<1%)
  of tests, and the properties are shallow invariants (e.g. `ultraloglog_test.rs`:
  "estimate is non-negative"). Committed `.proptest-regressions` files show it has
  caught bugs (count_min, binary_fuse, ultraloglog, nitrosketch, learned_bloom).
- **Statistical guarantees:** accuracy tests are **single-trial, fixed-seed** regression
  checks, not statistical bound tests. `hyperloglog_test.rs:112–166` runs one
  deterministic stream (hash `XxHash64::with_seed(0)`, `hyperloglog.rs:226`) and asserts
  loose tolerances (error < 5–15% where p=12 theoretical SE is 1.6%). Bloom FPP measured
  once (`bloom_test.rs:153`), DDSketch rank-error checked per-quantile once. Only
  `reservoir_test.rs:359` and `varopt_test.rs:394` do multi-seed trials. There is **no
  harness asserting e.g. "over 100 seeds, ≥95% of HLL estimates fall within
  2·(1.04/√m)"**.

### Fuzzing & untrusted deserialization — **worst gap, with a concrete bug**

- No `fuzz/` dir, no cargo-fuzz, no `arbitrary` dep.
- A central validator exists (`src/common/validation.rs`: `MAX_BYTE_SIZE` 256 MB,
  `validate_precision`, etc.) and HLL/CPC use it well (`hyperloglog.rs:355`,
  `cpc.rs:408` do incremental bounds checks). **But it is not uniformly applied:**
  - **`src/quantiles/spline_sketch.rs:296` (`deserialize`)**: `sample_count` is
    attacker-controlled u64;
    `let expected_len = 16 + sample_count * 8 + 24;` **overflows in release** (no
    `overflow-checks` in any profile), bypassing the length check, then
    `Vec::with_capacity(sample_count)` → multi-exabyte allocation request (abort/OOM) or
    out-of-bounds indexing panic. This is a remote-DoS on any service deserializing
    sketch bytes — and it ships through PyO3/napi to Python/Node users.
  - **`src/membership/binary_fuse.rs:435`**: `(segment_length * segment_count)` is a u32
    multiply that can wrap (segment_length unvalidated up to u32::MAX, count=3); the
    fingerprint-length check is **skipped entirely when `size == 0`**; `size: usize`
    read raw from bytes and never bounded.
- ~25 sketches implement the `serialize/deserialize` trait (`common/traits.rs:47–59`);
  each is a hand-rolled binary parser — exactly the surface fuzzing exists for.

### Miri / sanitizers

- None in CI. 11 `unsafe` sites, all `get_unchecked` in hot paths
  (`ultraloglog.rs:276,284`, `count_min.rs:176,234`, `count_sketch.rs:191,258,317`,
  `removable_sketch.rs:111`, `bloom.rs:135,160`). Workspace lint is only
  `unsafe_code = "warn"`. These are index-math-dependent — precisely what
  Miri/debug-assert runs catch.

### Cross-language conformance

- `tests/cross_language_validation.py` (321 lines): docstring claims 5-language
  validation but the code **only imports the Python bindings** and re-asserts loose
  ranges (e.g. HLL of 3 items in (2.5, 3.5)). Node/Java/C# are never exercised.
- `validation_results.json` at repo root is its output — **a stale committed artifact
  showing all zeros** (`"total": 0, "passed": 0`), i.e. the last run couldn't even
  import the bindings. It is referenced by nothing in CI and never regenerated.
- **No golden files** (`find` for `*.bin`/`*.golden`: none). Serialization is nearly
  untested in bindings: exactly **2** `to_bytes` references across all of
  `python/tests` (287 tests) + `nodejs/__tests__` (253 tests). Nothing asserts
  Rust-serialized bytes deserialize in Python/Node, or that estimates match across
  languages.

### Coverage, semver, mutation

- **Coverage:** no tarpaulin/llvm-cov/grcov anywhere. No gate.
- **Semver/API:** `VERSION` + `scripts/sync-version.sh` + `VERSIONING.md` exist, but no
  cargo-semver-checks / public-api snapshot.
- **Mutation testing:** absent.

### Doctests / examples — the bright spot

- `cargo test --doc` runs in CI (test.yml:74); ~670 doc code fences in src; `cargo doc`
  is warning-fatal; `sketch_oxide/examples/*.rs` compile as part of `cargo test`.

---

## 2. Gaps ranked by risk (correctness shipping to 5 ecosystems)

| # | Risk | Gap | Evidence |
|---|------|-----|----------|
| 1 | **Critical** | Untrusted-deserialization panics/OOM: overflow bypass in `spline_sketch.rs:296`; wrap + skipped check in `binary_fuse.rs:435`; ~25 hand-rolled parsers, zero fuzzing, no `overflow-checks` in release | shown above |
| 2 | **High** | No cross-language conformance: no golden files, no byte-compat tests, no numeric parity; the one "validator" is Python-only, stale, and not in CI | `tests/cross_language_validation.py`, `validation_results.json` |
| 3 | **High** | Statistical guarantees not actually tested — single fixed-seed runs with tolerances 3–10x looser than theory; a subtle estimator bias regression would pass CI | `hyperloglog_test.rs:112–195` |
| 4 | **High** | Publish pipeline decoupled from tests; token-based PyPI; single-platform npm publish; no attestation | `publish.yml` |
| 5 | **Medium** | `unsafe get_unchecked` hot paths with no Miri/ASan/debug-assertion CI lane | 11 sites listed above |
| 6 | **Medium** | Java/C# binding tests exist but never run; those FFI crates also escape clippy (`--lib` in sketch_oxide dir only) | test.yml:185–189, 209 |
| 7 | **Medium** | No coverage measurement → 1,067 tests but unknown blind spots (deserialize error paths likely uncovered given bug #1) | — |
| 8 | **Low** | No semver-checks; no CI on `releases/*` branches; mypy/tsc soft-failed; cache@v3 deprecated | test.yml:3–7, 237, 248 |
| 9 | **Low** | Benches compile-only; no perf regression tracking | test.yml:78 |

---

## 3. Recommendations (SOTA-2026 bar for a probabilistic library)

### Immediate (S)

1. **Fix the deserialization bugs.** `spline_sketch::deserialize`: checked arithmetic
   (`sample_count.checked_mul(8)`), cap against `bytes.len()` **before**
   `with_capacity`. `binary_fuse::deserialize`: checked mul, validate `size`, drop the
   `size > 0` escape hatch. Add `overflow-checks = true` to `[profile.release]` (or at
   least a CI test profile).
2. Add a CI lane running the test suite with a `-C debug-assertions` release build —
   free bounds-adjacent coverage for the `get_unchecked` math.
3. Un-soft-fail mypy/tsc; add `releases/**` to CI triggers; bump cache@v3→v4; make
   `publish.yml` `needs:` a green test run; switch PyPI to Trusted Publishing.

### Short term (M)

4. **cargo-fuzz workspace** with one target per `deserialize` impl (a single generic
   target over the `Sketch` trait covers most), plus round-trip targets
   (`deserialize(serialize(x)) == x` via `arbitrary`). Run a 5-min smoke on PR + nightly
   cron with corpus caching. Table stakes for a library whose bytes cross trust and
   language boundaries.
5. **Golden-file conformance suite:** commit `testdata/golden/*.bin` (serialized
   HLL/CPC/Bloom/DDSketch/CountMin at fixed seeds/inputs) + expected estimates in JSON;
   assert byte-exact round-trip + estimate parity from Rust, Python, and Node tests in
   CI. This simultaneously pins the wire format (there is currently no format-stability
   guarantee across versions). Retire `validation_results.json` or regenerate it in CI
   as an artifact, not a committed file. (Matches roadmap testing-strategy item 6:
   one test-vector corpus consumed by all binding suites.)
6. **Statistical CI harness:** per estimator, N=100–1000 seeded trials asserting the
   *distribution* of errors (e.g. empirical RMSE ≤ 1.2 × theoretical SE; coverage of 2σ
   interval ≥ 90%; Bloom measured FPP ≤ 1.5 × configured). Deterministic seed list so
   it's flake-free. `#[ignore]`-heavy variants for nightly. (Matches roadmap testing
   item 2: error-bound validation against theory.)
7. Miri job (`cargo miri test` on the modules containing unsafe; nightly cron if too
   slow) — 11 unsafe sites makes this cheap.
8. `cargo llvm-cov` in CI with a report artifact (gate later, measure now); expect
   deserialize error paths to light up as uncovered.

### Medium term (M/L)

9. cargo-semver-checks on PR + a `public-api` snapshot test — 5 downstream ecosystems
   amplify any accidental break.
10. Re-enable Java/C# test jobs or remove the bindings from the workspace; half-alive
    bindings are the worst state.
11. **Mutation testing (cargo-mutants): yes, valuable here** — tolerance-based asserts
    (`error < 0.15` where theory says 0.016) are exactly the tests that let mutants of
    estimator constants/bias-correction terms survive. Run scoped per-module nightly,
    not on PR. Expect it to flag the loose HLL tolerances immediately.
12. Criterion benchmarks on a cron with `criterion-compare`/bencher-style tracking
    (benches already exist for ~30 algorithms; currently only compile-checked).
