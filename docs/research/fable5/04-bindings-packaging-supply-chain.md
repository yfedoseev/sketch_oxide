# Bindings, Packaging & Supply Chain — Path to SOTA 2026

> Part of the `docs/research/fable5/` improvement research series (2026-07-01).
> Repo state audited: workspace v0.2.0, edition 2021, `rust-toolchain.toml` pins
> `channel = "stable"` only, no `rust-version` anywhere; binding manifests still say
> 0.1.6 pending `scripts/sync-version.sh`. Workflows: `.github/workflows/test.yml`,
> `.github/workflows/publish.yml`.

## Current-state table

| Component | Current | 2026 best practice | Status |
|---|---|---|---|
| PyO3 (`Cargo.toml`) | 0.22, `abi3-py38` | PyO3 **0.29** (Jun 2026); free-threading opt-out default since 0.28; abi3t (PEP 803) in 0.29 ([releases](https://github.com/pyo3/pyo3/releases)) | **7 versions behind** |
| Python floor (`python/pyproject.toml`) | `requires-python >=3.8` | 3.8 EOL Oct 2024, 3.9 EOL Oct 2025 ([devguide](https://devguide.python.org/versions/)); PyO3 0.28+ can't even build for 3.8; peers: polars/pydantic-core ≥3.10, cryptography ≥3.9 | **Obsolete** |
| rust-numpy | 0.22 | numpy crate 0.29 (tracks PyO3 1:1); NumPy 1.x+2.x runtime support since 0.21 | Behind (upgrade in lockstep) |
| Python wheels (`publish.yml`) | 4 wheels: linux x64 gnu, mac x64/arm64, win x64; plain `maturin build`, no manylinux image, no sdist; twine + long-lived token | maturin 1.14 + maturin-action; manylinux_2_17/2_28 docker or zig; **linux aarch64 + musllinux**; sdist always; cp314t wheels; `maturin generate-ci github` template ([distribution docs](https://www.maturin.rs/distribution)) | **Major gaps** (linux wheel likely not even manylinux-tagged) |
| Python type stubs | 8-line placeholder `python/sketch_oxide/__init__.pyi` for 41+ algorithms | Generated stubs: pyo3-stub-gen 0.23, or maturin `--generate-stubs` (experimental, Mar 2026) ([pyo3 type-stub guide](https://pyo3.rs/main/type-stub.html)) | **Effectively missing** |
| napi-rs (`nodejs/`) | napi 2.15, @napi-rs/cli 2.16 | **v3 stable since Jul 2025** (napi 3.10, cli 3.7.2); lifetime-safe API, WASM target ([announcement](https://napi.rs/blog/announce-v3)) | Major version behind; v2 frozen |
| npm distribution | Single package, `prepublishOnly` local build, 4 targets, no `engines` | Per-platform packages via `optionalDependencies` + `napi pre-publish`; 8 standard targets + `wasm32-wasip1-threads` fallback ([template](https://github.com/napi-rs/package-template/blob/main/package.json)) | **Anti-pattern** — consumers off the publisher's platform get nothing prebuilt |
| Node floor | none; CI tests 20/22 | Node 18 EOL Apr 2025, **20 EOL Apr 2026**; supported: 22/24/26 ([endoflife.date](https://endoflife.date/nodejs)) | Missing `engines` field |
| Java (`java/`) | JNI 0.21 hand-written, Java 11 floor, publishing disabled | FFM API final since JDK 22 (JEP 454); JNI still fine for ≤21 users (17+21 ≈ 65% share); floor 17; Lucene-style MR-JAR for FFM path ([JEP 454](https://openjdk.org/jeps/454)) | Dated floor; JNI acceptable but not forward-looking |
| .NET (`dotnet/`) | `[DllImport]`, TFMs `net6.0;net7.0;net8.0;netstandard2.1`, publishing disabled (trimming issue) | `[LibraryImport]` source-gen (NativeAOT/trim-safe); net6/net7 EOL; ship `net8.0` (+`net10.0`), `netstandard2.0` if NetFx matters; `runtimes/{rid}/native` 7-RID set ([best practices](https://learn.microsoft.com/dotnet/standard/native-interop/best-practices)) | **Two EOL TFMs**; DllImport is why trimming breaks |
| Publish auth | Long-lived `CARGO_TOKEN`, PyPI token + twine, `NPM_TOKEN` | Trusted publishing everywhere: crates.io GA Jul 2025, PyPI + PEP 740 attestations, npm OIDC GA Jul 2025 (npm classic tokens revoked Dec 2025 post-Shai-Hulud) ([crates.io](https://crates.io/docs/trusted-publishing), [npm](https://github.blog/changelog/2025-07-31-npm-trusted-publishing-with-oidc-is-generally-available/)) | **NPM_TOKEN path is effectively dead**; all three secrets outdated |
| CI hygiene | cargo-deny on PR only; no semver-checks, no Scorecard; `actions/cache@v3` (deprecated), `setup-python@v4` mixed | + scheduled advisories job, `cargo-semver-checks-action@v2`, `ossf/scorecard-action@v2`, artifact attestations (SLSA L2/L3) | Partial |
| Rust edition / MSRV | 2021, no `rust-version` | Edition 2024 (Rust 1.85, Feb 2025); stable now 1.96.1; declare MSRV + CI check ([1.85 blog](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/)) | Behind |
| Maven publish job | Targets legacy OSSRH/Nexus env vars | OSSRH shut down **Jun 30, 2025** → Central Publisher Portal + `central-publishing-maven-plugin` ([sunset notice](https://central.sonatype.org/news/20250326_ossrh_sunset/)) | **Would fail if ever enabled** |

## Ranked recommendations

### 1. Move publishing to trusted publishing (OIDC) on all registries — Risk: low, Effort: S

The npm token path is dead (classic tokens revoked Dec 9 2025; write tokens
force-expired Feb 2026 after the Shai-Hulud worm, ~800 packages hit in the Nov 2025 wave
— [GitHub changelog](https://github.blog/changelog/2025-09-29-strengthening-npm-security-important-changes-to-authentication-and-token-management/),
[CISA](https://www.cisa.gov/news-events/alerts/2025/09/23/widespread-supply-chain-compromise-impacting-npm-ecosystem)).
Swap: `rust-lang/crates-io-auth-action@v1` for crates.io; `pypa/gh-action-pypi-publish`
(replaces twine; PEP 740 attestations automatic); npm trusted publishing (provenance
automatic). NuGet trusted publishing shipped Sept 2025 for when that publisher is
re-enabled. Config change only — no code risk; **the next release without this may
simply fail to publish to npm**.

### 2. napi-rs 2→3 + real prebuilt distribution — Risk: medium, Effort: M

v3 stable a year now, v2 frozen since Mar 2025. Migration is mostly mechanical for
`#[napi]`-style code (rename `napi.triples`→`targets`; lifetime-safe types matter only
if raw `Js*` handles are used — grep `nodejs/src` for those). The bigger win is fixing
distribution: today `prepublishOnly` builds one local binary, so most npm consumers get
nothing prebuilt. Adopt the CI matrix + `napi pre-publish` optionalDependencies pattern
with the 8 standard targets (adds linux arm64 gnu+musl, linux x64 musl, win arm64) plus
`wasm32-wasip1-threads` fallback — which gets browser/StackBlitz support for free
([WASM docs](https://napi.rs/docs/concepts/webassembly)). Add
`"engines": {"node": ">=20"}` (or ≥22).

### 3. PyO3 0.22→0.29 + Python floor to 3.10 + wheel matrix overhaul — Risk: medium, Effort: M–L

Forced anyway: PyO3 0.28+ dropped 3.8, and 0.29 carries two security fixes. Migration
hot spots across 7 releases: `IntoPyObject` (0.23), `Python::with_gil`→`attach` (0.26),
`FromPyObject` lifetimes (0.27), module-init rework (0.28) — mechanical but broad given
184 bound classes; upgrade the numpy crate to 0.29 in lockstep. Change
`abi3-py38`→`abi3-py310`, `requires-python >=3.10` (matches polars/pydantic-core;
SPEC 0 would even allow 3.11). Wheels: use maturin-action with manylinux_2_28, add
linux aarch64 and musllinux_1_2, always ship sdist; the current bare-`maturin build`
Linux wheel is probably not manylinux-compliant at all. Optionally add cp314t
free-threaded wheels (abi3 can't serve free-threaded builds; PyO3 0.28 made support
opt-out, and abi3t/PEP 803 lands the long-term fix) — sketches are exactly the kind of
parallel-workload library free-threaded users want, and `gil_used = false` needs a
thread-safety review of any interior mutability first.

### 4. Generate real Python type stubs — Risk: low, Effort: S–M

An 8-line `.pyi` for 184 classes means no IDE/mypy support — a visible quality gap vs
peers. Use pyo3-stub-gen 0.23 (PyO3 0.29-compatible) now; switch to maturin's built-in
`--generate-stubs` when it stabilizes. Add `py.typed`.

### 5. Edition 2024, `rust-version`, semver/Scorecard CI — Risk: low, Effort: S

`cargo fix --edition` to 2024 (MSRV floor 1.85; PyO3 0.29 needs ≥1.83 anyway); set
`rust-version = "1.85"` in `[workspace.package]` with a stated N-2 or 6-month policy and
a CI MSRV check; add `obi1kenobi/cargo-semver-checks-action@v2`,
`ossf/scorecard-action@v2` (weekly cron + badge), and a scheduled cargo-deny advisories
job. Also refresh deprecated `actions/cache@v3`/`setup-python@v4` pins.

### 6. .NET modernization (prerequisite to re-enabling NuGet publishing) — Risk: medium, Effort: M

Retarget `net8.0` (+`net10.0` LTS; `netstandard2.0` only if .NET Framework matters —
current `netstandard2.1` is the worst of both worlds). Convert
`[DllImport]`→`[LibraryImport]` in
`dotnet/SketchOxide/src/Native/SketchOxideNative.cs` (code-fixer automates it) — this is
likely the actual fix for the trimming issue that disabled publishing, since DllImport's
runtime marshalling stubs are what break AOT/trim. Package natives under
`runtimes/{rid}/native/` for win-x64/arm64, linux-x64/arm64, linux-musl-x64,
osx-x64/arm64. Note .NET 9's EOL is Nov 10 2026 (STS extended to 24 months), same day
as .NET 8.

### 7. Java: raise floor to 17, fix the publish path; FFM as a later track — Risk: low (floor) / high (FFM rewrite), Effort: S / L

Java 11 is ~15% share and falling; 17+21 ≈ 65% (Azul 2026 survey). Bump
`maven.compiler.release` to 17, keep JNI 0.21 (still legitimate), and bundle
per-platform natives in the jar (sqlite-jdbc pattern). Rewrite the publish job for the
Central Publisher Portal — the current OSSRH/Nexus config targets infrastructure that
shut down Jun 30 2025. FFM (final since JDK 22, ~12% faster calls, 3.4x faster string
marshalling) via Lucene-style multi-release JAR is the forward path but a large rewrite;
defer until Java bindings are actively maintained again — or consider
uniffi-bindgen-java to generate FFM bindings instead of hand-maintaining either.

### 8. WASM/edge package — Risk: low, Effort: M (mostly covered by #2)

The napi-rs v3 wasm32-wasip1-threads fallback delivers browser/Node-fallback WASM with
zero extra binding code. Cloudflare Workers is a genuinely good fit for sketches (edge
counting/dedup) but requires threadless `wasm32-unknown-unknown` via workers-rs — worth
a `cargo check --target wasm32-unknown-unknown` on the core crate to confirm
compatibility (core deps look pure-compute), then a `@sketch-oxide/wasm` wasm-bindgen
package only if demand appears. `wasm32-wasip2` targets server-side component-model
runtimes, not npm/browsers — skip.

**Not recommended:** cargo-dist — it survived the axo wind-down (upstream v0.32.0,
May 2026, re-merged the astral fork) but adds little for a registry-driven library;
trusted publishing + per-registry attestations is the better 2026 baseline.

## Suggested sequencing

- **#1 and #5** are quick config-level wins for the v0.2.0 release branch.
- **#2 and #3** are the substantive modernizations for a v0.3.0; **#4** rides along
  with #3.
- **#6–#8** gate on whether .NET/Java/edge distribution is strategically wanted.
