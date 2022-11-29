use elk_mq::*;

#[test]
fn tls_enabled() {
    let _ = EventQueue::new(
        "tls",
        "rediss://127.0.0.1"
    );
}

#[test]
fn tls_simple_queue() {
    let mut q = EventQueue::new(
        "tls_simple_queue",
        "rediss://127.0.0.1"
    );

    let event = ServiceEvent::new(
        10,
        "tls_test",
        None
    );

    let timestamp = q.enqueue(&event).unwrap();
    let result = q.dequeue_blocking(10).unwrap();

    assert_eq!(timestamp, result.timestamp());
    assert_eq!(result.event(), &event);
}
