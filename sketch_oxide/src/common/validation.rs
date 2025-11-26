//! Validation utilities for sketch deserialization and parameter bounds checking

use crate::common::{Result, SketchError};

/// Maximum capacity for any sketch (2^31 - 1, ~2.1 billion items)
pub const MAX_CAPACITY: u64 = (1u64 << 31) - 1;

/// Maximum serialized sketch size (256MB) to prevent resource exhaustion
pub const MAX_BYTE_SIZE: usize = 256 * 1024 * 1024; // 256MB

/// Validate that precision is within acceptable range (4-18)
/// Typically used for HyperLogLog, UltraLogLog, and similar cardinality sketches
pub fn validate_precision(precision: u8) -> Result<()> {
    if !(4..=18).contains(&precision) {
        return Err(SketchError::InvalidParameter {
            param: "precision".to_string(),
            value: precision.to_string(),
            constraint: "must be in range [4, 18]".to_string(),
        });
    }
    Ok(())
}

/// Validate that capacity is positive and within limits
pub fn validate_capacity(capacity: u64) -> Result<()> {
    if capacity == 0 {
        return Err(SketchError::InvalidParameter {
            param: "capacity".to_string(),
            value: capacity.to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    if capacity > MAX_CAPACITY {
        return Err(SketchError::InvalidParameter {
            param: "capacity".to_string(),
            value: capacity.to_string(),
            constraint: format!("must not exceed {}", MAX_CAPACITY),
        });
    }
    Ok(())
}

/// Validate that a value is a valid probability (0.0 < p < 1.0)
/// Typically used for epsilon (error rate) and delta (confidence)
pub fn validate_probability(value: f64, param_name: &str) -> Result<()> {
    if !(0.0 < value && value < 1.0) {
        return Err(SketchError::InvalidParameter {
            param: param_name.to_string(),
            value: value.to_string(),
            constraint: "must be in range (0.0, 1.0) (exclusive)".to_string(),
        });
    }
    Ok(())
}

/// Validate that a deserialized byte size doesn't exceed safety limits
pub fn validate_byte_size(size: usize) -> Result<()> {
    if size > MAX_BYTE_SIZE {
        return Err(SketchError::DeserializationError(format!(
            "Deserialized sketch size {} exceeds maximum allowed size {}",
            size, MAX_BYTE_SIZE
        )));
    }
    Ok(())
}

/// Validate minimum required bytes for deserialization header
pub fn validate_min_size(actual: usize, required: usize) -> Result<()> {
    if actual < required {
        return Err(SketchError::DeserializationError(format!(
            "Insufficient data: need at least {} bytes, got {}",
            required, actual
        )));
    }
    Ok(())
}

/// Validate that width and depth are reasonable for Count-Min Sketch
pub fn validate_width_depth(width: u32, depth: u32) -> Result<()> {
    // Reasonable bounds: width and depth should each be at least 1 and at most 2^20 (1M)
    const MAX_DIM: u32 = 1 << 20; // 1,048,576

    if width == 0 {
        return Err(SketchError::InvalidParameter {
            param: "width".to_string(),
            value: width.to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    if width > MAX_DIM {
        return Err(SketchError::InvalidParameter {
            param: "width".to_string(),
            value: width.to_string(),
            constraint: format!("must not exceed {}", MAX_DIM),
        });
    }

    if depth == 0 {
        return Err(SketchError::InvalidParameter {
            param: "depth".to_string(),
            value: depth.to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    if depth > MAX_DIM {
        return Err(SketchError::InvalidParameter {
            param: "depth".to_string(),
            value: depth.to_string(),
            constraint: format!("must not exceed {}", MAX_DIM),
        });
    }

    Ok(())
}

/// Validate Bloom Filter parameters
pub fn validate_bloom_parameters(n: u64, m: u64, k: u32) -> Result<()> {
    // Validate capacity n
    validate_capacity(n)?;

    // Validate bit array size m
    if m == 0 {
        return Err(SketchError::InvalidParameter {
            param: "m (bit array size)".to_string(),
            value: m.to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    if m > (1u64 << 32) {
        return Err(SketchError::InvalidParameter {
            param: "m (bit array size)".to_string(),
            value: m.to_string(),
            constraint: "must not exceed 2^32".to_string(),
        });
    }

    // Validate hash function count k
    if k == 0 {
        return Err(SketchError::InvalidParameter {
            param: "k (hash functions)".to_string(),
            value: k.to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    if k > 64 {
        return Err(SketchError::InvalidParameter {
            param: "k (hash functions)".to_string(),
            value: k.to_string(),
            constraint: "must not exceed 64".to_string(),
        });
    }

    Ok(())
}

/// Validate CPC Sketch lg_k parameter
pub fn validate_lg_k(lg_k: u8) -> Result<()> {
    if !(4..=26).contains(&lg_k) {
        return Err(SketchError::InvalidParameter {
            param: "lg_k".to_string(),
            value: lg_k.to_string(),
            constraint: "must be in range [4, 26]".to_string(),
        });
    }
    Ok(())
}

/// Validate Vacuum Filter capacity
pub fn validate_vacuum_capacity(capacity: u32) -> Result<()> {
    if capacity == 0 {
        return Err(SketchError::InvalidParameter {
            param: "capacity".to_string(),
            value: capacity.to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    if capacity > (1u32 << 30) {
        // ~1 billion max for safety
        return Err(SketchError::InvalidParameter {
            param: "capacity".to_string(),
            value: capacity.to_string(),
            constraint: "must not exceed 2^30".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_precision_valid() {
        assert!(validate_precision(4).is_ok());
        assert!(validate_precision(10).is_ok());
        assert!(validate_precision(18).is_ok());
    }

    #[test]
    fn test_validate_precision_invalid() {
        assert!(validate_precision(3).is_err());
        assert!(validate_precision(19).is_err());
    }

    #[test]
    fn test_validate_capacity_valid() {
        assert!(validate_capacity(1).is_ok());
        assert!(validate_capacity(1_000_000).is_ok());
        assert!(validate_capacity(MAX_CAPACITY).is_ok());
    }

    #[test]
    fn test_validate_capacity_invalid() {
        assert!(validate_capacity(0).is_err());
        assert!(validate_capacity(MAX_CAPACITY + 1).is_err());
    }

    #[test]
    fn test_validate_probability_valid() {
        assert!(validate_probability(0.1, "epsilon").is_ok());
        assert!(validate_probability(0.5, "delta").is_ok());
        assert!(validate_probability(0.99, "confidence").is_ok());
    }

    #[test]
    fn test_validate_probability_invalid() {
        assert!(validate_probability(0.0, "epsilon").is_err());
        assert!(validate_probability(1.0, "delta").is_err());
        assert!(validate_probability(-0.1, "value").is_err());
    }

    #[test]
    fn test_validate_bloom_parameters_valid() {
        assert!(validate_bloom_parameters(1000, 10000, 7).is_ok());
    }

    #[test]
    fn test_validate_bloom_parameters_invalid() {
        assert!(validate_bloom_parameters(0, 10000, 7).is_err()); // n = 0
        assert!(validate_bloom_parameters(1000, 0, 7).is_err()); // m = 0
        assert!(validate_bloom_parameters(1000, 10000, 0).is_err()); // k = 0
    }
}
