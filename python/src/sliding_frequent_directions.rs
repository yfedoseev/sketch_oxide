//! Python bindings for Sliding-Window Frequent Directions.

use pyo3::prelude::*;
use sketch_oxide::matrix::SlidingFrequentDirections as RustSlidingFrequentDirections;

/// SlidingFrequentDirections — Frequent Directions over a sliding window of rows,
/// maintaining `num_blocks` block sketches of `rows_per_block` rows each so the
/// covariance reflects only the most recent window.
///
/// Args:
///     ell (int): sketch rows retained per block.
///     d (int): number of columns (row dimension).
///     rows_per_block (int): rows per block (expiry granularity).
///     num_blocks (int): number of blocks kept in the window.
#[pyclass(module = "sketch_oxide")]
pub struct SlidingFrequentDirections {
    inner: RustSlidingFrequentDirections,
}

#[pymethods]
impl SlidingFrequentDirections {
    #[new]
    fn new(ell: usize, d: usize, rows_per_block: usize, num_blocks: usize) -> PyResult<Self> {
        RustSlidingFrequentDirections::new(ell, d, rows_per_block, num_blocks)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Appends a `d`-dimensional row to the current window.
    fn append(&mut self, row: Vec<f64>) {
        self.inner.append(&row);
    }

    /// Approximated covariance matrix `Aᵀ A` over the current window (`d × d`).
    fn windowed_covariance(&self) -> Vec<Vec<f64>> {
        self.inner.windowed_covariance()
    }

    /// Number of sketch rows per block.
    fn ell(&self) -> usize {
        self.inner.ell()
    }

    /// Row dimension.
    fn dim(&self) -> usize {
        self.inner.dim()
    }

    /// Window length in rows.
    fn window(&self) -> usize {
        self.inner.window()
    }

    fn __repr__(&self) -> String {
        format!(
            "SlidingFrequentDirections(ell={}, dim={}, window={})",
            self.inner.ell(),
            self.inner.dim(),
            self.inner.window()
        )
    }
}
