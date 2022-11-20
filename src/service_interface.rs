pub mod service_event;

#[derive(Debug, PartialEq)]
pub enum InterfaceError {
    EmptyQueue,
    TimeoutExpired
}

pub type InterfaceResult<'a, T> = Result<T, InterfaceError>;

pub struct ServiceInterface<'a> {
    queue_name: &'a str
}

impl<'a> ServiceInterface<'a> {
    pub fn new(queue_name: &'a str) -> Self {
        ServiceInterface {
            queue_name
        }
    }

    pub fn enqueue(&mut self, event: &service_event::ServiceEvent) -> InterfaceResult<()> {
        Ok(())
    }

    pub fn dequeue(&mut self) -> InterfaceResult<'a, service_event::ServiceEvent<'a>> {
        Err(InterfaceError::EmptyQueue)
    }

    pub fn await_response(&mut self, event: &service_event::ServiceEvent) -> InterfaceResult<'a, service_event::ServiceEvent<'a>> {
        Err(InterfaceError::TimeoutExpired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_ok() {
        let interface = ServiceInterface::new(
            "test_queue"
        );

        assert_eq!(interface.queue_name, "test_queue");
    }

    #[test]
    fn enqueue_dequeue_ok() {
        let mut interface = ServiceInterface::new(
            "test_event_enqueue_dequeue"
        );

        let event = service_event::ServiceEvent::new(
            0,
            "test_enqueue",
            None
        );

        interface.enqueue(&event).unwrap();

        let result = interface.dequeue().unwrap();

        assert_eq!(event, result);
    }

    #[test]
    fn await_blocking_timeout() {
        let mut interface = ServiceInterface::new(
            "test_event_enqueue_dequeue"
        );

        let event = service_event::ServiceEvent::new(
            10,
            "test_enqueue",
            None
        );

        let result = interface.await_response(&event);

        assert_eq!(result, Err(InterfaceError::TimeoutExpired));
    }

    #[test]
    fn await_blocking_ok() {
        let mut interface = ServiceInterface::new(
            "test_event_enqueue_dequeue"
        );

        let event = service_event::ServiceEvent::new(
            10,
            "test_enqueue",
            Some("Ping!")
        );

        let result = interface.await_response(&event).unwrap();

        assert_eq!(event.get_uuid(), result.get_uuid());
        assert_eq!(result.get_payload(), Some("Pong!"));
    }
}
