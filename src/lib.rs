mod name_generator;
mod event_queue;

#[cfg(feature="python_bindings")]
mod python_bindings;

pub use event_queue::{ EventQueue, EventQueueError, EventQueueResult, ServiceEvent, Timestamp, TimestampedEvent };

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_public_api_ok() {
        let mut queue: EventQueue = EventQueue::new("lib_queue", "redis://127.0.0.1");

        let event: ServiceEvent = ServiceEvent::new(10, "lib_test", None);

        let timestamp: Timestamp = queue.enqueue(&event).unwrap();

        let timestamped_event: TimestampedEvent = queue.dequeue().unwrap();

        assert_eq!(timestamp, timestamped_event.get_timestamp());
        assert_eq!(&event, timestamped_event.get_event());
    }
}
