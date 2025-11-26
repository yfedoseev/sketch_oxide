# Contributing to sketch_oxide

Thank you for your interest in contributing to sketch_oxide! This document provides guidelines for contributing to the project.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How Can I Contribute?](#how-can-i-contribute)
- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Submitting Changes](#submitting-changes)
- [Documentation](#documentation)
- [Performance Considerations](#performance-considerations)

---

## Code of Conduct

This project adheres to a code of conduct that all contributors are expected to follow:

- **Be respectful**: Treat all contributors with respect and professionalism
- **Be constructive**: Provide helpful, actionable feedback
- **Be collaborative**: Work together to improve the project
- **Be inclusive**: Welcome contributors of all backgrounds and skill levels

---

## How Can I Contribute?

### Reporting Bugs

Before creating a bug report, please check existing issues to avoid duplicates.

**Good bug reports include**:
- Clear, descriptive title
- Exact steps to reproduce
- Expected vs actual behavior
- Code sample demonstrating the issue
- Environment details (OS, Rust version, Python version)

**Example**:
```markdown
## Bug: UltraLogLog estimate() returns NaN for empty sketch

### Environment
- OS: Linux (Ubuntu 22.04)
- Rust: 1.70.0
- sketch_oxide: 0.1.0

### Steps to Reproduce
```rust
let ull = UltraLogLog::new(12)?;
println!("{}", ull.estimate()); // Prints NaN
```

### Expected
Should return 0.0 for empty sketch

### Actual
Returns NaN
```

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. Include:

- Clear description of the enhancement
- Use case / motivation
- Proposed API (if applicable)
- Performance impact considerations
- References to research papers (if applicable)

### Pull Requests

We actively welcome pull requests for:

- Bug fixes
- Performance improvements
- Documentation improvements
- New algorithms (with research backing)
- Test coverage improvements

---

## Development Setup

### Prerequisites

```bash
# Rust (latest stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Python 3.8+ (for Python bindings)
python3 --version

# Maturin (for Python package building)
pip install maturin

# Pre-commit (for code quality)
pip install pre-commit
```

### Clone and Build

```bash
# Clone repository
git clone https://github.com/yourusername/sketch_oxide.git
cd sketch_oxide

# Install pre-commit hooks
pre-commit install

# Build Rust library
cargo build --release

# Run tests
cargo test --all-features

# Build Python bindings
cd python
maturin develop --release
pytest -v
```

---

## Coding Standards

### Rust Code Style

**We follow standard Rust conventions**:

1. **Use rustfmt**: All code must pass `cargo fmt --all --check`
2. **Use clippy**: All code must pass `cargo clippy --all-targets --all-features -- -D warnings`
3. **Follow naming conventions**:
   - `snake_case` for functions and variables
   - `PascalCase` for types and traits
   - `SCREAMING_SNAKE_CASE` for constants

### Code Organization

```rust
// Good: Clear, well-organized
pub struct UltraLogLog {
    precision: u8,
    registers: Vec<u8>,
}

impl UltraLogLog {
    /// Creates a new UltraLogLog sketch.
    ///
    /// # Arguments
    /// * `precision` - Precision parameter (4-18)
    ///
    /// # Errors
    /// Returns error if precision is out of range
    pub fn new(precision: u8) -> Result<Self, SketchError> {
        if precision < 4 || precision > 18 {
            return Err(SketchError::InvalidParameter(
                format!("Precision must be 4-18, got {}", precision)
            ));
        }

        Ok(Self {
            precision,
            registers: vec![0; 1 << precision],
        })
    }
}
```

### Error Handling

- Use `Result<T, SketchError>` for fallible operations
- Provide descriptive error messages
- Document error conditions in doc comments

```rust
// Good: Clear error handling
pub fn quantile(&self, q: f64) -> Result<f64, SketchError> {
    if q < 0.0 || q > 1.0 {
        return Err(SketchError::InvalidParameter(
            format!("Quantile must be 0.0-1.0, got {}", q)
        ));
    }
    // ...
}
```

### Documentation

**All public APIs must have doc comments**:

```rust
/// Estimates the cardinality (number of unique items).
///
/// Returns an approximation of the number of unique items added to the sketch.
/// The relative error is approximately 1.04 / sqrt(2^precision).
///
/// # Examples
///
/// ```
/// use sketch_oxide::cardinality::UltraLogLog;
///
/// let mut ull = UltraLogLog::new(12)?;
/// for i in 0..1000 {
///     ull.update(&i);
/// }
/// assert!((ull.estimate() - 1000.0).abs() < 50.0);
/// # Ok::<(), sketch_oxide::SketchError>(())
/// ```
///
/// # Performance
///
/// This operation is O(2^precision) in time complexity.
pub fn estimate(&self) -> f64 {
    // Implementation
}
```

---

## Testing Guidelines

### Test-Driven Development (TDD)

**We follow TDD methodology**:

1. Write failing tests first
2. Implement minimal code to pass tests
3. Refactor while keeping tests passing
4. Add property-based tests for robustness

### Test Categories

#### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_valid_precision() {
        let ull = UltraLogLog::new(12);
        assert!(ull.is_ok());
    }

    #[test]
    fn test_new_invalid_precision() {
        let ull = UltraLogLog::new(3);
        assert!(ull.is_err());
    }
}
```

#### Integration Tests

```rust
// tests/integration_test.rs
use sketch_oxide::prelude::*;

#[test]
fn test_ultraloglog_basic_workflow() {
    let mut ull = UltraLogLog::new(14).unwrap();

    // Add items
    for i in 0..10000 {
        ull.update(&i);
    }

    // Verify estimate
    let est = ull.estimate();
    let error = (est - 10000.0).abs() / 10000.0;
    assert!(error < 0.02); // <2% error
}
```

#### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_estimate_never_negative(items in prop::collection::vec(any::<u64>(), 0..10000)) {
        let mut ull = UltraLogLog::new(12).unwrap();
        for item in items {
            ull.update(&item);
        }
        assert!(ull.estimate() >= 0.0);
    }
}
```

### Test Requirements

All pull requests must:

- Include tests for new functionality
- Maintain or improve code coverage
- Pass all existing tests
- Include property-based tests for core algorithms

---

## Submitting Changes

### Branch Naming

Use descriptive branch names:

```
feature/add-kll-sketch
fix/ultraloglog-overflow
docs/improve-readme
perf/optimize-binary-fuse
```

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(ultraloglog): implement sparse mode for small cardinalities
fix(ddsketch): correct quantile calculation for negative values
docs(readme): add Python NumPy integration examples
perf(binary_fuse): optimize peeling algorithm by 20%
test(count_min): add property-based tests for merge operation
```

### Pull Request Process

1. **Fork and create branch**:
   ```bash
   git checkout -b feature/your-feature
   ```

2. **Make changes**:
   - Write tests first (TDD)
   - Implement functionality
   - Ensure all tests pass
   - Run formatter and linter

3. **Verify quality**:
   ```bash
   cargo fmt --all --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-features
   cargo bench  # if performance-related
   ```

4. **Commit changes**:
   ```bash
   git add .
   git commit -m "feat(algorithm): add new feature"
   ```

5. **Push and create PR**:
   ```bash
   git push origin feature/your-feature
   ```

6. **PR Description Template**:
   ```markdown
   ## Description
   Brief description of changes

   ## Motivation
   Why is this change needed?

   ## Changes
   - Change 1
   - Change 2

   ## Testing
   - [ ] Unit tests added/updated
   - [ ] Integration tests added/updated
   - [ ] Benchmarks added/updated (if performance-related)
   - [ ] All tests pass

   ## Documentation
   - [ ] Code comments updated
   - [ ] README updated (if needed)
   - [ ] Examples added/updated (if needed)

   ## Performance Impact
   Describe any performance implications
   ```

---

## Documentation

### Inline Documentation

- All public APIs must have doc comments
- Include examples in doc comments
- Document error conditions
- Note performance characteristics

### README and Guides

When updating documentation:

- Keep examples simple and runnable
- Provide context for why something is useful
- Link to relevant research papers
- Include performance benchmarks where relevant

---

## Performance Considerations

### Benchmarking

**All performance-sensitive code must be benchmarked**:

```rust
// benches/my_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sketch_oxide::cardinality::UltraLogLog;

fn ultraloglog_update_benchmark(c: &mut Criterion) {
    let mut ull = UltraLogLog::new(12).unwrap();

    c.bench_function("ultraloglog_update", |b| {
        b.iter(|| {
            ull.update(black_box(&12345u64));
        });
    });
}

criterion_group!(benches, ultraloglog_update_benchmark);
criterion_main!(benches);
```

### Performance Targets

When adding new algorithms, aim to:

- **Meet or exceed research paper targets**
- **Be competitive with reference implementations**
- **Maintain <100ns for simple operations**
- **Scale gracefully with data size**

### Optimization Guidelines

1. **Profile first**: Use `cargo flamegraph` or similar
2. **Benchmark before and after**: Use Criterion.rs
3. **Document trade-offs**: Space vs time vs accuracy
4. **Avoid premature optimization**: Correctness first

---

## Adding New Algorithms

### Research-Backed Algorithms Only

New algorithms must:

1. **Be published in peer-reviewed venues** (VLDB, SIGMOD, ACM, IEEE, etc.)
2. **Provide measurable improvements** over existing alternatives
3. **Have clear use cases** distinct from existing algorithms
4. **Be production-proven** (ideally) or have strong theoretical backing

### Algorithm Addition Checklist

- [ ] Research paper linked in issue/PR
- [ ] Performance comparison with existing algorithms
- [ ] Use case analysis
- [ ] Full TDD implementation
- [ ] Comprehensive tests (unit + integration + property-based)
- [ ] Benchmarks demonstrating performance
- [ ] Documentation with examples
- [ ] Python bindings (if applicable)

### Example: Adding New Algorithm

```markdown
## Proposal: Add KLL Sketch

### Research
- Paper: "Optimal Quantile Approximation in Streams" (FOCS 2016)
- Authors: Zohar Karnin, Kevin Lang, Edo Liberty
- Venue: IEEE Symposium on Foundations of Computer Science

### Motivation
KLL provides absolute error guarantees (vs DDSketch's relative error),
useful for formal verification scenarios.

### Performance Targets
- Update: <100ns
- Quantile query: <10µs
- Space: O(1/ε × log n)

### Use Cases
- Formal verification requiring absolute error bounds
- Complement to DDSketch (different error model)

### Implementation Plan
1. Core algorithm (1-2 weeks)
2. Tests (1 week)
3. Benchmarks (3-5 days)
4. Python bindings (1 week)
5. Documentation (3-5 days)

Total: 4-6 weeks
```

---

## Python Bindings

### PyO3 Guidelines

When adding Python bindings:

1. **Match Rust API closely** but use Pythonic conventions
2. **Handle Python type conversions** properly
3. **Provide good error messages** for Python users
4. **Add type stubs** for IDE support
5. **Include doctests** in Python docstrings

### Example Python Binding

```rust
use pyo3::prelude::*;

#[pyclass]
pub struct UltraLogLog {
    inner: RustUltraLogLog,
}

#[pymethods]
impl UltraLogLog {
    #[new]
    fn new(precision: u8) -> PyResult<Self> {
        RustUltraLogLog::new(precision)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        let hash = hash_python_item(item)?;
        self.inner.update(&hash);
        Ok(())
    }

    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }
}
```

---

## Getting Help

- **GitHub Issues**: For bugs, features, and questions
- **GitHub Discussions**: For design discussions and ideas
- **Email**: [maintainer email] for private inquiries

---

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).

---

## Acknowledgments

Thank you for contributing to sketch_oxide! Your contributions help make this the best DataSketches library for Rust and Python.

**No nostalgia. Just the best algorithms available in 2025.**
