# sketch_oxide — Session Handoff

**Branch:** `releases/v0.2.0`
**HEAD at handoff:** `b50cd75`
**Scope of this doc:** everything done this session **plus** the open roadmap, so the full
project state is recoverable from one file.

- **Part 0** — what this session accomplished (summary).
- **Part 1** — Python (PyO3) bindings: full detail (the bulk of this session).
- **Part 2** — broader project status / open roadmap (Rust core P1/P2 backlog, testing, other
  FFI, docs/DoD).

---

# Part 0 — What this session accomplished

Two big pieces of work landed this session, both on `releases/v0.2.0`, all paper-faithful /
tested / clippy+fmt clean / committed & pushed serially (local == remote), **no PRs opened**.

### A. Rust core — remaining P2 backlog algorithms (Phase-1 of this session)

Implemented the faithfully-implementable remainder of the roadmap-2026 P2 backlog in the Rust
lib (`sketch_oxide/src/...`), each paper-verified where a source existed, with unit+doctests.
The faithfully-implementable backlog was **exhausted** this session — remaining items were
flagged as paywalled / niche / blocked / redundant (superseded by an existing sketch). The
lib test suite stands at **~1067 tests** (up from 689 at the start of the larger effort).
See the live task list #13 and `docs/sandbox/research/00-roadmap-2026.md` for the exact
per-algorithm breakdown; the remaining paper-verify tail is enumerated in **Part 2 → #13**.

### B. Python (PyO3) bindings — full coverage (main of this session)

Brought `sketch_oxide/python` to **full algorithm coverage** of the core lib: **184 `#[pyclass]`
types + 7 module-level DP functions** across all **16 core modules**, one commit per module
(17 commits, `a507af9` → `b50cd75`). Full detail in **Part 1**.

### Commit trail (Python bindings, newest first)

```
b50cd75 feat(python): bind HllJointEstimator (statistics) — close last algorithm gap
27f3f4f feat(python): full differential-privacy coverage in PyO3 bindings
b2a5145 feat(python): full learned-sketch coverage in PyO3 bindings
4bc0016 feat(python): full network-telemetry coverage in PyO3 bindings
e8809af feat(python): full universal-monitoring coverage in PyO3 bindings
dc00e53 feat(python): full matrix-sketch coverage in PyO3 bindings
9c2a355 feat(python): full statistics coverage in PyO3 bindings
2265ca8 feat(python): full graph-sketch coverage in PyO3 bindings
e961d15 feat(python): full range-filter & learned-index coverage in PyO3 bindings
de39555 feat(python): full set-reconciliation coverage in PyO3 bindings
5783a8a feat(python): full streaming coverage in PyO3 bindings
d49a8c8 feat(python): full sampling coverage in PyO3 bindings
307ebe6 feat(python): full similarity coverage in PyO3 bindings
f8547d1 feat(python): full quantiles coverage in PyO3 bindings
b906639 feat(python): full membership coverage in PyO3 bindings
829e7f2 feat(python): full frequency coverage in PyO3 bindings
a507af9 feat(python): full cardinality coverage in PyO3 bindings
```

> The Rust P2-backlog commits (piece A) landed earlier on the same branch; identify them with
> `git log --oneline releases/v0.2.0` (they precede `a507af9` and are described in task #13).

---

# Part 1 — Python (PyO3) bindings (full detail)

## TL;DR

The Python PyO3 bindings are at **full algorithm coverage** of the core `sketch_oxide`
library: **184 `#[pyclass]` types + 7 module-level DP functions**, across all 16 core
modules. Every commit was `cargo fmt`/`clippy`-clean, built with `maturin develop --release`,
runtime smoke-tested, and pushed serially (local == remote), one commit per module.

The only lib exports **not** bound as standalone Python classes are non-algorithm items
(stat/diff return structs, config enums, a pure type alias, transport types) plus a small set
of **truly-generic multi-type-parameter sketches** that are intentionally deferred (see below).

---

## Architecture / conventions

- **Mechanism:** thin PyO3 wrappers binding directly to the `sketch_oxide` lib types — the
  same idiomatic approach as `pdf_oxide`. One wrapper file per algorithm in
  `python/src/<name>.rs`, registered in `python/src/lib.rs`.
- **Crate:** `sketch_oxide_py` (`python/`), `crate-type = ["cdylib"]`, built with `maturin`.
  The Python module is named `sketch_oxide`.
- **Item conversion helpers** (`python/src/common.rs`):
  - `python_item_to_hash(item) -> u64` — for hash-keyed sketches.
  - `python_item_to_bytes(item) -> Vec<u8>` — for byte-keyed / heavy-hitter sketches so keys
    stay meaningful (returned to Python as `bytes`).
  - `with_python_item!` macro — for generic dispatch.
- **Generic-T sketches** are instantiated over `Vec<u8>` (bytes in/out) unless the existing
  wrapper used `String` (e.g. `ReservoirSampling`, `VarOptSampling`).
- **u64-keyed** structures accept a Python `int` directly (e.g. `L0Sampler`, `PinSketch`,
  `SignedUpdateSampler`, graph node IDs) so returned keys round-trip exactly.
- **Differential privacy:** every DP type holds an OS-seeded `rand::rngs::StdRng`
  (ChaCha CSPRNG, matching the lib's `mechanisms::secure_rng`). **No seed parameter is
  exposed** for DP noise, so the noise can never be made predictable. Verified that real noise
  is injected (e.g. `DpContinualCounter` returned 59 for a true count of 100).
- **Error mapping:** `Result::Err` → `PyValueError` with the lib's error string. Constructor
  parameter validation is delegated to the lib (so e.g. `BloomRf(min_level=0)` surfaces the
  lib's "must be in 1..=64" message).

### Build / verify loop (use this for any further work)

```bash
export VIRTUAL_ENV=/tmp/so_py_test
export PATH="/tmp/so_py_test/bin:$PATH"

cargo fmt -p sketch_oxide_py
cargo clippy -p sketch_oxide_py        # must be clean
cd python && maturin develop --release # builds + installs into the venv
python -c "import sketch_oxide as so; ..."   # runtime smoke test
```

> **Do NOT run `cargo test` on the whole workspace** — the Node.js FFI crate fails to link.
> Use `-p sketch_oxide` / `-p sketch_oxide_py`.

> **Commit hygiene:** the repo's pre-commit hook runs a napi build and **stashes unstaged/
> untracked files**. Commit strictly serially; never edit tracked files mid-commit. If you add
> a dependency, stage `Cargo.lock` together with `Cargo.toml` or the hook's stash will conflict
> (this happened once with the `rand` addition and had to be re-committed).

---

## What was done (one commit per module)

| Module | Commit | Notes |
|---|---|---|
| cardinality | `a507af9` | 15 algorithms |
| frequency | `829e7f2` | heavy-hitter sketches return `bytes` keys |
| membership | `b906639` | quotient/cuckoo/xor/bloom family |
| quantiles | `f8547d1` | DD/REQ/KLL/t-digest/GK/Q-digest/… |
| similarity | `307ebe6` | MinHash family + LSH indexes |
| sampling | `d49a8c8` | reservoir/weighted/priority/EBPPS/L0/… |
| streaming | `5783a8a` | sliding-window + time-adaptive sketches |
| reconciliation | `de39555` | IBLT/CPISync/PinSketch/RangeReconciler/Strata |
| range_filters + learned indexes | `e961d15` | SuRF/Rosetta/Proteus/Arf/PGM/RadixSpline/… |
| graph | `2265ca8` | triangle counting + TCM/GSS/HyperANF/AGM |
| statistics | `9c2a355` | AMS/p-stable/k-ary/join/density/Morris/Moments |
| matrix | `dc00e53` | FrequentDirections family + JL + CountSketch |
| universal | `e8809af` | UnivMon/CocoSketch/OmniSketch |
| net | `4bc0016` | BeauCoup |
| learned | `b2a5145` | oracle-augmented sketches + FeatureHasher + GradientSketch |
| privacy | `27f3f4f` | DP sketches + LDP oracles + accountant + mechanisms |
| statistics (gap-closer) | `b50cd75` | `HllJointEstimator` (union/intersection/Jaccard over 2 HLLs) |

All pushed; local == remote.

### Naming decisions (to avoid collisions)
- `statistics::MomentsSketch` → Python **`MomentsStatistics`** (the quantiles `MomentsSketch`
  keeps the canonical name).
- `reconciliation::rateless_iblt::RatelessIblt` (coded-symbol variant) → Python
  **`RatelessIbltCoded`** (the cell-based `iblt::RatelessIBLT` keeps **`RatelessIBLT`**).

---

## What is intentionally NOT bound (and why)

These appear in the lib's `pub use` exports but are **not** standalone algorithms, so they are
not separate Python classes:

- **Stat / diff return structs** — already surfaced as tuples/values via the parent class's
  methods: `*Stats` (`NitroSketchStats`, `VacuumFilterStats`, `LearnedBloomStats`,
  `SlidingHLLStats`, `IbltStats`, `GrafiteStats`, `GRFStats`, `MementoStats`,
  `RatelessIBLTStats`, `UnivMonStats`), and `CpiDiff` / `RangeDiff`.
- **Config enums** (passed as method/constructor params where relevant): `ErrorType`, `ReqMode`.
- **Pure type alias**: `KllFloatSketch = KllSketch` (the target `KllSketch` is bound).
- **Transport type**: `CodedSymbol` — the one-shot static `RatelessIbltCoded.reconcile(...)` is
  exposed instead of the per-symbol streaming protocol.

### Deferred — truly-generic multi-type-parameter sketches

Per the binding instructions, sketches generic over a `Summary` / `Mergeable` / closure type
parameter are deferred (PyO3 cannot express an open generic to Python; each would need a
concrete instantiation choice). Documented here so a future pass can decide on concrete
instantiations:

| Type | Module | Generic over | Suggested concrete binding if pursued |
|---|---|---|---|
| `TupleSketch<S: Summary>` + `ThetaCore<S>` / `Summary` / `NoSummary` / `SumDoubles` | cardinality | a `Summary` impl | bind over `SumDoubles` (a sum-of-doubles summary) |
| `FrequentDistinctTuples<K, V>` | frequency | key/value types | bind over `(Vec<u8>, Vec<u8>)` |
| `WindowedAggregator<S: Mergeable>` | streaming | a `Mergeable` summary | pick one concrete mergeable summary, or several named variants |
| `FibaAggregator<V>` | streaming | value + combine fn | needs a Python-callable combine → see `ClosureOracle` note |
| `ClosureOracle<F>` / `Oracle` / `Score` | learned | a Rust closure | `Oracle` capability already provided concretely via the bound `PrecomputedOracle` |

---

## What still needs doing (optional / future)

1. **Python-callable–backed oracle** (analogue of `ClosureOracle`). The three learned sketches
   (`LearnedCountMin`, `LearnedFrequent`, `SandwichedLearnedBloom`) currently accept
   `&PrecomputedOracle`. To support an oracle backed by an arbitrary Python function, add a
   `CallableOracle` that holds a `Py<PyAny>` and implements the lib's `Oracle` trait (calling
   back into Python), then **widen those three sketches' method signatures** from
   `&PrecomputedOracle` to accept either oracle. This changes their already-committed API, so it
   was left out pending a decision. `PrecomputedOracle` already covers the "precomputed score
   table" use case.
2. **Decide on concrete instantiations for the deferred generics** above if those sketches are
   wanted from Python (table gives suggested choices).
3. **Packaging / publish**: build wheels for distribution (cibuildwheel / maturin) and publish
   to PyPI — not attempted here; only `maturin develop` was used for local verification.
4. **Python-level tests & type stubs**: there is no `tests/` suite or `.pyi` stub file for the
   Python package yet; smoke tests were run inline during development but not committed as a
   test suite.
5. **Docstring/README parity**: the module docstring in `python/src/lib.rs` lists representative
   algorithms per category; a fuller per-class reference / examples doc could be generated.
6. **Other language bindings** (nodejs/napi, java/JNI, dotnet/csharp-ffi) were explicitly out of
   scope and were not updated to match this coverage.

---

## Quick verification recipe

Re-run the coverage audit (lib `pub use` exports vs registered Python classes):

```bash
cd sketch_oxide   # repo root containing sketch_oxide/ and python/
grep -oE 'm.add_class::<[a-z0-9_]+::[A-Za-z0-9]+>' python/src/lib.rs \
  | sed -E 's/^.*:://; s/>$//' | sort -u > /tmp/registered.txt
for m in cardinality frequency membership quantiles similarity sampling streaming \
         reconciliation range_filters graph statistics matrix universal privacy learned net; do
  grep -E '^pub use' "sketch_oxide/src/$m/mod.rs" \
    | sed -E 's/^pub use [a-z0-9_:]+:://' | tr -d '{};' | tr ',' '\n' \
    | sed -E 's/[[:space:]]//g' | grep -E '^[A-Z]' | sort -u > /tmp/mod_types.txt
  echo "== $m: UNREGISTERED:" $(comm -23 /tmp/mod_types.txt /tmp/registered.txt | tr '\n' ' ')
done
```

Anything listed as "UNREGISTERED" should match one of the documented non-algorithm /
deferred-generic categories above. Note that renamed bindings
(`RatelessIblt`→`RatelessIbltCoded`, `statistics::MomentsSketch`→`MomentsStatistics`) will show
as "unregistered" by raw name even though they are bound.

---
---

# Part 2 — Broader project status (beyond Python bindings)

> The Python binding effort above is **complete**. This part captures the rest of the
> `sketch_oxide` roadmap so the whole project state is recoverable from one file. It mirrors the
> live task list (#13–#18) and the roadmap docs. **Source of truth** for the full plan:
>
> - `docs/sandbox/research/00-roadmap-2026.md` — the master 2026 roadmap (phases, waves, DoD).
> - `docs/sandbox/research/00-master-gap-analysis.md` — gap analysis.
> - `docs/sandbox/research/00-baseline-audit-v0.2.0.md` — baseline audit.
> - `docs/sandbox/research/01-cardinality.md` … `07-sampling.md` (+ more) — per-domain research.
> - Top-level `ROADMAP.md`, `CHANGELOG.md`, `ALGORITHM_SELECTION_GUIDE.md`.
>
> All algorithm work targets branch **`releases/v0.2.0`**, paper-faithful, with unit+doctests,
> clippy/fmt clean, committed & pushed. Lib test suite grew 689 → ~1067 tests over the effort.

## Status overview (task list mirror)

| # | Track | Status |
|---|---|---|
| 1–9 | Wave 1.0 infra (IBLT rename, EH engine, tick/watermark, WindowedAggregator, privacy mechanisms, oracle trait, Theta summary trait, salted hashing) | ✅ done |
| 10 | Wave 1.1 — cheap P0 wins (~22 algos) | ✅ done |
| 11 | Wave 1.2 — moderate P0s (~19 algos) | ✅ done |
| 12 | Wave 1.3 — hard P0s (~9 algos) | ✅ done |
| 13 | **Phase 2 — P1s (~64 items)** | 🟡 in progress: 64 done, a tail still needs paper-verify |
| 14 | Phase 3 — P2s / interop / watch-list (~49) | ⬜ backlog (opportunistic) |
| 15 | Cross-cutting — universal testing strategy | ⬜ pending |
| 16 | Cross-cutting — FFI binding follow-up (per wave) | 🟡 Python done; Java/Node/.NET pending |
| 17 | Cross-cutting — docs, module hygiene & Definition of Done | ⬜ pending |
| 18 | Phase-3 backlog — implement remaining Group A/B algos | ⬜ pending (overlaps #14) |

## #13 — Phase 2 P1s (in progress)

64 P1 algorithms delivered this effort (all paper-faithful + tested + committed). Includes 5
paper-VERIFIED hard-tier flagships transcribed from source PDFs: **SuperMinHash** (Ertl Alg 4),
**DpMisraGries** (Lebeda–Tětek §5.1/5.2), **ProbMinHash** (Ertl Alg 5), **SetSketch** (Ertl
Alg 1 + Eq 12/13).

**Remaining (each needs its own paper-verify pass before implementing/refining):**
ExaLogLog & HLL++ (packed registers / bias tables), HyperMinHash (empirical estimator —
SetSketch largely supersedes), Aleph / InfiniFilter / Morton (packed filters), EBPPS,
CocoSketch, OmniSketch, Proteus, DPSW, SketchPolymer, HyperCalm, and recent heavy-hitter
sketches. **User directive: paper-verify each** (use `crgx pdf_oxide_cli markdown <pdf>` to read
papers — see memory `pdf-extraction-cli`).

> Note: several of these names are already **bound in Python** as functional implementations
> (CocoSketch, OmniSketch, Proteus, DPSW, SketchPolymer, HyperCalm, EBPPS, ExaLogLog, HLL++).
> The "remaining" flag here is about a rigorous **paper-fidelity re-verification** of the Rust
> core, not about their existence.

## #14 / #18 — Phase 3 P2 backlog (opportunistic)

Do on user signal. See roadmap §Phase 3.

- **Group A — cheap completeness & interop:** explicit KMV mode, Scalable Bloom, HdrHistogram
  wire-compat, RMI, ARF, SuperMinHash, stratified reservoir, Vector of Counts, FlowRadar/
  LossRadar shims, HyperBitBit64 / HyperTwoBits, Hokusai, VDAF/Prio3 encoding interop.
- **Group B — niche on-demand:** DCS, GK-KG, bloomRF, Taffy, C-MinHash, LSH-Ensemble, SemDeDup
  recipe, PeriodicSketch, Morton Filter, Stingy Sketch, Telescoping Filter, Persistent CM,
  Double-Anonymous (DAS), PBSketch, signed-update weighted sampling, etc.
- **Group C — watch-list, DO NOT implement yet** (revisit late 2026 / 2027, review ≥2× in 2026
  with dated notes): Zeno Filter, MagnifierSketch, perfect samplers, ConflictSync, learned-DB
  cardinality (out of scope).

> Many Group A/B items already exist in the lib + Python (ARF, SuperMinHash, stratified
> reservoir, Hokusai, bloomRF, Taffy, C-MinHash, LSH-Ensemble, Persistent CM, PeriodicSketch,
> Morton, Telescoping, Double-Anonymous, signed-update sampler). Before implementing anything
> here, **re-audit existence first** (the Python audit recipe above + `grep` the lib) — the
> backlog text predates much of the delivered work.

## #15 — Universal testing strategy (pending)

Per roadmap §Testing:
1. **proptest for every structure:** no-false-negative (filters), monotonicity (cardinality),
   merge associativity/commutativity vs sequential ingestion, serde round-trips, deletion
   consistency.
2. **Error-bound validation:** Monte-Carlo harness per sketch checking the paper's bound (KLL
   rank error, UDDSketch rel error, Grafite FPR, AMS variance, RIBLT overhead, APBF windowed
   FPR, DP (ε,δ) distinguishability). This is what catches "Memento-class" silent
   simplifications.
3. **Cross-checks vs reference impls:** DataSketches (Theta/Tuple/KLL/REQ/EBPPS), hash4j
   (ULL/martingale/SetSketch), minisketch (PinSketch), CrowdStrike apbf, IBM SWAG (DABA/FiBA),
   Stanford msketch, pure-ldp, datasketch (MinHash-LSH).
4. **Adversarial/robustness harness.**
5. **criterion benches as regression gates.**
6. **Cross-language conformance:** one YAML/JSON test-vector corpus consumed by all 4 binding
   suites.

## #16 — FFI binding follow-up (Python done; others pending)

After each wave's Rust core stabilizes, generate bindings against the same surface:
**Python (PyO3) ✅ complete (this handoff)**, **Java (JNI/Panama) ⬜**, **Node (napi-rs) ⬜**,
**.NET (P/Invoke) ⬜**.

- New cross-cutting APIs (tick/watermark, oracle scores, LDP client/aggregator, RIBLT streaming
  decode) get a **design doc each BEFORE the first binding**.
- **Serialization round-trip tests in every language** (Rust→Python→Java→Rust) for every
  mergeable sketch.
- Per-language idiom layers: scikit-learn-compatible `FeatureHasher`, OTel SDK adapters,
  datasketch-compatible MinHash-LSH.
- FFI-specific tests for new query classes (windowed w/ simulated clocks, LDP encode→aggregate,
  learned-filter score arrays).

> When extending the **other** language bindings to match Python's coverage, the Python wrappers
> in `python/src/*.rs` are the reference for which types/methods to expose and the naming
> decisions (e.g. `MomentsStatistics`, `RatelessIbltCoded`).

## #17 — Docs, module hygiene & Definition of Done (pending)

- Module/doc hygiene: RatelessIBLT rename done (#2); **document t-digest's lack of guarantees**
  (point to SplineSketch/REQ); mark HLL "baseline, prefer ULL/ELL"; verify SplineSketch matches
  the final SIGMOD 2026 paper; document 2024–25 adaptive attacks on cardinality sketches.
- Update `ALGORITHM_SELECTION_GUIDE.md` for all new domains; per-algorithm guarantee docs
  ("what bound, under what model: insertion / bounded-deletion / turnstile / window /
  adversarial / DP").
- **Rewrite `ROADMAP.md` for v0.2.0** (currently still v0.1.6). **Write `CHANGELOG.md` [0.2.0]
  section.**
- Definition of Done (roadmap §DoD): all 52 P0 shipped w/ 4 FFI + tests + cross-checks; no empty
  domains; no mislabeled impls; use-case matrix closure; DataSketches superset; infra invariants
  green.
- Also: `no_std + alloc` audit for new code; SIMD + portable-fallback feature flag; optional
  concurrent feature.

## Operational constraints (carry forward — apply to ALL further work)

- **Never `cargo test` the whole workspace** — the Node.js FFI crate fails to link. Use
  `-p sketch_oxide` (lib) or `-p sketch_oxide_py`.
- **Strictly serial git commits.** The pre-commit hook runs a napi build (warm-build flakiness)
  and **stashes unstaged/untracked files** — never edit tracked files mid-commit; stage
  `Cargo.lock` with `Cargo.toml` when adding deps. (See memory `precommit-napi-hook`.)
- **Privacy/DP code:** CSPRNG (`StdRng::from_os_rng` / `mechanisms::secure_rng`) + discrete
  mechanisms + no exposed noise seed + no secret leakage.
- **Do NOT open PRs and do NOT run code review** unless explicitly asked.
- Read papers with `crgx pdf_oxide_cli markdown <pdf>` (memory `pdf-extraction-cli`), not image
  rendering.
- Target branch for everything: **`releases/v0.2.0`**; keep local == remote after each push.

## Suggested recovery order for a fresh session

1. Read this file + `docs/sandbox/research/00-roadmap-2026.md`.
2. Run the Python coverage audit recipe (Part 1) — confirm still green.
3. Re-audit lib existence for any #13/#14/#18 item before implementing (much is already done).
4. Pick up #13's paper-verify tail, or a cross-cutting track (#15 testing is highest-leverage
   for correctness; #17 docs/CHANGELOG is required for a v0.2.0 release).
