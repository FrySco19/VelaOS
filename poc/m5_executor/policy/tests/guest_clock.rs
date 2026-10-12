//! M-CLK-3 host tests of the in-guest deadline logic (`app/src/guest_clock.rs`,
//! the same file the QEMU PCB compiles). The hardware read is covered by
//! `verify-bridge-e2e` markers.

use vela_executor_policy::guest_clock::{Deadline, MAX_TTL_SECS, Ticks};

const HZ: u64 = 62_500_000; // QEMU virt generic timer

fn t(count: u64) -> Ticks {
    Ticks { count, freq: HZ }
}

#[test]
fn guest_clock_alive_until_deadline_inclusive_then_expired() {
    let d = Deadline::start(t(1_000), 120).unwrap();
    assert_eq!(d.remaining_ms(t(1_000)), Some(120_000));
    assert!(d.alive(t(1_000 + 120 * HZ)));
    assert!(!d.alive(t(1_000 + 120 * HZ + 1)));
    assert_eq!(d.elapsed_ms(t(1_000 + HZ)), 1_000);
}

#[test]
fn guest_clock_fail_closed_inputs() {
    assert_eq!(Deadline::start(Ticks { count: 5, freq: 0 }, 10), None, "no frequency");
    assert_eq!(Deadline::start(t(5), 0), None);
    assert_eq!(Deadline::start(t(5), MAX_TTL_SECS + 1), None, "ttl > 120 s");
    assert_eq!(Deadline::start(t(u64::MAX - 10), 1), None, "overflow");
    let d = Deadline::start(t(1_000), 1).unwrap();
    assert!(!d.alive(t(999)), "counter below anchor is untrusted");
    assert!(!d.alive(Ticks { count: 1_001, freq: HZ / 2 }), "frequency changed");
}

#[test]
fn guest_clock_short_ttl_expires_after_one_second() {
    let d = Deadline::start(t(0), 1).unwrap();
    assert!(d.alive(t(HZ)));
    assert!(!d.alive(t(HZ + 1)));
}
