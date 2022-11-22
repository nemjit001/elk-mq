mod service_event;

use std::collections::HashMap;
use regex::Regex;
use lazy_static::lazy_static;
use redis::Commands;

pub use service_event::ServiceEvent;

#[derive(Debug, PartialEq)]
pub enum InterfaceError {
    ConnectionError(String),
    JSONDumpError(String),
    JSONParseError(String),
    EnqueueError(String),
    DequeueError(String),
    EmptyQueue,
    TimeoutExpired
}

pub type InterfaceResult<T> = Result<T, InterfaceError>;

#[derive(Debug, PartialEq)]
pub struct TimestampedEvent(u64, ServiceEvent);

impl TimestampedEvent {
    pub fn get_timestamp(&self) -> u64 {
        self.0
    }

    pub fn get_event(&self) -> &ServiceEvent {
        &self.1
    }
}

pub struct ServiceInterface {
    redis_client: redis::Client,
    queue_name: String,
    stream_name: String
}

impl ServiceInterface {
    pub fn new(queue_name: &str, connection_url: &str) -> Self {
        let redis_client = redis::Client::open(connection_url).unwrap();
        let stream_name = std::fmt::format(format_args!("stream{}", queue_name));

        ServiceInterface {
            redis_client,
            queue_name: String::from(queue_name),
            stream_name
        }
    }

    fn extract_timestamp_from_event_key(key: &str) -> u64 {
        lazy_static! {
            static ref KEY_REGEX: Regex = Regex::new(r"(?P<timestamp>\d+)-\d+").unwrap();
        }

        let timestamp = match KEY_REGEX.captures(key) {
            None => panic!("invalid event key passed to function"),
            Some(captures) => captures["timestamp"].to_string()
        };

        timestamp.parse::<u64>().unwrap()
    }

    fn setup_connection(&self) -> InterfaceResult<redis::Connection> {
        let connection = self.redis_client.get_connection();

        match connection {
            Err(error) => Err(InterfaceError::ConnectionError(error.to_string())),
            Ok(connection) => Ok(connection)
        }
    }

    fn get_service_event_by_key(&self, connection: &mut redis::Connection, event_key: &str) -> InterfaceResult<ServiceEvent> {
        let event_data_list: Vec<HashMap<String, HashMap<String, String>>> = match connection.xrange_count(
            &self.stream_name,
            &event_key,
            &event_key,
            1
        ) {
            Err(error) => return Err(InterfaceError::DequeueError(error.to_string())),
            Ok(data) => data
        };

        let event_data = match event_data_list.into_iter().next() {
            None => return Err(InterfaceError::DequeueError(String::from("unexpected empty value in stream"))),
            Some(event_data) => event_data
        };

        let event = match event_data.get(event_key) {
            None => return Err(InterfaceError::DequeueError(String::from("expected event map, found None"))),
            Some(event) => match event.get("event") {
                None => return Err(InterfaceError::DequeueError(String::from("expected event at key \"event\", found None"))),
                Some(event) => event
            }
        };

        let event: ServiceEvent = match serde_json::from_str(&event) {
            Err(error) => return Err(InterfaceError::JSONParseError(error.to_string())),
            Ok(event) => event
        };

        Ok(event)
    }

    pub fn enqueue(&mut self, event: &ServiceEvent) -> InterfaceResult<()> {
        let mut connection = self.setup_connection()?;

        let event_as_json = match serde_json::to_string(&event) {
            Err(error) => return Err(InterfaceError::JSONDumpError(error.to_string())),
            Ok(json) => json
        };

        let event_key: String = match connection.xadd(
            &self.stream_name,
            "*",
            &[("event", &event_as_json)]
        ) {
            Err(error) => return Err(InterfaceError::EnqueueError(error.to_string())),
            Ok(key) => key
        };

        if let Err(error) = connection.lpush::<_, _, ()>(
            &self.queue_name,
            &event_key
        ) {
            return Err(InterfaceError::EnqueueError(error.to_string()));
        }

        Ok(())
    }

    pub fn dequeue(&mut self) -> InterfaceResult<TimestampedEvent> {
        let mut connection = self.setup_connection()?;

        let event_key: String = match connection.rpop(&self.queue_name, None) {
            Err(error) => return Err(InterfaceError::DequeueError(error.to_string())),
            Ok(key) => match key {
                None => return Err(InterfaceError::EmptyQueue),
                Some(key) => key
            }
        };

        let event = self.get_service_event_by_key(&mut connection, &event_key)?;
        let timestamp = Self::extract_timestamp_from_event_key(&event_key);

        Ok(TimestampedEvent(timestamp, event))
    }

    pub fn dequeue_blocking(&mut self, timeout: u16) -> InterfaceResult<TimestampedEvent> {
        let mut connection = self.setup_connection()?;

        let event_kvp: (String, String) = match connection.brpop(
            &self.queue_name, 
            timeout.into()
        ) {
            Err(error) => return Err(InterfaceError::DequeueError(error.to_string())),
            Ok(key) => match key {
                None => return Err(InterfaceError::EmptyQueue),
                Some(kvp) => kvp
            }
        };

        let event_key = event_kvp.1.clone();

        let event = self.get_service_event_by_key(&mut connection, &event_key)?;
        let timestamp = Self::extract_timestamp_from_event_key(&event_key);

        Ok(TimestampedEvent(timestamp, event))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::thread;
    use super::*;

    #[test]
    fn create_ok() {
        let interface = ServiceInterface::new(
            "test_queue",
            "redis://127.0.0.1"
        );

        assert_eq!(interface.queue_name, "test_queue");
    }

    #[test]
    fn enqueue_dequeue_ok() {
        let mut interface = ServiceInterface::new(
            "test_event_enqueue_dequeue",
            "redis://127.0.0.1"
        );

        let event = ServiceEvent::new(
            10,
            "test_enqueue",
            None
        );

        interface.enqueue(&event).unwrap();

        let result = interface.dequeue().unwrap();

        assert_eq!(&event, result.get_event());
    }

    #[test]
    fn dequeue_blocking_ok() {
        let mut interface = ServiceInterface::new(
            "test_event_dequeue_blocking",
            "redis://127.0.0.1"
        );

        let event = ServiceEvent::new(
            10,
            "test_enqueue",
            Some(String::from("Payload!"))
        );

        let event_uuid = event.get_uuid();

        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(1000));

            let mut local_interface = ServiceInterface::new(
                "test_event_dequeue_blocking",
                "redis://127.0.0.1"
            );

            local_interface.enqueue(&event).unwrap();
        });

        let result = interface.dequeue_blocking(10).unwrap();

        handle.join().unwrap();

        assert_eq!(event_uuid, result.get_event().get_uuid());
        assert_eq!(result.get_event().get_payload(), Some(String::from("Payload!")));
    }
}
