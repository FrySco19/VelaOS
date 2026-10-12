//! M-CLK-3: in-guest trusted monotonic clock for grant TTL, `no_std`.
//!
//! Source: the Arm generic timer physical counter `CNTPCT_EL0` and its
//! frequency `CNTFRQ_EL0`, readable at EL0 because the Microkit seL4 kernel is
//! built with `CONFIG_EXPORT_PCNT_USER`. The counter is hardware time since
//! reset: it is not the host wall clock, the host cannot set it after boot, and
//! it never goes backwards.
//!
//! Policy (ADR-CLK-1 §3.2):
//! * the admission carries a *relative* TTL (`1..=120` s, minted by the host
//!   PDP at UserSelection time); the guest PCB anchors it on the counter when it
//!   publishes the admitted frame -> [`Deadline`];
//! * before committing the Executor's result to the LFS writer the PCB checks
//!   the deadline; dead -> `EXPIRED` (C5 7), no commit;
//! * fail-closed: frequency 0, TTL outside `1..=120`, overflow, or a counter
//!   reading earlier than the anchor all count as expired.
//!
//! Host and guest checks compose: the host refuses to mint/sign after
//! `not_after` (wall), the guest refuses to persist after `ttl` of its own
//! monotonic time. A boot-time epoch change invalidates older grants (OWN-v3).
//! Shared verbatim with the host test crate via `#[path]`.
#![allow(dead_code)]

/// Upper bound of a grant TTL (same as host `MAX_READ_GRANT_TTL`).
pub const MAX_TTL_SECS: u64 = 120;

/// A counter reading: ticks and the counter frequency (Hz).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticks {
    pub count: u64,
    pub freq: u64,
}

/// Read the guest's monotonic counter.
#[cfg(target_arch = "aarch64")]
pub fn now() -> Ticks {
    let count: u64;
    let freq: u64;
    // SAFETY: plain system-register reads, EL0-accessible (EXPORT_PCNT_USER).
    // `isb` keeps the read from being speculated ahead of earlier work.
    unsafe {
        core::arch::asm!("isb", "mrs {c}, cntpct_el0", c = out(reg) count, options(nomem, nostack));
        core::arch::asm!("mrs {f}, cntfrq_el0", f = out(reg) freq, options(nomem, nostack));
    }
    Ticks { count, freq }
}

/// Monotonic deadline of one admitted grant inside the guest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Deadline {
    start: u64,
    deadline: u64,
    freq: u64,
}

impl Deadline {
    /// Anchor `ttl_secs` at `at`. `None` (fail-closed) if there is no usable
    /// frequency, the TTL is outside `1..=120` s, or the deadline overflows.
    pub fn start(at: Ticks, ttl_secs: u64) -> Option<Self> {
        if at.freq == 0 || ttl_secs == 0 || ttl_secs > MAX_TTL_SECS {
            return None;
        }
        let span = at.freq.checked_mul(ttl_secs)?;
        Some(Self {
            start: at.count,
            deadline: at.count.checked_add(span)?,
            freq: at.freq,
        })
    }

    /// Remaining milliseconds if alive (inclusive of the deadline, like the
    /// Biscuit `$t <= exp`), `None` if expired or if time is untrustworthy
    /// (counter below the anchor, or a different frequency).
    pub fn remaining_ms(&self, at: Ticks) -> Option<u64> {
        if at.freq != self.freq || at.count < self.start || at.count > self.deadline {
            return None;
        }
        Some((self.deadline - at.count) / (self.freq / 1000).max(1))
    }

    pub fn alive(&self, at: Ticks) -> bool {
        self.remaining_ms(at).is_some()
    }

    /// Elapsed milliseconds since the anchor (for audit lines).
    pub fn elapsed_ms(&self, at: Ticks) -> u64 {
        at.count.saturating_sub(self.start) / (self.freq / 1000).max(1)
    }
}
