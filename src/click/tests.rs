use std::time::Duration;

use super::{ClickTiming, Holds};

#[test]
fn makcu_holds_stay_between_35_and_75_ms() {
    let mut holds = Holds::seeded(7);
    for _ in 0..1000 {
        let hold = holds.pick(ClickTiming::Makcu).unwrap();
        assert!((Duration::from_millis(35)..=Duration::from_millis(75)).contains(&hold));
    }
}

#[test]
fn holds_actually_vary() {
    let mut holds = Holds::seeded(7);
    let first = holds.pick(ClickTiming::Ferrum);
    assert!((0..50).any(|_| holds.pick(ClickTiming::Ferrum) != first));
}

#[test]
fn fixed_and_off() {
    let mut holds = Holds::seeded(0);
    let hold = Duration::from_millis(40);
    assert_eq!(holds.pick(ClickTiming::Fixed(hold)), Some(hold));
    assert_eq!(holds.pick(ClickTiming::Off), None);
}
