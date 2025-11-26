# sketch_oxide Python Bindings

State-of-the-art DataSketches library with Python bindings via PyO3.

## Installation

```bash
pip install sketch-oxide
```

## Development

```bash
cd python
pip install maturin
maturin develop --release
pytest
```

## Features

- UltraLogLog (28% more efficient than HyperLogLog)
- Binary Fuse Filter (75% more efficient than Bloom Filter)
- DDSketch (modern quantiles with relative error)
- And more...

## Quick Start

```python
import sketch_oxide

# More examples coming soon as we implement algorithms
print(f"sketch_oxide version: {sketch_oxide.__version__}")
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../LICENSE-MIT))

at your option.
