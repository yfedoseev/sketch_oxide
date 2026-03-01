# Algorithm Roadmap: SOTA 2026

> Target: ~60 algorithms across 12-13 categories, making sketch_oxide the most comprehensive probabilistic data structures library in any language.

**Current state:** 41 algorithms across 10 categories.
**Goal:** Add ~30 algorithms (including composed sketches), open 3-4 new categories.

---

## Tier 1: Must-Have

Production-proven algorithms used in major systems. Not having these is a credibility gap.

### 1. ExaLogLog

- **Category:** Cardinality
- **Year:** 2025 (EDBT)
- **Author:** Otmar Ertl (same author as UltraLogLog)
- **Description:** Next-generation cardinality estimator that requires 43% less space than HyperLogLog for the same estimation error. Supports distinct counts up to exa-scale. Commutative, idempotent, mergeable, reducible, with constant-time insert. Includes a sparse mode for small cardinalities.
- **Why we need it:** Direct successor to our UltraLogLog from the same author. Represents the latest point in the HLL lineage. Not including the 2025 state-of-the-art from our own UltraLogLog author would be a clear gap.
- **Paper:** "ExaLogLog: Space-Efficient and Practical Approximate Distinct Counting" -- Ertl, EDBT 2025

### 2. HyperLogLog++

- **Category:** Cardinality
- **Year:** 2013 (Google)
- **Description:** Google's improved HyperLogLog with 64-bit hashing, empirical bias correction for small cardinalities, and a sparse-to-dense representation that saves 10-100x memory for small sets. The de facto industry standard for cardinality estimation.
- **Why we need it:** Used by Google BigQuery, ClickHouse (`uniqCombined`), Apache Spark, and Amazon Redshift. Our HyperLogLog is the classic version -- HLL++ is what production databases actually deploy. Users coming from any major database will expect this.
- **Paper:** "HyperLogLog in Practice" -- Heule, Nunkesser, Hall, Google Research 2013

### 3. XOR Filter (Xor8 / Xor16)

- **Category:** Membership (Static AMQ)
- **Year:** 2020 (Graf & Lemire)
- **Description:** Static membership filter using exactly 3 memory accesses regardless of false-positive probability. Xor8 uses ~1.23 bytes/key with 0.39% FPR; Xor16 uses ~2.36 bytes/key with 0.0015% FPR. Faster and smaller than Bloom and Cuckoo filters for static (immutable) sets.
- **Why we need it:** We have Binary Fuse (the successor), but XOR filters are simpler, more widely deployed, and many systems reference Xor8/Xor16 by name. Having both provides completeness. Used in RocksDB and across the Rust/Go/Zig ecosystems.
- **Paper:** "Xor Filters: Faster and Smaller Than Bloom and Cuckoo Filters" -- Graf & Lemire, JEA 2020

### 4. Quotient Filter (CQF / RSQF)

- **Category:** Membership (Dynamic AMQ)
- **Year:** 2017 (Pandey et al., SIGMOD)
- **Description:** Compact hash table storing fingerprints that supports all six operations: insert, query, delete, count, merge, and resize. Good cache locality, scales to SSD, supports concurrent access. The Counting Quotient Filter (CQF) adds per-element counts with essentially zero space overhead. Foundation for modern filter research (InfiniFilter, Aleph Filter, Adaptive QF).
- **Why we need it:** No filter in sketch_oxide supports all of insert + query + delete + count + merge + resize. Quotient filters are the canonical choice when all operations are needed. Also the foundation for Tier 2 filters (InfiniFilter, Aleph, AQF).
- **Paper:** "A General-Purpose Counting Filter" -- Pandey et al., SIGMOD 2017

### 5. Aleph Filter

- **Category:** Membership (Scalable/Dynamic AMQ)
- **Year:** 2024 (VLDB)
- **Description:** The first scalable filter maintaining constant-time operations AND optimal memory-FPR tradeoffs as data grows unboundedly. Uses a novel technique of duplicating void entries across primary and secondary hash tables. Supports insertions, queries, and deletes in O(1) regardless of data size.
- **Why we need it:** All our membership filters are either static (Binary Fuse) or have fixed capacity. Aleph Filter is the first to provide infinite expandability with constant-time guarantees and no FPR degradation. VLDB 2024 -- cutting edge.
- **Paper:** "Aleph Filter: To Infinity in Constant Time" -- Dayan, Bercea, Pagh, VLDB 2024

### 6. Tuple Sketch

- **Category:** Frequency / Associative
- **Year:** Apache DataSketches
- **Description:** Extends Theta Sketch to associate arbitrary summary values (doubles, integers, strings) with each distinct key. Enables computation of summaries like impressions, clicks, and revenue alongside distinct counting. Supports full set operations (union, intersection, difference) with summaries preserved.
- **Why we need it:** The #1 most-used sketch in production analytics. Powers real-world queries like "sum of revenue for distinct users in segment A intersected with segment B." Used by Apache Druid, Google BigQuery, Databricks, LinkedIn/Pinot. Not having this is the single biggest production credibility gap.
- **Paper:** Apache DataSketches framework

### 7. GK Sketch (Simplified Optimal, 2024)

- **Category:** Quantiles
- **Year:** 2024 (SIGMOD)
- **Description:** A simplified version of the Greenwald-Khanna sketch achieving optimal O(1/eps * log(eps*n)) space with a clean analysis. The original GK (2001) was always optimal but too complex to implement correctly. The 2024 paper bridges theory and practice with a version that is both simple and provably optimal.
- **Why we need it:** The only deterministic quantile sketch with formal optimality guarantees. All our quantile sketches (DDSketch, KLL, REQ, TDigest, Spline) are randomized. For users who need deterministic error bounds with no randomness, GK is the only option. Used internally by Spark and Prometheus.
- **Paper:** "Simple & Optimal Quantile Sketch" -- Gribelyuk et al., SIGMOD 2024

### 8. OmniSketch

- **Category:** Universal / Multi-dimensional Frequency
- **Year:** 2024 (VLDB Best Paper)
- **Description:** The first sketch supporting count aggregates with filters on multiple attributes, dynamically chosen at query time. One compact sketch per attribute with fixed-size record-ID samples per cell. Handles both inserts and deletes with logarithmic update and query complexity.
- **Why we need it:** Won Best Paper at VLDB 2024 -- the most important sketch paper of the year. Already has a PostgreSQL extension. Enables a new class of multi-dimensional frequency queries that no other sketch can handle.
- **Paper:** "OmniSketch: Efficient Multi-Dimensional High-Velocity Stream Analytics with Arbitrary Predicates" -- Punter et al., VLDB 2024

---

## Tier 2: Strong Additions

Fill clear gaps, enable new use cases, open new categories.

### 9. SetSketch

- **Category:** Similarity + Cardinality (Unified)
- **Year:** 2021 (VLDB)
- **Author:** Otmar Ertl
- **Description:** Bridges the gap between MinHash and HyperLogLog. Provides cardinality estimation AND Jaccard similarity search in a single sketch. Commutative, idempotent insert operations. Mergeable for distributed environments.
- **Why we need it:** Our library treats cardinality (HLL) and similarity (MinHash) as separate concerns. SetSketch unifies them -- one data structure for cardinality, union cardinality, and Jaccard similarity. Same author as our UltraLogLog. Used in genomics (Dashing 2).
- **Paper:** "SetSketch: Filling the Gap Between MinHash and HyperLogLog" -- Ertl, VLDB 2021

### 10. Frequent Directions (Matrix Sketch)

- **Category:** NEW -- Matrix Sketch / Dimensionality Reduction
- **Year:** 2013/2016 (Liberty, KDD/SIAM)
- **Description:** Deterministic streaming algorithm for approximate SVD. Processes one row at a time with O(d*l) operations, maintaining a sketch matrix for dimensionality reduction. Enables streaming PCA, low-rank approximation, and feature extraction in a single pass.
- **Why we need it:** Opens an entirely new category that sketch_oxide doesn't cover. Essential for ML pipelines that need streaming dimensionality reduction. Part of Apache DataSketches (experimental, `datasketches-vector`).
- **Paper:** "Frequent Directions: Simple and Deterministic Matrix Sketching" -- Liberty, SIAM 2016

### 11. InfiniFilter

- **Category:** Membership (Expandable AMQ)
- **Year:** 2023 (SIGMOD)
- **Description:** A quotient filter that doubles in size when reaching capacity, sacrificing one fingerprint bit per expansion but using a novel hash slot format that keeps newer entries with longer fingerprints. Stable insert/query/delete performance across arbitrary growth.
- **Why we need it:** Precursor to Aleph Filter, solves the growing-dataset problem that fixed-capacity filters cannot handle. Superior to Scalable Bloom Filters for expanding datasets.
- **Paper:** "InfiniFilter: Expanding Filters to Infinity and Beyond" -- Dayan et al., SIGMOD 2023

### 12. Adaptive Quotient Filter (AQF)

- **Category:** Membership (Adaptive AMQ)
- **Year:** 2024 (SIGMOD)
- **Description:** The first practical adaptive filter: when a false positive is detected, the AQF updates its representation so the same false positive never recurs. Space-optimal. Supports deletions, counting, resizing, merging, and concurrent access.
- **Why we need it:** A fundamentally new capability -- filters that learn from their mistakes. SIGMOD 2024. No other filter in any library offers this.
- **Paper:** "Adaptive Quotient Filters" -- Wen et al., SIGMOD 2024

### 13. EBPPS Sampling

- **Category:** Sampling
- **Year:** 2023 (Information Processing Letters)
- **Description:** Enforces the PPS (probability proportional to size) property at all times while bounding sample size, with amortized constant processing time per item. Improves accuracy of downstream classification tasks compared to VarOpt in certain scenarios.
- **Why we need it:** Natural complement to our existing VarOpt and Reservoir samplers. Part of Apache DataSketches. Provides better accuracy for classification workloads.
- **Paper:** "Exact PPS Sampling with Bounded Sample Size" -- Lang, IPL 2023

### 14. Age-Partitioned Bloom Filter (APBF)

- **Category:** Streaming / Time-Windowed Membership
- **Year:** 2020 (with Time-Limited extensions 2023)
- **Description:** Bloom filter designed for sliding window duplicate detection. Automatically ages out old entries using partitioned bit arrays. Answers "have I seen this in the last N minutes?" with formal sliding-window semantics.
- **Why we need it:** Our Stable Bloom does time-decay but not formal sliding-window semantics. APBF is better for bounded-time membership queries. CrowdStrike maintains an open-source Go implementation for security monitoring.
- **Paper:** "Age-Partitioned Bloom Filters" -- Shtul et al., 2020

### 15. CVM Algorithm

- **Category:** Cardinality
- **Year:** 2024 (highlighted by Donald Knuth)
- **Description:** Remarkably simple cardinality estimator using sampling instead of hashing. Provides an unbiased estimator with near-optimal logarithmic space. The only practical sampling-based cardinality approach.
- **Why we need it:** All our cardinality estimators use hashing. CVM is the only practical sampling-based approach. Provides an unbiased estimator (HLL is biased at low cardinalities). Endorsed by Knuth himself.
- **Paper:** Chakraborty, Vinodchandran, Meel; Knuth's endorsement at cs.stanford.edu/~knuth/papers/cvm-note.pdf

### 16. ProbMinHash / BagMinHash

- **Category:** Similarity (Weighted)
- **Year:** 2018-2020 (Otmar Ertl)
- **Description:** Family of modern MinHash algorithms for weighted sets. ProbMinHash handles probability Jaccard similarity; BagMinHash handles multiset Jaccard. Single-pass, same sketch size as standard MinHash.
- **Why we need it:** Our MinHash/SimHash only handle unweighted sets. Weighted set similarity (comparing documents by TF-IDF weights, gene expression profiles, etc.) requires these algorithms. Same author as UltraLogLog/SetSketch.
- **Paper:** "ProbMinHash" and "BagMinHash" -- Ertl, 2018-2020

### 17. Frequent Distinct Tuples (FDT)

- **Category:** Frequency / Multi-Dimensional
- **Year:** Apache DataSketches
- **Description:** Finds "heavy hitter tuples" across multiple dimensions -- e.g., which IP addresses have the most distinct User IDs, or which User IDs connect from the most distinct IPs.
- **Why we need it:** Our frequency sketches find heavy hitters by count but cannot answer "which keys have the most distinct associated values?" This is a different and important query pattern. Used in fraud detection and security analytics.
- **Paper:** Apache DataSketches FDT documentation

### 18. Top-K (Filtered Space-Saving)

- **Category:** Frequency
- **Year:** Ongoing
- **Description:** Specialized algorithm for finding the top-K most frequent items with a tight memory budget, combining hashing with a Space-Saving structure. Specifically optimized for the "give me exactly the top K" query.
- **Why we need it:** Redis offers this as a first-class `TOPK` command. Users coming from Redis expect a dedicated Top-K API. While Space Saving can be used for top-k, Filtered Space-Saving is more memory-efficient for this specific task.
- **Paper:** Metwally et al. (Space-Saving); Redis RedisBloom TOPK

### 19. Moments Sketch

- **Category:** Quantiles / Distribution
- **Year:** 2018 (VLDB)
- **Description:** Uses the method of moments to estimate quantile distributions. Stores only ~20 moment values (~200 bytes total). Has the fastest merge speed of any quantile sketch -- O(k) merge with k=20 is unmatched.
- **Why we need it:** When merge speed is the bottleneck (e.g., aggregating quantiles across thousands of shards), Moments Sketch is unmatched. Works best for smooth distributions.
- **Paper:** "Moment-Based Quantile Sketches for Efficient High Cardinality Aggregation Queries" -- Gan et al., VLDB 2018

### 20. Scalable Bloom Filter

- **Category:** Membership
- **Year:** 2007
- **Description:** Bloom filter variant that dynamically adapts to the number of elements, growing as needed while maintaining a target false positive rate. No need to know capacity upfront.
- **Why we need it:** All current Bloom variants in sketch_oxide require knowing capacity upfront. Scalable Bloom handles the unknown-size case, which is common in distributed systems where data size is unpredictable.
- **Paper:** "Scalable Bloom Filters" -- Almeida et al., Information Processing Letters 2007

---

## Tier 3: Differentiation

Cutting-edge algorithms that would make sketch_oxide unique.

### 21. JoinSketch

- **Category:** Frequency / Inner-Product Estimation
- **Year:** 2023 (SIGMOD)
- **Description:** 10x more accurate than AGMS and Fast-AGMS for inner-product estimation (join size estimation, cosine similarity). Separates items of different frequency into different components for unbiased, lower-variance estimates.
- **Why we need it:** Inner-product / join-size estimation is a key database optimization query. No sketch in our library directly addresses this. SIGMOD 2023.
- **Paper:** "JoinSketch: A Sketch Algorithm for Accurate and Unbiased Inner-Product Estimation" -- Yang et al., SIGMOD 2023

### 22. WavingSketch

- **Category:** Frequency / Top-K
- **Year:** 2020 (KDD), extended 2024 (VLDB Journal)
- **Description:** Unbiased and generic sketch for finding top-k items. 10x faster and 1000x more accurate than Unbiased Space-Saving. Generic to five measurement tasks: top-k frequent items, heavy changes, persistent items, super-spreaders, and join-aggregate estimation.
- **Why we need it:** Unbiased frequency estimation is a key differentiator. Our Count-Min and Space-Saving are biased estimators. WavingSketch provides unbiased estimates with dramatically better accuracy.
- **Paper:** "WavingSketch: An Unbiased and Generic Sketch" -- Li et al., KDD 2020

### 23. CocoSketch

- **Category:** Universal / Partial-Key Queries
- **Year:** 2021 (SIGCOMM)
- **Description:** High-performance sketch for arbitrary partial key queries in network measurement. Uses a novel "coordinate-level" counting approach. Handles queries like "frequency of flows from source IP X to any destination."
- **Why we need it:** Our universal sketches (UnivMon, NitroSketch) focus on general-purpose measurement. CocoSketch specifically handles partial-key queries, a gap in network telemetry workloads.
- **Paper:** "CocoSketch: High-Performance Sketch-based Measurement over Arbitrary Partial Key Query" -- SIGCOMM 2021

### 24. Proteus (Self-Designing Range Filter)

- **Category:** Range Filters
- **Year:** 2022 (SIGMOD)
- **Description:** Self-configures based on sampled data to optimize FPR for a given space budget. Unifies probabilistic and deterministic design spaces. Up to 5.3x better end-to-end performance than SuRF and Rosetta.
- **Why we need it:** Our range filters (Memento, GRF, Grafite) are all manually configured. Proteus auto-tunes itself and adapts to workload shifts. A natural complement to the existing range filter suite.
- **Paper:** "Proteus: A Self-Designing Range Filter" -- Benson et al., SIGMOD 2022

### 25. SuRF (Succinct Range Filter)

- **Category:** Range Filters
- **Year:** 2018 (SIGMOD)
- **Description:** The foundational range filter based on Fast Succinct Tries (FST), consuming only 10 bits per trie node. Supports both single-key lookups AND range queries.
- **Why we need it:** The "classic" range filter that Memento/GRF/Grafite improve upon. Deployed in RocksDB. Provides the baseline option for users who want the battle-tested original.
- **Paper:** "SuRF: Practical Range Query Filtering with Fast Succinct Tries" -- Zhang et al., SIGMOD 2018

### 26. Prefix Filter

- **Category:** Membership (Static AMQ)
- **Year:** 2022 (VLDB)
- **Description:** Provably better than Bloom filters for incremental (insert-only) use cases. Space-optimal among incremental filters.
- **Why we need it:** A strict theoretical improvement over Bloom for the insert-only case, from VLDB 2022. Completes the "modern alternatives to Bloom" story alongside Binary Fuse and XOR.
- **Paper:** "Prefix Filter: Practically and Theoretically Better Than Bloom" -- Even et al., VLDB 2022

### 27. Density Sketch (Kernel Density Estimation)

- **Category:** NEW -- Distribution Estimation
- **Year:** 2020+ (Apache DataSketches)
- **Description:** Estimates kernel density functions in streaming fashion. Useful for anomaly detection, statistical learning, and distribution monitoring.
- **Why we need it:** Opens a new category -- distribution estimation. Part of Apache DataSketches. Enables anomaly detection and statistical learning use cases that no current sketch_oxide algorithm addresses.
- **Paper:** Apache DataSketches KDE implementation

### 28. DP Misra-Gries (Differentially Private Heavy Hitters)

- **Category:** NEW -- Privacy-Preserving
- **Year:** 2024 (SIGMOD Record)
- **Description:** Computes differentially private approximate histograms and heavy hitters using the Misra-Gries sketch. Provides formal differential privacy guarantees.
- **Why we need it:** Opens a new category -- privacy-preserving sketches. As privacy regulations tighten (GDPR, CCPA), DP-compatible sketches are increasingly production-relevant.
- **Paper:** "Better Differentially Private Approximate Histograms and Heavy Hitters using the Misra-Gries Sketch" -- SIGMOD Record 2024

### 29. L0 Sampler

- **Category:** Sampling / Graph Streaming
- **Year:** 2011 (PODS), ongoing use through 2025
- **Description:** Samples uniformly from non-zero elements in a turnstile stream (supports both insertions and deletions). Foundation for graph streaming algorithms: connectivity, sparsification, spanners.
- **Why we need it:** A building-block data structure that enables an entire class of graph streaming algorithms. Required for dynamic graph problems. Our sampling algorithms (Reservoir, VarOpt) only handle insert-only streams.
- **Paper:** "Optimal Algorithms for the l0 Difference Problem" -- Jowhari et al., PODS 2011

### 30. Morton Filter

- **Category:** Membership
- **Year:** 2018
- **Description:** Faster, more space-efficient Cuckoo filter variant using biasing, compression, and decoupled logical sparsity. Achieves 1.3-2.5x faster lookups and up to 15.5x faster insertions than standard Cuckoo.
- **Why we need it:** A strict performance improvement over our existing Cuckoo filter with the same design philosophy. Drop-in upgrade for Cuckoo users who need more speed.
- **Paper:** "Morton Filters: Faster, Space-Efficient Cuckoo Filters via Biasing, Compression, and Decoupled Logical Sparsity" -- Breslow & Jayasena, VLDB 2018

---

## New Categories Summary

| New Category | Algorithms | Why it matters |
|---|---|---|
| **Matrix Sketch** | Frequent Directions | Streaming SVD/PCA for ML pipelines |
| **Distribution Estimation** | Density Sketch | Anomaly detection, statistical learning |
| **Privacy-Preserving** | DP Misra-Gries | Regulatory compliance (GDPR, CCPA) |

## Final Target

| Category | Current | After Roadmap |
|---|---|---|
| Cardinality | 5 | 8 (+ExaLogLog, HLL++, CVM) |
| Membership | 9 | 18 (+XOR, Quotient, Aleph, AQF, InfiniFilter, Scalable Bloom, Prefix, Morton, APBF) |
| Quantiles | 5 | 8 (+GK, Moments, OmniSketch*) |
| Frequency | 8 | 14 (+Tuple, OmniSketch, FDT, Top-K, JoinSketch, WavingSketch) |
| Similarity | 2 | 4 (+SetSketch, ProbMinHash/BagMinHash) |
| Sampling | 2 | 4 (+EBPPS, L0 Sampler) |
| Streaming | 3 | 4 (+APBF*) |
| Reconciliation | 1 | 1 |
| Range Filters | 3 | 6 (+SuRF, Proteus, CocoSketch*) |
| Universal | 3 | 4 (+CocoSketch) |
| **Matrix Sketch** | 0 | 1 (+Frequent Directions) |
| **Distribution** | 0 | 1 (+Density Sketch) |
| **Privacy** | 0 | 1 (+DP Misra-Gries) |
| **Composed** | 0 | 7 (+EHKLL, EHUniv, M4, SketchPolymer, DPSW, HiddenSketch, HyperCalm) |
| **TOTAL** | **41** | **~70** |

---

## Tier 4: Composed Sketches

Higher-level data structures that combine existing sketch_oxide primitives into new capabilities. All building blocks already exist in the library -- these compositions unlock new query patterns.

### 31. PromSketch: EHKLL (Exponential Histogram + KLL)

- **Category:** Composed / Streaming Quantiles
- **Year:** 2025 (VLDB)
- **Authors:** Zeying Zhu, Jonathan Chamberlain, Kenny Wu, David Starobinski, Zaoxing Liu
- **Description:** Combines Exponential Histogram with KLL sketch to answer approximate quantile queries over arbitrary sliding time windows. Small buckets use exact frequency maps; large buckets use KLL sketches. EH only requires additive mergeability from the inner sketch, giving O(1) amortized insertion.
- **Primitives used:** Exponential Histogram (have it), KLL Sketch (have it)
- **Why we need it:** Enables `quantile_over_time(p99, 5m)` style queries used in Prometheus/VictoriaMetrics monitoring. Covers 70% of Prometheus aggregation-over-time queries. Direct collaboration opportunity with authors.
- **Paper:** "Approximation-First Timeseries Monitoring Query At Scale" -- Zhu et al., VLDB 2025
- **Code:** github.com/Froot-NetSys/promsketch

### 32. PromSketch: EHUniv (Exponential Histogram + Universal Sketch)

- **Category:** Composed / Streaming Universal
- **Year:** 2025 (VLDB)
- **Authors:** Same as above
- **Description:** Combines Exponential Histogram with Universal Sketch (Count-Sketch-based) to answer count, entropy, L2 norm, distinct count, and top-k queries over arbitrary sliding time windows. A single composed sketch supports multiple query functions.
- **Primitives used:** Exponential Histogram (have it), UnivMon/Universal Sketch (have it)
- **Why we need it:** One data structure for `count_over_time`, `entropy_over_time`, `topk_over_time`, `distinct_over_time` -- the multi-function query capability is unique. No other composed sketch does this.
- **Paper:** Same as EHKLL above

### 33. M4 (Per-Flow Quantile Adapter)

- **Category:** Composed / Per-Key Quantiles
- **Year:** 2024 (ICDE)
- **Description:** Generic adapter that wraps any single-stream quantile sketch (DDSketch, TDigest, REQ) into a per-flow/per-key quantile sketch. Uses multi-layer hash buckets with MINIMUM and SUM techniques to minimize noise from hash collisions. Answers "what is the p99 latency for flow X?" across millions of concurrent flows.
- **Primitives used:** DDSketch (have it), TDigest (have it), REQ (have it)
- **Why we need it:** Per-key quantile estimation is one of the most requested features in monitoring and network measurement. Can be implemented as a generic `PerFlowQuantile<Q: QuantileSketch>` adapter. Turns our existing quantile sketches into per-key variants for free.
- **Paper:** "M4: A Framework for Per-Flow Quantile Estimation" -- ICDE 2024

### 34. SketchPolymer (Per-Item Tail Quantile)

- **Category:** Composed / Per-Key Tail Quantiles
- **Year:** 2023 (KDD)
- **Description:** A pipeline of early filtration + frequency estimation (Count-Min style) + value sampling/selection to estimate per-item tail quantiles (e.g., p99 per user) in a single sketch. 32x more accurate than prior art. Implemented on P4 and FPGA.
- **Primitives used:** Count-Min Sketch (have it), quantile primitives (have them)
- **Why we need it:** Per-item tail quantiles are critical for SLA monitoring ("is any single user experiencing p99 > 100ms?"). Complementary to M4 -- SketchPolymer is tighter-coupled and more hardware-friendly.
- **Paper:** "SketchPolymer: Estimate Per-item Tail Quantile Using One Sketch" -- KDD 2023

### 35. DPSW-Sketch (Differentially Private Sliding Window Frequency)

- **Category:** Composed / Privacy + Streaming
- **Year:** 2024 (KDD)
- **Description:** Combines Smooth Histogram (for checkpoint creation over sliding windows) with Private Count-Min Sketch (PCMS) at each checkpoint, with differential privacy noise calibration. Enables frequency estimation and heavy-hitter detection over sliding windows with formal DP guarantees.
- **Primitives used:** Count-Min Sketch (have it), Sliding Window (have it)
- **Why we need it:** The intersection of privacy + streaming is increasingly mandated. This is the first practical DP sketch for sliding window frequency queries. Composes two primitives we already have with a DP noise layer.
- **Paper:** "DPSW-Sketch: A Differentially Private Sketch Framework for Frequency Estimation over Sliding Windows" -- KDD 2024

### 36. Hidden Sketch (Reversible Frequency Sketch)

- **Category:** Composed / Invertible Frequency
- **Year:** 2025 (arXiv)
- **Description:** Composes a Reversible Bloom Filter with Count-Min Sketch to enable precise reconstruction of both keys AND frequencies from the sketch. Overcomes the "catastrophic information loss" of standard sketches -- you can extract the actual heavy hitter keys, not just verify known keys.
- **Primitives used:** Bloom Filter (have it), Count-Min Sketch (have it)
- **Why we need it:** Standard frequency sketches (Count-Min, Heavy Keeper) can only answer "what is the frequency of key X?" but cannot tell you which keys are heavy hitters without a separate tracking mechanism. Hidden Sketch solves this by making the sketch invertible.
- **Paper:** "Hidden Sketch: A Space-Efficient Reversible Sketch for Tracking Frequent Items" -- arXiv 2025

### 37. HyperCalm Sketch (Periodic Batch Detection)

- **Category:** Composed / Periodicity Detection
- **Year:** 2023 (ICDE)
- **Description:** Two-phase pipeline: (1) HyperBloomFilter -- a time-aware Bloom filter detects batch starts, (2) Calm Space-Saving -- an enhanced top-k tracks periodic batch candidates. 4x better error and 13.2x faster than baselines. Integrated into Apache Flink.
- **Primitives used:** Bloom Filter (have it), Space Saving (have it)
- **Why we need it:** Periodic batch detection is critical for cache prefetching, workload prediction, and anomaly detection. Already integrated into Apache Flink. Composes two primitives we have into a capability no other library offers.
- **Paper:** "HyperCalm Sketch: One-Pass Mining Periodic Batches in Data Streams" -- ICDE 2023
- **Code:** github.com/HyperCalmSketch

---

## Also Notable: Single-Sketch Innovations (2024-2025)

These are not composed sketches but standalone new algorithms from top venues worth tracking:

| Algorithm | Venue | What it does |
|---|---|---|
| **Stable-Sketch** | WWW 2024 (Best Student Paper) | Versatile sketch with stochastic replacement for heavy hitter detection |
| **HeavyLocker** | KDD 2025 | Distributed heavy hitter detection with dynamic threshold locking, 10x error reduction |
| **Bubble Sketch** | CIKM 2024 | High-performance top-k with threshold relocation |
| **DS-FD** | VLDB 2024 (Best Paper Nomination) | Optimal matrix sketching over sliding windows (requires Frequent Directions primitive first) |
