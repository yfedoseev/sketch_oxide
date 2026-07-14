//! OTLP protobuf encoding + Prometheus native-histogram conversion for the OTel
//! exponential histogram (fable5 doc 03 C.2 / doc 00 C6).
//!
//! [`OtelExponentialHistogram`] already implements the base-2 exponential
//! histogram; this module makes it *shippable*:
//! - [`OtelExponentialHistogram::to_otlp_data_point`] encodes it as an OTLP
//!   `ExponentialHistogramDataPoint` protobuf message (the de-facto wire format
//!   of modern observability), using the standardized OTLP field numbers.
//! - [`OtlpExponentialHistogramDataPoint::decode`] parses that wire form back,
//!   so the codec is self-checking (encode→decode round-trips losslessly).
//! - [`OtelExponentialHistogram::to_prometheus_native`] converts to the
//!   Prometheus native-histogram shape (spans + delta-encoded counts); the
//!   span/delta encoding is verified lossless by reconstruction in tests.
//!
//! The protobuf writer/reader here is a minimal hand-rolled implementation (no
//! `prost`/`protoc` dependency) covering exactly the wire types the message uses.

use crate::common::{ReadCursor, Result, SketchError, WriteBuf};
use crate::quantiles::OtelExponentialHistogram;

// ---------------------------------------------------------------------------
// Minimal protobuf primitives
// ---------------------------------------------------------------------------

const WIRE_VARINT: u32 = 0;
const WIRE_I64: u32 = 1;
const WIRE_LEN: u32 = 2;

fn write_varint(buf: &mut WriteBuf, mut v: u64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            buf.write_u8(byte | 0x80);
        } else {
            buf.write_u8(byte);
            break;
        }
    }
}

fn write_tag(buf: &mut WriteBuf, field: u32, wire: u32) {
    write_varint(buf, u64::from((field << 3) | wire));
}

/// ZigZag-encode a signed 32-bit integer (protobuf `sint32`).
fn zigzag32(v: i32) -> u64 {
    ((v << 1) ^ (v >> 31)) as u32 as u64
}

fn unzigzag32(v: u64) -> i32 {
    let v = v as u32;
    ((v >> 1) as i32) ^ -((v & 1) as i32)
}

fn write_fixed64_field(buf: &mut WriteBuf, field: u32, v: u64) {
    write_tag(buf, field, WIRE_I64);
    buf.write_u64_le(v);
}

fn write_double_field(buf: &mut WriteBuf, field: u32, v: f64) {
    write_fixed64_field(buf, field, v.to_bits());
}

fn write_sint32_field(buf: &mut WriteBuf, field: u32, v: i32) {
    write_tag(buf, field, WIRE_VARINT);
    write_varint(buf, zigzag32(v));
}

fn write_len_field(buf: &mut WriteBuf, field: u32, payload: &[u8]) {
    write_tag(buf, field, WIRE_LEN);
    write_varint(buf, payload.len() as u64);
    buf.write_bytes(payload);
}

fn read_varint(cur: &mut ReadCursor) -> Result<u64> {
    let mut result = 0u64;
    let mut shift = 0u32;
    loop {
        let byte = cur.read_u8()?;
        if shift >= 64 {
            return Err(SketchError::DeserializationError("varint overflow".into()));
        }
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// OTLP message types (subset of ExponentialHistogramDataPoint)
// ---------------------------------------------------------------------------

/// A run of exponential buckets (`ExponentialHistogramDataPoint.Buckets`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OtlpBuckets {
    /// Index of the first bucket (`sint32 offset = 1`).
    pub offset: i32,
    /// Dense per-bucket counts (`repeated uint64 bucket_counts = 2`, packed).
    pub bucket_counts: Vec<u64>,
}

/// A decoded OTLP `ExponentialHistogramDataPoint` (the fields sketch_oxide fills).
#[derive(Debug, Clone, PartialEq)]
pub struct OtlpExponentialHistogramDataPoint {
    /// `fixed64 time_unix_nano = 3`.
    pub time_unix_nano: u64,
    /// `fixed64 count = 4`.
    pub count: u64,
    /// `optional double sum = 5`.
    pub sum: Option<f64>,
    /// `sint32 scale = 6`.
    pub scale: i32,
    /// `fixed64 zero_count = 7`.
    pub zero_count: u64,
    /// `Buckets positive = 8`.
    pub positive: OtlpBuckets,
    /// `Buckets negative = 9`.
    pub negative: OtlpBuckets,
    /// `optional double min = 12`.
    pub min: Option<f64>,
    /// `optional double max = 13`.
    pub max: Option<f64>,
    /// `double zero_threshold = 14`.
    pub zero_threshold: f64,
}

fn encode_buckets(field: u32, buf: &mut WriteBuf, offset: i32, counts: &[u64]) {
    if counts.is_empty() {
        return;
    }
    let mut inner = WriteBuf::new();
    write_sint32_field(&mut inner, 1, offset);
    // Packed repeated uint64: one length-delimited field holding concatenated varints.
    let mut packed = WriteBuf::new();
    for &c in counts {
        write_varint(&mut packed, c);
    }
    write_len_field(&mut inner, 2, &packed.into_bytes());
    write_len_field(buf, field, &inner.into_bytes());
}

impl OtelExponentialHistogram {
    /// Encodes this histogram as an OTLP `ExponentialHistogramDataPoint`
    /// protobuf message with the given observation timestamp.
    #[must_use]
    pub fn to_otlp_data_point(&self, time_unix_nano: u64) -> Vec<u8> {
        let mut buf = WriteBuf::new();
        write_fixed64_field(&mut buf, 3, time_unix_nano);
        write_fixed64_field(&mut buf, 4, self.count());
        write_double_field(&mut buf, 5, self.sum());
        write_sint32_field(&mut buf, 6, self.scale());
        write_fixed64_field(&mut buf, 7, self.zero_count());
        encode_buckets(8, &mut buf, self.positive_offset(), self.positive_counts());
        encode_buckets(9, &mut buf, self.negative_offset(), self.negative_counts());
        if let Some(m) = self.min() {
            write_double_field(&mut buf, 12, m);
        }
        if let Some(m) = self.max() {
            write_double_field(&mut buf, 13, m);
        }
        write_double_field(&mut buf, 14, 0.0); // zero_threshold: exact-zero bucket
        buf.into_bytes()
    }
}

impl OtlpExponentialHistogramDataPoint {
    /// Decodes an OTLP `ExponentialHistogramDataPoint` message, ignoring fields
    /// this subset does not model (e.g. attributes, exemplars).
    ///
    /// # Errors
    /// Returns `DeserializationError` on malformed input (panic-free).
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut dp = OtlpExponentialHistogramDataPoint {
            time_unix_nano: 0,
            count: 0,
            sum: None,
            scale: 0,
            zero_count: 0,
            positive: OtlpBuckets::default(),
            negative: OtlpBuckets::default(),
            min: None,
            max: None,
            zero_threshold: 0.0,
        };
        let mut cur = ReadCursor::new(bytes);
        while cur.remaining() > 0 {
            let tag = read_varint(&mut cur)?;
            let field = (tag >> 3) as u32;
            let wire = (tag & 0x7) as u32;
            match (field, wire) {
                (3, WIRE_I64) => dp.time_unix_nano = cur.read_u64_le()?,
                (4, WIRE_I64) => dp.count = cur.read_u64_le()?,
                (5, WIRE_I64) => dp.sum = Some(f64::from_bits(cur.read_u64_le()?)),
                (6, WIRE_VARINT) => dp.scale = unzigzag32(read_varint(&mut cur)?),
                (7, WIRE_I64) => dp.zero_count = cur.read_u64_le()?,
                (8, WIRE_LEN) => dp.positive = decode_buckets(&mut cur)?,
                (9, WIRE_LEN) => dp.negative = decode_buckets(&mut cur)?,
                (12, WIRE_I64) => dp.min = Some(f64::from_bits(cur.read_u64_le()?)),
                (13, WIRE_I64) => dp.max = Some(f64::from_bits(cur.read_u64_le()?)),
                (14, WIRE_I64) => dp.zero_threshold = f64::from_bits(cur.read_u64_le()?),
                // Unknown field: skip by wire type.
                (_, WIRE_VARINT) => {
                    read_varint(&mut cur)?;
                }
                (_, WIRE_I64) => {
                    cur.read_u64_le()?;
                }
                (_, WIRE_LEN) => {
                    let len = read_varint(&mut cur)? as usize;
                    cur.read_bytes(len)?;
                }
                _ => {
                    return Err(SketchError::DeserializationError(format!(
                        "unsupported wire type {wire} for field {field}"
                    )));
                }
            }
        }
        Ok(dp)
    }
}

fn decode_buckets(cur: &mut ReadCursor) -> Result<OtlpBuckets> {
    let len = read_varint(cur)? as usize;
    let inner = cur.read_bytes(len)?;
    let mut b = OtlpBuckets::default();
    let mut c = ReadCursor::new(inner);
    while c.remaining() > 0 {
        let tag = read_varint(&mut c)?;
        let field = (tag >> 3) as u32;
        let wire = (tag & 0x7) as u32;
        match (field, wire) {
            (1, WIRE_VARINT) => b.offset = unzigzag32(read_varint(&mut c)?),
            (2, WIRE_LEN) => {
                let plen = read_varint(&mut c)? as usize;
                let packed = c.read_bytes(plen)?;
                let mut pc = ReadCursor::new(packed);
                while pc.remaining() > 0 {
                    b.bucket_counts.push(read_varint(&mut pc)?);
                }
            }
            (_, WIRE_VARINT) => {
                read_varint(&mut c)?;
            }
            (_, WIRE_LEN) => {
                let l = read_varint(&mut c)? as usize;
                c.read_bytes(l)?;
            }
            _ => {
                return Err(SketchError::DeserializationError(
                    "bad wire type in Buckets".into(),
                ));
            }
        }
    }
    Ok(b)
}

// ---------------------------------------------------------------------------
// Prometheus native histogram
// ---------------------------------------------------------------------------

/// A Prometheus native-histogram span: `length` consecutive populated buckets
/// starting `offset` buckets after the previous span's end (or bucket 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrometheusSpan {
    /// Gap (in bucket indices) from the previous span's end to this span's start.
    pub offset: i32,
    /// Number of consecutive buckets in this span.
    pub length: u32,
}

/// The Prometheus native-histogram representation of an exponential histogram.
///
/// `schema` equals the OTel `scale` (both use base `2^(2^-schema)`). Bucket
/// counts are delta-encoded: `deltas[0]` is the first bucket's absolute count,
/// each subsequent value the signed difference from the previous bucket.
#[derive(Debug, Clone, PartialEq)]
pub struct PrometheusNativeHistogram {
    /// Resolution parameter (= OTel scale).
    pub schema: i32,
    /// Count of observations in the zero bucket.
    pub zero_count: u64,
    /// Half-open width of the zero bucket.
    pub zero_threshold: f64,
    /// Total observation count.
    pub count: u64,
    /// Sum of observations.
    pub sum: f64,
    /// Positive-bucket spans.
    pub positive_spans: Vec<PrometheusSpan>,
    /// Positive-bucket delta-encoded counts.
    pub positive_deltas: Vec<i64>,
    /// Negative-bucket spans.
    pub negative_spans: Vec<PrometheusSpan>,
    /// Negative-bucket delta-encoded counts.
    pub negative_deltas: Vec<i64>,
}

/// Converts a dense `(offset, counts)` OTel run into Prometheus spans + deltas.
///
/// Prometheus bucket index = OTel bucket index + 1. Zero-count buckets are
/// skipped (they start/end spans); the returned encoding is lossless — the tests
/// reconstruct the dense counts from it and compare.
fn to_spans_deltas(offset: i32, counts: &[u64]) -> (Vec<PrometheusSpan>, Vec<i64>) {
    let mut spans = Vec::new();
    let mut deltas = Vec::new();
    let mut prev_count = 0i64;
    let mut prev_end_index: Option<i32> = None; // Prometheus index of last emitted bucket
    let mut i = 0usize;
    while i < counts.len() {
        if counts[i] == 0 {
            i += 1;
            continue;
        }
        // Start of a run of consecutive non-zero buckets.
        let run_start = i;
        while i < counts.len() && counts[i] != 0 {
            i += 1;
        }
        let start_index = offset + run_start as i32 + 1; // +1: OTel→Prometheus index
        let span_offset = match prev_end_index {
            None => start_index,
            Some(end) => start_index - end - 1,
        };
        spans.push(PrometheusSpan {
            offset: span_offset,
            length: (i - run_start) as u32,
        });
        for &c in &counts[run_start..i] {
            deltas.push(c as i64 - prev_count);
            prev_count = c as i64;
        }
        prev_end_index = Some(offset + (i - 1) as i32 + 1);
    }
    (spans, deltas)
}

impl OtelExponentialHistogram {
    /// Converts this histogram to the Prometheus native-histogram shape.
    #[must_use]
    pub fn to_prometheus_native(&self) -> PrometheusNativeHistogram {
        let (positive_spans, positive_deltas) =
            to_spans_deltas(self.positive_offset(), self.positive_counts());
        let (negative_spans, negative_deltas) =
            to_spans_deltas(self.negative_offset(), self.negative_counts());
        PrometheusNativeHistogram {
            schema: self.scale(),
            zero_count: self.zero_count(),
            zero_threshold: 0.0,
            count: self.count(),
            sum: self.sum(),
            positive_spans,
            positive_deltas,
            negative_spans,
            negative_deltas,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_histogram() -> OtelExponentialHistogram {
        let mut h = OtelExponentialHistogram::new(3, 160).unwrap();
        for i in 1..=1000u64 {
            h.record(i as f64);
            h.record(-(i as f64) * 0.5);
        }
        h.record(0.0);
        h
    }

    #[test]
    fn otlp_encode_decode_round_trips() {
        let h = sample_histogram();
        let bytes = h.to_otlp_data_point(1_700_000_000_000_000_000);
        let dp = OtlpExponentialHistogramDataPoint::decode(&bytes).expect("decode");

        assert_eq!(dp.time_unix_nano, 1_700_000_000_000_000_000);
        assert_eq!(dp.count, h.count());
        assert_eq!(dp.sum, Some(h.sum()));
        assert_eq!(dp.scale, h.scale());
        assert_eq!(dp.zero_count, h.zero_count());
        assert_eq!(dp.positive.offset, h.positive_offset());
        assert_eq!(dp.positive.bucket_counts, h.positive_counts());
        assert_eq!(dp.negative.offset, h.negative_offset());
        assert_eq!(dp.negative.bucket_counts, h.negative_counts());
        assert_eq!(dp.min, h.min());
        assert_eq!(dp.max, h.max());
    }

    #[test]
    fn otlp_decode_rejects_malformed_bytes_without_panic() {
        // Truncated varint / dangling length must error, not panic.
        assert!(OtlpExponentialHistogramDataPoint::decode(&[0xff, 0xff, 0xff]).is_err());
        // Field 8 (positive) length-delimited claiming more bytes than present.
        assert!(OtlpExponentialHistogramDataPoint::decode(&[(8 << 3) | 2, 200]).is_err());
    }

    #[test]
    fn zigzag_is_invertible() {
        for v in [0i32, 1, -1, 2, -2, i32::MIN, i32::MAX, -12345, 6789] {
            assert_eq!(unzigzag32(zigzag32(v)), v);
        }
    }

    #[test]
    fn prometheus_spans_deltas_reconstruct_original_counts() {
        // Losslessness: spans + delta-decoding must rebuild the dense OTel run
        // (Prometheus index = OTel index + 1).
        let h = sample_histogram();
        let prom = h.to_prometheus_native();
        assert_eq!(prom.schema, h.scale());
        assert_eq!(prom.zero_count, h.zero_count());

        let reconstructed = reconstruct(&prom.positive_spans, &prom.positive_deltas);
        let expected = dense_index_count(h.positive_offset(), h.positive_counts());
        assert_eq!(reconstructed, expected, "positive buckets lossless");

        let reconstructed_neg = reconstruct(&prom.negative_spans, &prom.negative_deltas);
        let expected_neg = dense_index_count(h.negative_offset(), h.negative_counts());
        assert_eq!(reconstructed_neg, expected_neg, "negative buckets lossless");

        // Total count is preserved: zero + all positive + all negative.
        let pos_sum: u64 = prom.positive_deltas_absolute().iter().sum();
        let neg_sum: u64 = prom.negative_deltas_absolute().iter().sum();
        assert_eq!(prom.zero_count + pos_sum + neg_sum, h.count());
    }

    // Rebuild (prom_index -> count) pairs from spans + deltas (inverse of the
    // encoder: subsequent spans start `prev_end + 1 + offset`).
    fn reconstruct(spans: &[PrometheusSpan], deltas: &[i64]) -> Vec<(i32, u64)> {
        let mut out = Vec::new();
        let mut count = 0i64;
        let mut d = 0usize;
        let mut prev_end: Option<i32> = None;
        for span in spans {
            let start = match prev_end {
                None => span.offset,
                Some(end) => end + 1 + span.offset,
            };
            for j in 0..span.length {
                count += deltas[d];
                d += 1;
                out.push((start + j as i32, count as u64));
            }
            prev_end = Some(start + span.length as i32 - 1);
        }
        out
    }

    // Expected (prom_index -> count) for the non-zero dense buckets.
    fn dense_index_count(offset: i32, counts: &[u64]) -> Vec<(i32, u64)> {
        counts
            .iter()
            .enumerate()
            .filter(|(_, c)| **c != 0)
            .map(|(i, &c)| (offset + i as i32 + 1, c))
            .collect()
    }

    impl PrometheusNativeHistogram {
        fn positive_deltas_absolute(&self) -> Vec<u64> {
            absolute(&self.positive_deltas)
        }
        fn negative_deltas_absolute(&self) -> Vec<u64> {
            absolute(&self.negative_deltas)
        }
    }

    fn absolute(deltas: &[i64]) -> Vec<u64> {
        let mut acc = 0i64;
        deltas
            .iter()
            .map(|&d| {
                acc += d;
                acc as u64
            })
            .collect()
    }
}
