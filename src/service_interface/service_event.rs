use uuid::Uuid;

#[derive(Debug, PartialEq)]
pub struct ServiceEvent<'a> {
    request_uuid: u128,
    timestamp: Option<u64>,
    timeout: u16,
    action: &'a str,
    payload: Option<&'a str>
}

impl<'a> ServiceEvent<'a> {
    pub fn new(timeout: u16, action: &'a str, payload: Option<&'a str>) -> Self {
        let request_uuid = Uuid::new_v4();
        let request_uuid = request_uuid.as_u128();
        let timestamp = None;

        ServiceEvent {
            request_uuid,
            timestamp,
            timeout,
            action,
            payload
        }
    }

    pub fn set_uuid(&mut self, request_uuid: u128) {
        self.request_uuid = request_uuid
    }

    pub fn get_uuid(&self) -> u128 {
        self.request_uuid
    }

    pub fn set_timestamp(&mut self, timestamp: u64) -> Result<(), &str> {
        if let Some(_) = self.timestamp {
            return Err("can set event timestamp only once")
        }

        self.timestamp = Some(timestamp);

        Ok(())
    }

    pub fn get_timestamp(&self) -> Option<u64> {
        self.timestamp
    }

    pub fn get_timeout(&self) -> u16 {
        self.timeout
    }

    pub fn get_action(&self) -> &'a str {
        self.action
    }

    pub fn get_payload(&self) -> Option<&'a str> {
        self.payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_ok() {
        let event = ServiceEvent::new(
            10,
            "test_event_create",
            None
        );

        assert_eq!(event.get_action(), "test_event_create");
        assert_eq!(event.get_payload(), None);
        assert_eq!(event.get_timeout(), 10);
        assert_eq!(event.get_timestamp(), None);
    }

    #[test]
    fn set_timestamp() {
        let mut event = ServiceEvent::new(
            10,
            "test_event_create",
            None
        );

        event.set_timestamp(42).unwrap();

        assert_eq!(event.get_timestamp().unwrap(), 42);
    }

    #[test]
    #[should_panic(expected="can set event timestamp only once")]
    fn set_timestamp_twice() {
        let mut event = ServiceEvent::new(
            10,
            "test_event_create",
            None
        );

        event.set_timestamp(42).unwrap();
        event.set_timestamp(43).unwrap();
    }

    #[test]
    fn change_uuid() {
        let mut event = ServiceEvent::new(
            10,
            "test_event_create",
            None
        );

        event.set_uuid(42);

        assert_eq!(event.get_uuid(), 42);
    }
}
