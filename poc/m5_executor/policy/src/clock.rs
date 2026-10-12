//! M-CLK-1: injectable, fail-closed time sources for grant TTL enforcement.
//!
//! Two kinds of time are kept apart on purpose:
//!
//! * **Wall time** (`SystemTime`) is only used to *mint* `not_after` inside the
//!   Biscuit grant (audit/UX: "expires in 90 s") and for the Datalog
//!   `time($t) <= exp` checks. Wall time can jump (NTP, RTC, user).
//! * **Monotonic time** (ticks since an arbitrary origin, never decreasing) is
//!   what the host-side guest path ([`GrantDeadline`]) uses to decide whether an
//!   admitted grant is still alive *before the Executor runs* and *before the LFS
//!   writer commits*. A wall-clock jump forward or backward cannot extend a
//!   grant past `ttl` of monotonic time.
//!
//! Fail-closed rules:
//! * a wall clock that goes backwards past its high-water mark is
//!   [`ClockError::Regressed`] -> callers deny with `EXPIRED` (C5 7);
//! * a missing monotonic source is [`ClockError::Unavailable`] -> deny;
//!   there is no "no clock, assume valid" path.
//!
//! This is the only module allowed to call `SystemTime::now()` /
//! `Instant::now()` (checked by `tests/clock_ttl.rs`).

use crate::DenyCode;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockError {
    /// Wall clock went backwards past the last value we handed out.
    Regressed,
    /// No trusted monotonic source in this domain.
    Unavailable,
}

impl ClockError {
    /// Every clock failure is fail-closed as `EXPIRED`: liveness of the grant
    /// cannot be proven.
    pub fn deny_code(self) -> DenyCode {
        DenyCode::Expired
    }
}

/// A source of wall and monotonic time.
pub trait Clock: Send + Sync {
    /// Wall time used only to mint/verify `not_after`.
    fn wall(&self) -> SystemTime;
    /// Monotonic time since an arbitrary fixed origin. `None` = no trusted source.
    fn monotonic(&self) -> Option<Duration>;
}

impl<C: Clock + ?Sized> Clock for std::sync::Arc<C> {
    fn wall(&self) -> SystemTime {
        (**self).wall()
    }
    fn monotonic(&self) -> Option<Duration> {
        (**self).monotonic()
    }
}

impl<C: Clock + ?Sized> Clock for Box<C> {
    fn wall(&self) -> SystemTime {
        (**self).wall()
    }
    fn monotonic(&self) -> Option<Duration> {
        (**self).monotonic()
    }
}

/// Production host clock: `SystemTime` for wall, `Instant` for monotonic.
#[derive(Debug)]
pub struct SystemClock {
    origin: Instant,
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
    /// Convenience for the legacy `*_at(.., now)` wrappers.
    pub fn wall_now() -> SystemTime {
        SystemTime::now()
    }
}

impl Clock for SystemClock {
    fn wall(&self) -> SystemTime {
        SystemTime::now()
    }
    fn monotonic(&self) -> Option<Duration> {
        Some(self.origin.elapsed())
    }
}

/// Deterministic test clock. Wall and monotonic can be moved independently
/// (to model NTP/RTC skew) and the monotonic source can be removed.
#[derive(Debug)]
pub struct FakeClock {
    wall_secs: AtomicU64,
    mono_nanos: AtomicU64,
    mono_available: AtomicBool,
}

impl FakeClock {
    pub fn at(wall: SystemTime) -> Self {
        Self {
            wall_secs: AtomicU64::new(
                wall.duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            ),
            mono_nanos: AtomicU64::new(0),
            mono_available: AtomicBool::new(true),
        }
    }
    /// Real passage of time: both wall and monotonic advance.
    pub fn advance(&self, d: Duration) {
        self.wall_secs.fetch_add(d.as_secs(), Ordering::SeqCst);
        self.mono_nanos
            .fetch_add(d.as_nanos() as u64, Ordering::SeqCst);
    }
    /// Wall-only jump (NTP step, RTC tamper); monotonic is unaffected.
    pub fn set_wall(&self, wall: SystemTime) {
        self.wall_secs.store(
            wall.duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            Ordering::SeqCst,
        );
    }
    pub fn set_monotonic_available(&self, available: bool) {
        self.mono_available.store(available, Ordering::SeqCst);
    }
}

impl Clock for FakeClock {
    fn wall(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.wall_secs.load(Ordering::SeqCst))
    }
    fn monotonic(&self) -> Option<Duration> {
        self.mono_available
            .load(Ordering::SeqCst)
            .then(|| Duration::from_nanos(self.mono_nanos.load(Ordering::SeqCst)))
    }
}

/// Wall clock guarded by a high-water mark: it never hands out a time earlier
/// than one it already handed out (no TTL resurrection by setting the clock back).
pub struct TrustedTime<C: Clock> {
    clock: C,
    high_water: Mutex<Option<SystemTime>>,
}

impl<C: Clock> TrustedTime<C> {
    pub fn new(clock: C) -> Self {
        Self {
            clock,
            high_water: Mutex::new(None),
        }
    }
    pub fn clock(&self) -> &C {
        &self.clock
    }
    /// Wall time for minting/verifying, or `Regressed` if it went backwards.
    pub fn wall(&self) -> Result<SystemTime, ClockError> {
        let now = self.clock.wall();
        let mut hw = self.high_water.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(prev) = *hw
            && now < prev
        {
            return Err(ClockError::Regressed);
        }
        *hw = Some(now);
        Ok(now)
    }
    pub fn monotonic(&self) -> Result<Duration, ClockError> {
        self.clock.monotonic().ok_or(ClockError::Unavailable)
    }
}

/// Monotonic deadline of one admitted grant, carried with the admission on the
/// host side of the guest path (PCB -> Executor -> LFS commit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrantDeadline {
    /// Monotonic instant after which the grant is dead.
    pub deadline: Duration,
    /// Wall `not_after` minted into the grant (display/audit only).
    pub not_after: SystemTime,
}

impl GrantDeadline {
    /// `wall_now` is the wall time the grant was minted at; the remaining
    /// lifetime `not_after - wall_now` is anchored on the monotonic clock *now*.
    pub fn start<C: Clock>(
        t: &TrustedTime<C>,
        wall_now: SystemTime,
        not_after: SystemTime,
    ) -> Result<Self, ClockError> {
        let ttl = not_after
            .duration_since(wall_now)
            .map_err(|_| ClockError::Regressed)?;
        let mono = t.monotonic()?;
        Ok(Self {
            deadline: mono + ttl,
            not_after,
        })
    }
    /// `Ok` while the grant is alive (inclusive of the deadline, like the
    /// Biscuit `$t <= exp` check); `EXPIRED` otherwise or if time is untrusted.
    pub fn check<C: Clock>(&self, t: &TrustedTime<C>) -> Result<Duration, DenyCode> {
        let mono = t.monotonic().map_err(ClockError::deny_code)?;
        if mono > self.deadline {
            return Err(DenyCode::Expired);
        }
        Ok(self.deadline - mono)
    }
}
