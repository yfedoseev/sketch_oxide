# State of the Art in Data Sketches, Mid-2026 — External Research Beyond the Internal Survey

> Part of the `docs/research/fable5/` improvement research series (2026-07-01).
> This document deliberately builds ON TOP of the internal research collection
> (`docs/sandbox/research/00-*.md` + docs 01–14, synthesized 2026-06-09) — it reports
> only what that collection does not cover, corrects two of its entries, and updates
> the competitive landscape. Compiled from ~130 web fetches/searches plus a codebase
> cross-check.

## (A) What prior internal research already covered (not duplicated here)

The internal collection already:

- Catalogs **~165 missing algorithms** (52 P0 / 64 P1 / 49 P2) across 14 domains, with a
  phased roadmap. All headline 2024–2026 papers are in it: ExaLogLog, Half-Xor, Cuckoo
  Heavy Keeper (VLDB'25), Aleph/AQF/Zeno filters, Diva/Memento/Oasis+, SpaceSaving±,
  WavingSketch, RIBLT (SIGCOMM'24)/CertainSync/ConflictSync, DS-FD, HIGGS, DPSW-Sketch,
  Lazy Sketch, UCL-Sketch, GK-KG, MagnifierSketch, adaptive attacks on HLL (ICML'24),
  and more.
- Explicitly dismissed/watch-listed: neural summaries (Meta-Sketch/Crane), MPC
  reimplementation (VDAF/Prio3 — interop only), Morton/Taffy/superseded filters,
  Hokusai, skimmed sketches, spectral sparsifiers, Merkle Search Trees (use external
  crate).

**Two corrections to the internal docs discovered during this pass** (verified in
source):

1. `sketch_oxide/src/quantiles/otel_histogram.rs` **already implements** a genuine OTel
   base-2 exponential histogram (scale ∈ [−10,20], zero bucket, ± bucket runs mirroring
   OTLP `ExponentialHistogramDataPoint`, downscale-on-merge) — the master gap analysis
   still lists it as a P0 gap. What remains is only OTLP **protobuf encoding** and a
   Prometheus native-histogram converter.
2. `streaming::ExponentialHistogram` is (correctly, per docs 04/09) the DGIM
   sliding-window structure — the naming collision with the OTel one should be
   documented.

Also confirmed already shipped (candidates sub-agents raised, then discarded): QSketch
(weighted cardinality, KDD'24), SplineSketch, UltraLogLog, CPC.

## (B) NEW findings — candidates not in (or materially updated since) the internal docs

### B.1 New algorithms

| Candidate | Citation / Year | What it improves over what we have | Ref. impl | Effort | Priority |
|---|---|---|---|---|---|
| **Extended RaBitQ** (vector quantization sketch: unbiased L2/dot estimators, 1–9 bits/dim, error bound matching Alon–Klartag lower bound) | SIGMOD 2024 + SIGMOD 2025, [arXiv:2409.09913](https://arxiv.org/abs/2409.09913) | Entirely new capability (no vector-quantization sketch in library or roadmap); adoption trifecta: [Milvus 2.6 IVF_RABITQ](https://milvus.io/blog/bring-vector-compression-to-the-extreme-how-milvus-serves-3%C3%97-more-queries-with-rabitq.md), [Elastic/Lucene BBQ](https://www.elastic.co/search-labs/blog/better-binary-quantization-lucene-elasticsearch), VectorChord (Rust) | [C++ RaBitQ-Library](https://github.com/VectorDB-NTU/Extended-RaBitQ) | 3/5 | **P0 — strongest new candidate of this survey**; no standalone Rust crate exists |
| **Sublime** (cache-line-local counter elongation + adaptive expansion → sublinear error on unbounded skewed streams) | SIGMOD 2026, [arXiv:2603.14190](https://arxiv.org/abs/2603.14190) (Pagh, Dayan et al.) | Upgrades CountMin/CountSketch fixed-width counters; complements TowerSketch P0 | Not yet | 2–3/5 as CM/CS wrapper | **P1, implement when code lands** (internal doc 02 listed "SublimeCS" as P2 watch — theory pedigree + venue now argue promotion) |
| **Breadcrumb Filters** (fast fully-featured filter: delete/count/merge/values, enumerable) | SIGMOD 2026, [DOI 10.1145/3786629](https://dl.acm.org/doi/abs/10.1145/3786629) | Claims to beat VQF; would compete with the CQF P0 as the modern feature-rich filter substrate | Awaiting artifact | 4/5 | **P1 — evaluate vs CQF plan before building CQF** |
| **Aeris Filter** (strongly+monotonically adaptive range filter, zero space overhead for adaptivity, never re-admits a FP) | SIGMOD 2026, [DOI 10.1145/3786621](https://dl.acm.org/doi/10.1145/3786621) | 3× less expansion overhead than Memento, 10× FPR vs non-adaptive under skew — directly relevant to the Memento fidelity-upgrade P0 | Expected at [n3slami](https://github.com/n3slami) (Memento/Diva authors) | 4/5 | **P1 — may change the Memento/Diva sequencing** |
| **ZOR filters** (static filter ~1% over info-theoretic optimum via abandon-<1%-keys peeling) | [arXiv:2602.03525](https://arxiv.org/abs/2602.03525) (Feb 2026) | Beats BinaryFuse/BuRR on space; several-fold slower builds | No | 4/5 | P2 watch (unreviewed; BuRR P0 stands) |
| **ZRing** (dynamic weighted cardinality) | KDD 2026, per [Tong Yang page](https://yangtonghome.github.io/) | Successor to our shipped QSketch | Not yet | ? | P2 watch |
| **Huffman-Bucket Sketch** (HLL registers Huffman-compressed to optimal bits, mergeable, O(1) amortized) | [arXiv:2603.10930](https://arxiv.org/abs/2603.10930) | Alternative to ULL/ExaLogLog space route; no ULL comparison, single author, unreviewed | No | — | Watch |
| **Dynamic Flat Filter**, **Fair-Count-Min**, **Meep Hashing** (MPHF), **Sketch-based Secure Query Processing** | SIGMOD 2026 [accepted list](https://2026.sigmod.org/sigmod_papers.shtml) | Expandable filters / group-fair CM / PtrHash-class MPHF / MPC sketches | No PDFs yet | — | Watch |
| **Learned Static Functions** (per-key prefix codes in static functions, breaks H₀ barrier, up to 10× space) | PVLDB 19(5) 2026, [arXiv:2510.27588](https://arxiv.org/abs/2510.27588) (BuRR/PGM lineage) | Beats BuRR-class retrieval on compressible value distributions | Likely (Lehmann/Vinciguerra) | 4/5 | P2 watch |
| **Trimmed-statistics sketches** (first sublinear top-k/trimmed F_p) | PODS 2026, [arXiv:2506.07342](https://arxiv.org/abs/2506.07342) | New query class for `statistics` | No | — | Watch (theory) |
| **HeavyLocker** (distributed top-k), **Hypersistent Sketch** (persistence) | KDD 2025 / ICDE 2025 | Distributed global top-k; On-Off successor | Tong Yang group usually releases | 3/5 | P2 |
| **Fingerprint filters are optimal** (n·log ε⁻¹ + n·log e − o(n) lower bound for all dynamic filters) | FOCS 2025, [arXiv:2510.18129](https://arxiv.org/abs/2510.18129) | Validates current designs; cite in docs | N/A | — | Doc reference |
| **DP upgrades**: banded matrix factorization (production-deployed in [Gboard training](https://research.google/blog/advances-in-private-training-for-production-on-device-language-models/)), normalized-square-root coefficients ([arXiv:2509.14334](https://arxiv.org/pdf/2509.14334)), **DP Misra-Gries** ([arXiv:2301.02457](https://arxiv.org/pdf/2301.02457)), **DP hierarchical HH** ([PACMMOD 2024](https://dl.acm.org/doi/10.1145/3695826)) | 2023–2026 | Drop-in coefficient schedules for the DP continual-counting P0; DP-MG is a ~1/5 add to the privacy suite; roadmap's binary-tree-first plan should target banded-MF as the production-proven endpoint | Google MF code exists | 1–2/5 each | **P0-adjacent — fold into Phase-1 privacy wave** |
| **TurboQuant** (Google; rotation + scalar quant, near-optimal distortion) | [arXiv:2504.19874](https://arxiv.org/pdf/2504.19874) | RaBitQ rival; lost the adoption race | No | — | Watch |

### B.2 Status updates on known items (change roadmap assumptions)

- **ExaLogLog is still NOT in hash4j** (v0.30.0, 2026-03-09 — only HLL+ULL ship;
  2025–26 releases added hash functions only). Reference impl remains
  [dynatrace-research/exaloglog-paper](https://github.com/dynatrace-research/exaloglog-paper).
  Ertl has published **no new sketch since**. A Rust ExaLogLog would be the first
  production implementation anywhere.
- **SplineSketch is final**: SIGMOD 2026,
  [DOI 10.1145/3769827](https://dl.acm.org/doi/abs/10.1145/3769827), code at
  [PavelVesely/SplineSketch-experiments](https://github.com/PavelVesely/SplineSketch-experiments)
  — the roadmap's "verify SplineSketch matches the final paper" item is now actionable
  against released Java/Python code.
- **"InfixStore" is not a separate paper** — it's Diva's internal compressed-key store
  ([PVLDB 18 Diva paper](https://www.vldb.org/pvldb/vol18/p3923-eslami.pdf)); nothing
  extra to implement.
- **Zeno Filter** preprint now public
  ([nivdayan.github.io/zeno_filter.pdf](https://nivdayan.github.io/zeno_filter.pdf)).
- **Robust cardinality**: "Breaking the Quadratic Barrier" landed at ICML 2025
  ([arXiv:2502.05723](https://arxiv.org/abs/2502.05723)) — strengthens the
  hardened-cardinality P1.
- Tong Yang pipeline to track: "Near-Optimal Per-Key Streaming Quantile Estimation"
  (SIGMOD 2027, to appear — bears on the SQUAD/SketchPolymer plan), "Measuring Item
  Freshness" (KDD 2025), PBSketch (KDD 2026).
- **NSDI/SIGCOMM 2025–26 are thin on core sketches** (telemetry
  transport/virtualization instead) — the network-doc conclusion holds.
- FineWeb's MinHash config (5-grams, **14 bands × 8 rows = 112 hashes**, ~0.75 Jaccard)
  is now the de-facto industry parametrization; DCLM/Dolma standardized on **Rust Bloom
  dedup (BFF)** past ~10 TB; [LSHBloom](https://arxiv.org/pdf/2411.04257)
  (Bloom-per-LSH-band) is a cheap high-value combinator over existing MinHash+Bloom.

## (C) Domain trends → productization ideas

1. **Data lakes (time-sensitive)** — Iceberg Puffin's `apache-datasketches-theta-v1`
   blob is the de-facto lake NDV standard (Trino/Athena/Glue read it; Glue now *writes*
   it), and
   [iceberg-rust can frame Puffin but cannot compute Theta payloads](https://github.com/apache/iceberg-rust/pull/745).
   **Ship DataSketches-v1-compatible compact-Theta serde + an iceberg-rust example/PR**
   before apache/datasketches-rust claims the slot. Effort ~2/5; validates the roadmap's
   "selective DataSketches binary compatibility" as the single highest-leverage interop
   item.
2. **Observability** — Prometheus native histograms **GA** (v3.8, defaults flipping in
   v4.0; [spec](https://prometheus.io/docs/specs/native_histograms/) defines OTel scale
   ↔ Prom schema conversion). sketch_oxide already has the histogram; add **OTLP prost
   encoding + Prometheus converter + merge/downscale operator**, and consider
   contributing the operator to OTel-Arrow's Rust `otap-dataflow` engine
   (Microsoft/F5-backed, mid-2026).
3. **Streaming engines (Rust-native demand)** — RisingWave carries a FIXME'd
   retractable `approx_count_distinct`, Feldera lists approx functions unsupported,
   Arroyo/DataFusion use home-grown HLL/t-digest with NaN issues.
   **Retraction-capable sketches** (Half-Xor P1, deletable UDDSketch) + a
   `sketch_oxide-datafusion` UDAF pack reaches Arroyo/GlareDB/LanceDB in one move.
   Chronon (Airbnb) runs **CPC + KLL over sawtooth tiled windows** — direct validation
   of the `WindowedAggregator<S: Mergeable>` P0 substrate.
4. **LLM data pipelines** — datatrove's Python MinHash is a publicized bottleneck
   ([fastdedup benchmark](https://wapplewhite4.github.io/fastdedup/)); Milvus 2.6 ships
   a `MINHASH_LSH` index with **BYO signatures** (256×64-bit as `BINARY_VECTOR`). Ship:
   datatrove-bit-compatible MinHash preset (14×8/5-gram), Milvus `to_binary_vector()`,
   BFF-compatible mmap sharded Bloom, LSHBloom mode.
5. **eBPF/security (first-mover)** — no Rust eBPF sketch crate exists for
   [Aya](https://github.com/aya-rs/aya); Cloudflare-style "sketch in kernel, decide in
   userspace" is production reality; Zeek/RITA show SIEM demand. A **`no_std`,
   integer-only, verifier-safe CM/HLL/top-k feature** plus userspace merge serves
   networking + security with one investment (aligns with the roadmap's no_std audit).
6. **P2P sync** — negentropy/NIP-77 **merged into the Nostr spec (May 2025)**, adopted
   by strfry and Waku Sync; Erlay still unmerged but alive
   ([tracking #30249](https://github.com/bitcoin/bitcoin/issues/30249)); minisketch-rs
   is abandoned. Pure-Rust PinSketch (minisketch-byte-compatible) + generic RBSR keeps
   its P0/P1 ranking with strengthened adoption evidence.
7. **Vector DBs** — RaBitQ-style rotation quantization is the sketch family that won
   2025–26 (see B.1); filtered-ANN selectivity estimation is an emerging consumer of
   classic HLL/quantile sketches inside vector DBs.

## (D) Competitive landscape snapshot, July 2026

- **Apache DataSketches went polyglot — the main competitive event.** Java 9.0.0
  (Dec 2025, zero-dependency FFM rewrite), C++ 5.2.0, **official Rust port**
  [`datasketches` 0.3.0](https://github.com/apache/datasketches-rust) (May 2026;
  HLL/Theta/CPC/CM/FI/t-digest/Bloom; **no KLL/REQ/Tuple/sampling yet**; ~849k recent
  downloads), **Go port** v0.2.0 (June 2026, includes KLL/REQ/tuple), BigQuery UDFs,
  DuckDB community extension, Spark 4.1 KLL/Theta functions, Snowflake
  `DATASKETCHES_HLL`. The DataSketches binary format is becoming the interchange
  standard; sketch_oxide's window is **breadth + bindings + interop before
  datasketches-rust matures**.
- **hash4j** v0.30.0: HLL/ULL/MinHash/SuperMinHash/SimHash only; no ExaLogLog, no
  SetSketch, Java-only.
- **Redis 8** folded the five probabilistic types into core (May 2025); nothing new
  since 2021. **ClickHouse**: `quantileDD` (Jan 2024) was the last sketch addition.
  **DuckDB core**: HLL/t-digest/approx_top_k, state not exposed.
- **Rust ecosystem**: vibrant but fragmented — fastbloom 13.9M dl, sketches-ddsketch
  69M dl, xorf, qfilter, gaoya, hyperloglockless (lock-free concurrency is a proven
  adoption lever); **the quantile space is a graveyard** (`quantiles` dead 2018,
  `tdigest` dead 2021, `tdigests` archived) — **no maintained KLL/REQ exists in Rust
  outside sketch_oxide**, its clearest published advantage.

## Net-new actions the internal roadmap does not contain

1. **Extended RaBitQ** as a new vector-sketch module (P0-class; no Rust crate exists).
2. **Puffin/Theta-v1 binary compat now** (time-boxed by datasketches-rust maturing).
3. **Promote Sublime; track Breadcrumb/Aeris** before committing the CQF/Memento waves.
4. **Banded-MF + DP-MG** folded into the Phase-1 privacy wave.
5. **LSHBloom + datatrove/Milvus-compatible presets** for the LLM-dedup story.
6. **no_std/eBPF feature** (first-mover; no Aya-compatible sketch crate exists).
7. **Mark the OTel-histogram P0 as done** except OTLP encoding; fix the doc mislabel.
