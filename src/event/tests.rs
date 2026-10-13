use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{DropSource, Event, Queue, Stamp};

fn reset() -> Event {
    Event::SessionReset
}

fn motion() -> Event {
    Event::Motion {
        dx: 1,
        dy: 0,
        wheel: 0,
        pan: 0,
        at: super::Stamp::new(std::time::Instant::now(), Duration::ZERO),
    }
}

fn dropped(count: u64) -> Event {
    Event::Dropped {
        count,
        source: DropSource::Host,
    }
}

fn drain(queue: &Queue) -> Vec<Event> {
    let mut state = queue.lock();
    state.events.drain(..).collect()
}

#[test]
fn a_full_queue_puts_the_loss_count_at_the_head() {
    let queue = Queue::new(4);
    for _ in 0..4 {
        queue.push(motion());
    }
    queue.push(Event::Disconnected);
    let events = drain(&queue);
    assert_eq!(events.len(), 4);
    assert_eq!(events[0], dropped(2));
    assert_eq!(events[3], Event::Disconnected);
}

#[test]
fn a_busy_producer_keeps_adding_to_the_same_loss_count() {
    let queue = Queue::new(4);
    for _ in 0..104 {
        queue.push(motion());
    }
    let events = drain(&queue);
    assert_eq!(events.len(), 4);
    assert_eq!(events[0], dropped(101));
}

#[test]
fn a_closed_queue_ends_after_what_was_queued() {
    let queue = Arc::new(Queue::new(8));
    queue.push(reset());
    queue.close();
    let events = super::Events::new(queue, std::sync::Weak::<NoListeners>::new(), 0);
    assert_eq!(events.recv(), Ok(reset()));
    assert_eq!(events.recv(), Err(super::Closed));
    assert_eq!(
        events.recv_timeout(Duration::from_millis(1)),
        Err(super::RecvTimeoutError::Closed)
    );
}

#[test]
fn recv_timeout_tells_a_timeout_from_a_closed_stream() {
    let queue = Arc::new(Queue::new(8));
    let events = super::Events::new(queue, std::sync::Weak::<NoListeners>::new(), 0);
    assert_eq!(
        events.recv_timeout(Duration::from_millis(5)),
        Err(super::RecvTimeoutError::Timeout)
    );
}

struct NoListeners;

impl super::Listeners for NoListeners {
    fn receiver_dropped(&self, _: u64) {}
}

#[test]
fn a_loss_report_from_the_box_is_never_the_one_that_goes() {
    let queue = Queue::new(3);
    let from_box = Event::Dropped {
        count: 7,
        source: DropSource::Device,
    };
    queue.push(from_box.clone());
    queue.push(motion());
    queue.push(motion());
    queue.push(Event::Disconnected);
    let events = drain(&queue);
    assert!(events.contains(&from_box), "{events:?}");
    assert_eq!(events.last(), Some(&Event::Disconnected));
}

#[test]
fn link_events_survive_a_full_queue() {
    let queue = Queue::new(3);
    queue.push(Event::Disconnected);
    queue.push(motion());
    queue.push(motion());
    queue.push(motion());
    queue.push(motion());
    let events = drain(&queue);
    assert!(events.contains(&Event::Disconnected), "{events:?}");
}

#[test]
fn a_flood_of_refusals_is_one_counted_event_and_never_pushes_input_out() {
    use crate::capability::Action;
    let queue = Queue::new(4);
    let input = motion();
    queue.push(input.clone());
    for _ in 0..10_000 {
        queue.push(Event::Rejected {
            action: Action::Move,
            count: 1,
        });
    }
    queue.push(Event::Rejected {
        action: Action::Click,
        count: 1,
    });
    assert_eq!(
        drain(&queue),
        vec![
            input,
            Event::Rejected {
                action: Action::Move,
                count: 10_000
            },
            Event::Rejected {
                action: Action::Click,
                count: 1
            },
        ]
    );
}

fn key(pressed: bool) -> Event {
    Event::Key {
        key: crate::key::Key::A,
        pressed,
        at: super::Stamp::new(std::time::Instant::now(), Duration::ZERO),
    }
}

#[test]
fn a_full_queue_drops_motion_before_a_key_edge() {
    let queue = Queue::new(4);
    queue.push(key(true));
    queue.push(motion());
    queue.push(motion());
    queue.push(key(false));
    queue.push(motion());
    let events = drain(&queue);
    assert_eq!(events.len(), 4);
    assert_eq!(events[0], dropped(2));
    assert!(matches!(events[1], Event::Key { pressed: true, .. }));
    assert!(matches!(events[2], Event::Key { pressed: false, .. }));
}

#[test]
fn a_queue_of_keys_alone_still_stays_bounded() {
    let queue = Queue::new(4);
    for _ in 0..20 {
        queue.push(key(true));
    }
    let events = drain(&queue);
    assert_eq!(events.len(), 4);
    assert_eq!(events[0], dropped(17));
}

#[test]
fn new_motion_on_a_queue_full_of_edges_is_what_goes() {
    let queue = Queue::new(4);
    for pressed in [true, false, true, false] {
        queue.push(key(pressed));
    }
    queue.push(motion());
    queue.push(motion());
    let events = drain(&queue);
    assert_eq!(events.len(), 4);
    assert_eq!(events[0], dropped(3));
    assert!(
        events[1..]
            .iter()
            .all(|event| matches!(event, Event::Key { .. }))
    );
}

#[test]
fn a_stamp_built_with_a_huge_since_open_saturates_instead_of_panicking() {
    let arrived = Instant::now();
    let stamp =
        Stamp::new(arrived, Duration::MAX).with_received(arrived + Duration::from_millis(1), None);
    assert_eq!(stamp.received_since_open, Duration::MAX);
}
