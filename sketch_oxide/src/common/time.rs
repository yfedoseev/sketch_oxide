//! Time, watermark, and late-data conventions for windowed and time-decayed sketches.
//!
//! Every windowed or decayed structure in the library shares one notion of time so the
//! semantics are identical across sketches and across the Python/Node/Java/.NET
//! bindings. This module defines that convention once.
//!
//! # The convention
//!
//! - **Time is a `u64` in caller-chosen units** ([`Timestamp`]). Milliseconds since an
//!   epoch, microseconds, logical ticks — the library never assumes a unit, it only
//!   requires that the same unit is used consistently for one structure. There is no
//!   hidden wall clock; nothing calls `SystemTime::now()` internally. This keeps results
//!   reproducible and identical in every binding.
//!
//! - **Event-time vs processing-time is the caller's choice** ([`TimeDomain`]). A sketch
//!   does not care which one you use; it only tracks the timestamps you give it. The
//!   distinction is documentation for *you*: with event-time you pass the time the event
//!   happened (records can arrive late/out of order); with processing-time you pass the
//!   time you observed it (always non-decreasing).
//!
//! - **A [`Watermark`] marks progress.** `advance(now)` moves a monotonic high-water
//!   mark forward; it never moves backward. `allowed_lateness` defines how far behind the
//!   watermark a record may still be accepted. A [`LateDataPolicy`] decides what happens
//!   to records older than `watermark - allowed_lateness`.
//!
//! - **`advance` is explicit.** Windowed sketches expose `advance(now)` (a.k.a. "tick")
//!   so window expiry and decay are driven by the caller, not by ingestion order. The
//!   same `advance(now: u64)` signature is mirrored in every binding.
//!
//! # FFI convention (mirrored in all four languages)
//!
//! - `advance(now: u64)` / `watermark() -> u64` are the canonical names.
//! - Timestamps cross the boundary as plain `u64`; no language-specific time objects.
//! - `LateDataPolicy` is exposed as a small enum/int; the default is
//!   [`LateDataPolicy::Drop`].

/// A point in time, in caller-defined units (e.g. milliseconds since an epoch).
///
/// The library is agnostic to the unit but requires it be consistent within a single
/// structure. Larger values are later in time.
pub type Timestamp = u64;

/// Which clock the caller's [`Timestamp`]s represent.
///
/// This is advisory metadata — it documents intent and lets late-data handling be
/// reasoned about; the arithmetic is identical either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum TimeDomain {
    /// Timestamps are when events actually occurred. Records may arrive out of order or
    /// late relative to the watermark.
    EventTime,
    /// Timestamps are when events were observed by this process. By construction they are
    /// non-decreasing, so late data does not occur.
    ProcessingTime,
}

impl TimeDomain {
    /// Whether late (out-of-order) data is possible in this domain.
    ///
    /// Always `false` for [`TimeDomain::ProcessingTime`].
    #[inline]
    pub fn allows_late_data(self) -> bool {
        matches!(self, TimeDomain::EventTime)
    }
}

/// What to do with a record whose timestamp is older than `watermark - allowed_lateness`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum LateDataPolicy {
    /// Discard the record. The safe default: a too-late record cannot be placed in a
    /// window that has already closed.
    #[default]
    Drop,
    /// Accept the record at its original (stale) timestamp anyway. Best-effort; the
    /// structure's error guarantees may not cover it.
    Accept,
    /// Treat the record as if it arrived exactly at the current watermark.
    ClampToWatermark,
}

/// How a record's timestamp was handled relative to the watermark.
///
/// Returned by [`Watermark::admit`] so callers know whether/at-what-time to record it.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// Record is on time (at or after `watermark - allowed_lateness`); use this timestamp.
    OnTime(Timestamp),
    /// Record is late but accepted; use this timestamp (possibly clamped to the watermark).
    Late(Timestamp),
    /// Record is too late under the policy and should be dropped.
    Dropped,
}

/// A monotonic watermark with a configurable lateness tolerance.
///
/// Embed this in any windowed or decayed sketch to get consistent progress tracking and
/// late-data handling. It carries no buckets or counts of its own — it only answers
/// "what time is it now, and may I admit a record stamped `t`?".
///
/// # Example
/// ```
/// use sketch_oxide::common::time::{Watermark, LateDataPolicy, Admission};
///
/// // Watermark with 100 units of allowed lateness, dropping anything older.
/// let mut wm = Watermark::new(100, LateDataPolicy::Drop);
/// wm.advance(1_000);
/// assert_eq!(wm.watermark(), 1_000);
///
/// // 950 is within [watermark - 100, watermark] -> on time.
/// assert_eq!(wm.admit(950), Admission::OnTime(950));
/// // 800 is older than 1000 - 100 = 900 -> dropped under the Drop policy.
/// assert_eq!(wm.admit(800), Admission::Dropped);
///
/// // advance never goes backwards.
/// wm.advance(500);
/// assert_eq!(wm.watermark(), 1_000);
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Watermark {
    /// Highest time seen via `advance`; never decreases.
    watermark: Timestamp,
    /// How far behind the watermark a record may still be admitted.
    allowed_lateness: u64,
    /// What to do with records that fall outside the lateness tolerance.
    policy: LateDataPolicy,
}

impl Watermark {
    /// Creates a watermark starting at time 0.
    ///
    /// * `allowed_lateness` — records stamped at or after `watermark - allowed_lateness`
    ///   are on time; older records are handled by `policy`.
    /// * `policy` — see [`LateDataPolicy`].
    pub fn new(allowed_lateness: u64, policy: LateDataPolicy) -> Self {
        Self {
            watermark: 0,
            allowed_lateness,
            policy,
        }
    }

    /// The current watermark (the latest time `advance` has reached).
    #[inline]
    pub fn watermark(&self) -> Timestamp {
        self.watermark
    }

    /// The configured lateness tolerance.
    #[inline]
    pub fn allowed_lateness(&self) -> u64 {
        self.allowed_lateness
    }

    /// The configured late-data policy.
    #[inline]
    pub fn policy(&self) -> LateDataPolicy {
        self.policy
    }

    /// The oldest timestamp still considered on time: `watermark - allowed_lateness`
    /// (saturating at 0).
    #[inline]
    pub fn horizon(&self) -> Timestamp {
        self.watermark.saturating_sub(self.allowed_lateness)
    }

    /// Advances the watermark to `now`. Monotonic: a `now` at or before the current
    /// watermark is ignored. Returns the watermark after the call.
    pub fn advance(&mut self, now: Timestamp) -> Timestamp {
        if now > self.watermark {
            self.watermark = now;
        }
        self.watermark
    }

    /// Decides how to admit a record stamped `timestamp`, advancing the watermark if the
    /// record is in the future.
    ///
    /// A record at or beyond the horizon is [`Admission::OnTime`]. A record behind the
    /// horizon is resolved by the policy: [`LateDataPolicy::Drop`] →
    /// [`Admission::Dropped`], [`LateDataPolicy::Accept`] → [`Admission::Late`] at its own
    /// timestamp, [`LateDataPolicy::ClampToWatermark`] → [`Admission::Late`] at the
    /// watermark.
    pub fn admit(&mut self, timestamp: Timestamp) -> Admission {
        // A record from the future advances our notion of "now".
        self.advance(timestamp);

        if timestamp >= self.horizon() {
            return Admission::OnTime(timestamp);
        }

        match self.policy {
            LateDataPolicy::Drop => Admission::Dropped,
            LateDataPolicy::Accept => Admission::Late(timestamp),
            LateDataPolicy::ClampToWatermark => Admission::Late(self.watermark),
        }
    }
}

/// A structure whose state advances with time.
///
/// Implemented by windowed and time-decayed sketches so they can be driven uniformly:
/// call [`advance`](Temporal::advance) to move "now" forward (expiring windows, applying
/// decay), and read [`watermark`](Temporal::watermark) to see how far time has progressed.
///
/// The same `advance(now: u64)` / `watermark() -> u64` pair is the canonical FFI surface
/// for every temporal sketch.
pub trait Temporal {
    /// Advances the structure's notion of "now" to `now`.
    ///
    /// Monotonic: implementations must ignore a `now` at or before the current watermark
    /// rather than moving time backward.
    fn advance(&mut self, now: Timestamp);

    /// The current watermark — the latest time the structure has advanced to.
    fn watermark(&self) -> Timestamp;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watermark_is_monotonic() {
        let mut wm = Watermark::new(0, LateDataPolicy::Drop);
        assert_eq!(wm.advance(100), 100);
        assert_eq!(wm.advance(50), 100, "advance must never go backwards");
        assert_eq!(wm.advance(100), 100, "equal advance is a no-op");
        assert_eq!(wm.advance(200), 200);
    }

    #[test]
    fn horizon_saturates_at_zero() {
        let mut wm = Watermark::new(500, LateDataPolicy::Drop);
        wm.advance(100);
        assert_eq!(wm.horizon(), 0, "watermark - lateness saturates at 0");
        wm.advance(900);
        assert_eq!(wm.horizon(), 400);
    }

    #[test]
    fn on_time_record_within_horizon() {
        let mut wm = Watermark::new(100, LateDataPolicy::Drop);
        wm.advance(1_000);
        assert_eq!(wm.admit(1_000), Admission::OnTime(1_000));
        assert_eq!(wm.admit(900), Admission::OnTime(900), "exactly at horizon");
    }

    #[test]
    fn drop_policy_drops_too_late() {
        let mut wm = Watermark::new(100, LateDataPolicy::Drop);
        wm.advance(1_000);
        assert_eq!(wm.admit(899), Admission::Dropped);
    }

    #[test]
    fn accept_policy_keeps_original_timestamp() {
        let mut wm = Watermark::new(100, LateDataPolicy::Accept);
        wm.advance(1_000);
        assert_eq!(wm.admit(500), Admission::Late(500));
    }

    #[test]
    fn clamp_policy_moves_to_watermark() {
        let mut wm = Watermark::new(100, LateDataPolicy::ClampToWatermark);
        wm.advance(1_000);
        assert_eq!(wm.admit(500), Admission::Late(1_000));
    }

    #[test]
    fn future_record_advances_watermark() {
        let mut wm = Watermark::new(0, LateDataPolicy::Drop);
        wm.advance(100);
        assert_eq!(wm.admit(250), Admission::OnTime(250));
        assert_eq!(wm.watermark(), 250, "a future record moves time forward");
    }

    #[test]
    fn default_policy_is_drop() {
        assert_eq!(LateDataPolicy::default(), LateDataPolicy::Drop);
    }

    #[test]
    fn time_domain_late_data() {
        assert!(TimeDomain::EventTime.allows_late_data());
        assert!(!TimeDomain::ProcessingTime.allows_late_data());
    }

    #[test]
    fn temporal_trait_is_object_safe_and_drivable() {
        struct Clock(Timestamp);
        impl Temporal for Clock {
            fn advance(&mut self, now: Timestamp) {
                if now > self.0 {
                    self.0 = now;
                }
            }
            fn watermark(&self) -> Timestamp {
                self.0
            }
        }

        let mut c = Clock(0);
        let driver: &mut dyn Temporal = &mut c;
        driver.advance(10);
        driver.advance(5); // ignored
        assert_eq!(driver.watermark(), 10);
    }
}
