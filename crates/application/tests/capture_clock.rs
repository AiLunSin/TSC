mod support;

use meeting_copilot_domain::port::CaptureClock;
use meeting_copilot_domain::vo::{ClockError, WallClock};
use support::ManualClock;

fn clock() -> ManualClock {
    ManualClock::new(WallClock::from_unix_millis(1_000))
}

#[test]
fn mono_starts_at_zero_and_advances() {
    let clock = clock();
    assert_eq!(clock.mono_now().expect("mono").as_millis(), 0);
    clock.advance_by(4_999);
    assert_eq!(clock.mono_now().expect("mono").as_millis(), 4_999);
}

#[test]
fn wall_tracks_base_plus_offset() {
    let clock = clock();
    assert_eq!(clock.wall_now().expect("wall").as_unix_millis(), 1_000);
    clock.advance_by(5_000);
    assert_eq!(clock.wall_now().expect("wall").as_unix_millis(), 6_000);
}

#[test]
fn mono_failure_is_injectable() {
    let clock = clock();
    clock.fail_mono();
    assert_eq!(clock.mono_now(), Err(ClockError::Overflow));
    clock.recover_mono();
    assert!(clock.mono_now().is_ok());
}

#[test]
fn support_contains_no_system_time_reads() {
    let source = include_str!("support/mod.rs");
    assert!(!source.contains("Instant::now"));
    assert!(!source.contains("SystemTime::now"));
}
