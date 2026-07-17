//! Panic-free binary read/write helpers for sketch (de)serialization.
//!
//! Every hand-rolled `deserialize` in this crate parses attacker-controllable
//! bytes. Slicing `bytes[pos..pos + 8]` and `try_into().unwrap()` panics on
//! truncated input — a remote-DoS when sketch bytes cross a trust boundary
//! (e.g. through the PyO3/napi bindings). [`ReadCursor`] replaces those with
//! bounds-checked reads that return [`SketchError::DeserializationError`]
//! instead of panicking.
//!
//! [`WriteBuf`] is the symmetric writer, and [`Framing`] provides a tiny shared
//! `[magic:2][sketch-id:1][format-version:1]` header so serialized artifacts are
//! self-describing and versioned (see docs/research/fable5 finding F2 / B2).

use crate::common::{Result, SketchError};

/// Two-byte magic that prefixes every framed sketch: ASCII "SO" (Sketch Oxide).
pub const MAGIC: [u8; 2] = *b"SO";

/// A bounds-checked, non-panicking reader over a byte slice.
///
/// All `read_*` methods advance an internal cursor and return
/// `Err(SketchError::DeserializationError)` when the slice is too short,
/// rather than panicking.
#[derive(Debug, Clone)]
pub struct ReadCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> ReadCursor<'a> {
    /// Create a cursor over `bytes`, positioned at the start.
    #[must_use]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    /// Number of unread bytes remaining.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    /// Current read offset from the start of the slice.
    #[must_use]
    pub fn position(&self) -> usize {
        self.pos
    }

    #[inline]
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| SketchError::DeserializationError("read length overflow".to_string()))?;
        if end > self.bytes.len() {
            return Err(SketchError::DeserializationError(format!(
                "unexpected end of input: need {} bytes at offset {}, have {}",
                n,
                self.pos,
                self.bytes.len()
            )));
        }
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    /// Read `N` raw bytes as a fixed array.
    pub fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let slice = self.take(N)?;
        let mut arr = [0u8; N];
        arr.copy_from_slice(slice);
        Ok(arr)
    }

    /// Read `n` raw bytes as a slice (borrowing the underlying buffer).
    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        self.take(n)
    }

    /// Read a single byte.
    pub fn read_u8(&mut self) -> Result<u8> {
        Ok(self.read_array::<1>()?[0])
    }

    /// Read a little-endian `u16`.
    pub fn read_u16_le(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.read_array()?))
    }

    /// Read a little-endian `u32`.
    pub fn read_u32_le(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read_array()?))
    }

    /// Read a little-endian `i32`.
    pub fn read_i32_le(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.read_array()?))
    }

    /// Read a little-endian `u64`.
    pub fn read_u64_le(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.read_array()?))
    }

    /// Read a little-endian `i64`.
    pub fn read_i64_le(&mut self) -> Result<i64> {
        Ok(i64::from_le_bytes(self.read_array()?))
    }

    /// Read a little-endian `i128`.
    pub fn read_i128_le(&mut self) -> Result<i128> {
        Ok(i128::from_le_bytes(self.read_array()?))
    }

    /// Read a little-endian `f64`.
    pub fn read_f64_le(&mut self) -> Result<f64> {
        Ok(f64::from_le_bytes(self.read_array()?))
    }

    /// Read a little-endian `u64` length prefix, validating it against the
    /// number of bytes actually remaining so a crafted huge length can never
    /// drive an over-allocation. `per_element` is the byte cost of one element.
    pub fn read_len_prefixed_count(&mut self, per_element: usize) -> Result<usize> {
        let count = self.read_u64_le()? as usize;
        self.check_count_fits(count, per_element)?;
        Ok(count)
    }

    /// Read a little-endian `u32` length prefix, validating it against the
    /// remaining bytes (see [`Self::read_len_prefixed_count`]).
    pub fn read_u32_len_prefixed_count(&mut self, per_element: usize) -> Result<usize> {
        let count = self.read_u32_le()? as usize;
        self.check_count_fits(count, per_element)?;
        Ok(count)
    }

    #[inline]
    fn check_count_fits(&self, count: usize, per_element: usize) -> Result<()> {
        if per_element > 0 {
            let needed = count.checked_mul(per_element).ok_or_else(|| {
                SketchError::DeserializationError("element count overflow".to_string())
            })?;
            if needed > self.remaining() {
                return Err(SketchError::DeserializationError(format!(
                    "declared {} elements ({} bytes) but only {} bytes remain",
                    count,
                    needed,
                    self.remaining()
                )));
            }
        }
        Ok(())
    }
}

/// A little-endian byte writer used by `serialize` implementations.
#[derive(Debug, Default, Clone)]
pub struct WriteBuf {
    bytes: Vec<u8>,
}

impl WriteBuf {
    /// Create an empty writer.
    #[must_use]
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Create an empty writer with pre-reserved capacity.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(cap),
        }
    }

    /// Append a single byte.
    pub fn write_u8(&mut self, v: u8) {
        self.bytes.push(v);
    }

    /// Append a little-endian `u16`.
    pub fn write_u16_le(&mut self, v: u16) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    /// Append a little-endian `u32`.
    pub fn write_u32_le(&mut self, v: u32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    /// Append a little-endian `i32`.
    pub fn write_i32_le(&mut self, v: i32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    /// Append a little-endian `u64`.
    pub fn write_u64_le(&mut self, v: u64) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    /// Append a little-endian `i64`.
    pub fn write_i64_le(&mut self, v: i64) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    /// Append a little-endian `i128`.
    pub fn write_i128_le(&mut self, v: i128) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    /// Append a little-endian `f64`.
    pub fn write_f64_le(&mut self, v: f64) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    /// Append raw bytes.
    pub fn write_bytes(&mut self, b: &[u8]) {
        self.bytes.extend_from_slice(b);
    }

    /// Consume the writer, returning the accumulated bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// The shared `[magic:2][sketch-id:1][format-version:1]` framing header.
///
/// Wrapping a payload in a `Framing` makes serialized artifacts
/// self-identifying (which sketch produced them) and versioned (so future
/// format changes are detectable rather than silently misinterpreted).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Framing {
    /// Stable identifier for the sketch kind (see [`SketchId`]).
    pub sketch_id: u8,
    /// Format version for this sketch's payload.
    pub version: u8,
}

impl Framing {
    /// Number of header bytes a framed payload carries.
    pub const HEADER_LEN: usize = 4;

    /// Create a framing header for `sketch_id` at `version`.
    #[must_use]
    pub fn new(sketch_id: u8, version: u8) -> Self {
        Self { sketch_id, version }
    }

    /// Prepend the framing header to a payload, returning the full byte vector.
    #[must_use]
    pub fn frame(&self, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::HEADER_LEN + payload.len());
        out.extend_from_slice(&MAGIC);
        out.push(self.sketch_id);
        out.push(self.version);
        out.extend_from_slice(payload);
        out
    }

    /// Parse and validate a framing header, returning the header and a cursor
    /// positioned at the start of the payload.
    ///
    /// # Errors
    /// Returns `DeserializationError` if the magic bytes are wrong or the input
    /// is shorter than the header, and `Unsupported` if the `sketch_id` does
    /// not match `expected_id`.
    pub fn parse(bytes: &[u8], expected_id: u8) -> Result<(Framing, ReadCursor<'_>)> {
        let mut cur = ReadCursor::new(bytes);
        let magic = cur.read_array::<2>()?;
        if magic != MAGIC {
            return Err(SketchError::DeserializationError(
                "bad magic bytes (not a framed sketch)".to_string(),
            ));
        }
        let sketch_id = cur.read_u8()?;
        let version = cur.read_u8()?;
        if sketch_id != expected_id {
            return Err(SketchError::Unsupported {
                op: "deserialize: sketch id in framing header does not match target type",
            });
        }
        Ok((Framing { sketch_id, version }, cur))
    }
}

/// Stable per-sketch identifiers used in [`Framing`] headers.
///
/// Values are part of the wire format — only ever append, never reuse or renumber.
#[non_exhaustive]
pub struct SketchId;

#[allow(missing_docs)]
impl SketchId {
    pub const HYPERLOGLOG: u8 = 1;
    pub const COUNT_MIN: u8 = 2;
    pub const BLOOM: u8 = 3;
    pub const DDSKETCH: u8 = 4;
    pub const KLL: u8 = 5;
    pub const THETA: u8 = 6;
    pub const SPACE_SAVING: u8 = 7;
    pub const BINARY_FUSE: u8 = 8;
    pub const SPLINE: u8 = 9;
    pub const CPC: u8 = 10;
    pub const HYPER_ANF: u8 = 11;
    pub const TRIEST: u8 = 12;
    pub const DOULION: u8 = 13;
    pub const MASCOT: u8 = 14;
    pub const THINKD: u8 = 15;
    pub const FLEET: u8 = 16;
    pub const GSS: u8 = 17;
    pub const AGM_CONNECTIVITY: u8 = 18;
    pub const RABITQ: u8 = 19;
    pub const RABITQ_CODE: u8 = 20;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_cursor_reads_values() {
        let mut buf = WriteBuf::new();
        buf.write_u8(7);
        buf.write_u32_le(0xDEAD_BEEF);
        buf.write_u64_le(0x0102_0304_0506_0708);
        buf.write_f64_le(3.5);
        let bytes = buf.into_bytes();

        let mut cur = ReadCursor::new(&bytes);
        assert_eq!(cur.read_u8().unwrap(), 7);
        assert_eq!(cur.read_u32_le().unwrap(), 0xDEAD_BEEF);
        assert_eq!(cur.read_u64_le().unwrap(), 0x0102_0304_0506_0708);
        assert_eq!(cur.read_f64_le().unwrap(), 3.5);
        assert_eq!(cur.remaining(), 0);
    }

    #[test]
    fn read_cursor_errors_on_truncation_instead_of_panicking() {
        let bytes = [1u8, 2, 3];
        let mut cur = ReadCursor::new(&bytes);
        assert!(cur.read_u64_le().is_err());
        // cursor did not advance past the error
        assert_eq!(cur.position(), 0);
    }

    #[test]
    fn len_prefix_rejects_oversized_count() {
        // A prefix claiming u64::MAX elements over an empty tail must not
        // over-allocate; it should error.
        let mut buf = WriteBuf::new();
        buf.write_u64_le(u64::MAX);
        let bytes = buf.into_bytes();
        let mut cur = ReadCursor::new(&bytes);
        assert!(cur.read_len_prefixed_count(12).is_err());
    }

    #[test]
    fn framing_round_trips_and_rejects_bad_magic() {
        let framing = Framing::new(SketchId::DDSKETCH, 1);
        let framed = framing.frame(&[9, 9, 9]);
        let (hdr, mut cur) = Framing::parse(&framed, SketchId::DDSKETCH).unwrap();
        assert_eq!(hdr.version, 1);
        assert_eq!(cur.read_bytes(3).unwrap(), &[9, 9, 9]);

        // wrong magic
        assert!(Framing::parse(&[0, 0, 1, 1, 9], SketchId::DDSKETCH).is_err());
        // wrong sketch id
        assert!(Framing::parse(&framed, SketchId::BLOOM).is_err());
    }
}
