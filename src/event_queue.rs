mod service_event;

use std::{ time, collections::HashMap };
use regex::Regex;
use lazy_static::lazy_static;
use redis::{Commands, Connection, Client};

pub use service_event::ServiceEvent;
use uuid::Uuid;

#[derive(Debug, PartialEq)]
pub enum EventQueueError {
    ConnectionError(String),
    JSONDumpError(String),
    JSONParseError(String),
    EnqueueError(String),
    DequeueError(String),
    EmptyQueue,
    TimeoutExpired
}

pub type EventQueueResult<T> = Result<T, EventQueueError>;

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

pub struct EventQueue {
    redis_client: Client,
    queue_name: String,
    stream_name: String,
    response_stream_name: String
}

impl EventQueue {
    pub fn new(queue_name: &str, connection_url: &str) -> Self {
        let redis_client = redis::Client::open(connection_url).unwrap();
        let redis_queue_name = std::format!("{}(queue)", queue_name);
        let redis_event_stream_name = std::format!("{}(event_stream)", queue_name);
        let redis_response_stream_name = std::format!("{}(response_stream)", queue_name);

        EventQueue {
            redis_client,
            queue_name: redis_queue_name,
            stream_name: redis_event_stream_name,
            response_stream_name: redis_response_stream_name
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

    fn setup_connection(&self) -> EventQueueResult<redis::Connection> {
        let connection = self.redis_client.get_connection();

        match connection {
            Err(error) => Err(EventQueueError::ConnectionError(error.to_string())),
            Ok(connection) => Ok(connection)
        }
    }

    fn get_service_event_by_key(&self, connection: &mut Connection, event_key: &str, event_type: &str) -> EventQueueResult<ServiceEvent> {
        let event_data_list: Vec<HashMap<String, HashMap<String, String>>> = match connection.xrange_count(
            &self.stream_name,
            &event_key,
            &event_key,
            1
        ) {
            Err(error) => return Err(EventQueueError::DequeueError(error.to_string())),
            Ok(data) => data
        };

        let event_data = match event_data_list.into_iter().next() {
            None => return Err(EventQueueError::DequeueError(String::from("unexpected empty value in stream"))),
            Some(event_data) => event_data
        };

        let event = match event_data.get(event_key) {
            None => return Err(EventQueueError::DequeueError(String::from("expected event map, found None"))),
            Some(event) => match event.get(event_type) {
                None => return Err(EventQueueError::DequeueError(String::from("expected event at key \"event\", found None"))),
                Some(event) => event
            }
        };

        let event: ServiceEvent = match serde_json::from_str(&event) {
            Err(error) => return Err(EventQueueError::JSONParseError(error.to_string())),
            Ok(event) => event
        };

        Ok(event)
    }

    fn get_last_response_id(&self, connection: &mut Connection) -> EventQueueResult<String> {
        let last_response: Vec<HashMap<String, HashMap<String, String>>> = match connection.xrevrange_count(&self.response_stream_name, "+", "-", 1) {
            Err(error) => return Err(EventQueueError::DequeueError(error.to_string())),
            Ok(response) => response
        };

        if last_response.is_empty() {
            return Ok(String::from("0-0"));
        }

        if last_response.len() != 1 {
            return Err(EventQueueError::DequeueError(String::from("unexpected response length")));
        }

        let last_response = &last_response[0];
        let id = last_response.keys().next().unwrap().to_string();

        Ok(id)
    }

    pub fn enqueue(&mut self, event: &ServiceEvent) -> EventQueueResult<()> {
        let mut connection = self.setup_connection()?;

        let event_as_json = match serde_json::to_string(&event) {
            Err(error) => return Err(EventQueueError::JSONDumpError(error.to_string())),
            Ok(json) => json
        };

        let event_key: String = match connection.xadd(
            &self.stream_name,
            "*",
            &[("event", &event_as_json)]
        ) {
            Err(error) => return Err(EventQueueError::EnqueueError(error.to_string())),
            Ok(key) => key
        };

        if let Err(error) = connection.lpush::<_, _, ()>(
            &self.queue_name,
            &event_key
        ) {
            return Err(EventQueueError::EnqueueError(error.to_string()));
        }

        Ok(())
    }

    pub fn dequeue(&mut self) -> EventQueueResult<TimestampedEvent> {
        let mut connection = self.setup_connection()?;

        let event_key: String = match connection.rpop(&self.queue_name, None) {
            Err(error) => return Err(EventQueueError::DequeueError(error.to_string())),
            Ok(key) => match key {
                None => return Err(EventQueueError::EmptyQueue),
                Some(key) => key
            }
        };

        let event = self.get_service_event_by_key(&mut connection, &event_key, "event")?;
        let timestamp = Self::extract_timestamp_from_event_key(&event_key);

        Ok(TimestampedEvent(timestamp, event))
    }

    pub fn dequeue_blocking(&mut self, timeout: u16) -> EventQueueResult<TimestampedEvent> {
        let mut connection = self.setup_connection()?;

        let event_kvp: (String, String) = match connection.brpop(
            &self.queue_name, 
            timeout.into()
        ) {
            Err(error) => return Err(EventQueueError::DequeueError(error.to_string())),
            Ok(key) => match key {
                None => return Err(EventQueueError::EmptyQueue),
                Some(kvp) => kvp
            }
        };

        let event_key = event_kvp.1.clone();

        let event = self.get_service_event_by_key(&mut connection, &event_key, "event")?;
        let timestamp = Self::extract_timestamp_from_event_key(&event_key);

        Ok(TimestampedEvent(timestamp, event))
    }

    pub fn enqueue_response(&mut self, event: &ServiceEvent) -> EventQueueResult<()> {
        let mut connection = self.setup_connection()?;

        let event_as_json = match serde_json::to_string(&event) {
            Err(error) => return Err(EventQueueError::JSONDumpError(error.to_string())),
            Ok(json) => json
        };

        let uuid_string = Uuid::from_u128(event.get_uuid()).to_string();
        let response_key: String = match connection.xadd(
            &self.stream_name,
            "*",
            &[("response", &event_as_json)]
        ) {
            Err(error) => return Err(EventQueueError::EnqueueError(error.to_string())),
            Ok(key) => key
        };

        if let Err(error) = connection.xadd::<_, _, _, _, ()>(&self.response_stream_name, "*", &[(&uuid_string, &response_key)]) {
            return Err(EventQueueError::EnqueueError(error.to_string()));
        }

        Ok(())
    }

    pub fn await_response(&mut self, event: &ServiceEvent) -> EventQueueResult<TimestampedEvent> {
        let mut connection = self.setup_connection()?;

        let start_time = time::Instant::now();
        let timeout = event.get_timeout();
        let target_uuid_string = Uuid::from_u128(event.get_uuid()).to_string();

        let mut current_time = start_time;
        let mut response_key: Option<String> = None;
        let mut last_response_id: String = self.get_last_response_id(&mut connection)?;

        self.enqueue(event)?;

        while start_time + time::Duration::new(timeout.into(), 0) >= current_time {
            // read new response entries from last seen ID onward
            let new_responses: Vec<HashMap<String, Vec<HashMap<String, HashMap<String, String>>>>> = match connection.xread(
                &[&self.response_stream_name],
                &[&last_response_id]
            ) {
                Err(error) => return Err(EventQueueError::DequeueError(error.to_string())),
                Ok(response_vec) => response_vec
            };

            // if no new responses are found, we continue with polling
            if new_responses.len() == 0 {
                current_time = time::Instant::now();
                continue;
            }

            // only 1 stream is read, convert [ hashmap ] -> hashmap
            let response_map = &new_responses[0];

            // extract the stream name and verify it actually matches read stream
            let new_responses = match response_map.get(&self.response_stream_name) {
                None => return Err(EventQueueError::DequeueError(String::from("invalid stream name in response map"))),
                Some(response_vec) => response_vec
            };

            for response in new_responses {
                // extract response id for this entry, we know only 1 exists because of structure (id, (key, data))
                let response_id = match response.keys().next() {
                    None => return Err(EventQueueError::DequeueError(String::from("no response ID in response map"))),
                    Some(id) => id.clone()
                };

                // extract metadata
                let response_metadata = match response.get(&response_id) {
                    None => return Err(EventQueueError::DequeueError(std::format!("no metadata stored for response ID {}", response_id))),
                    Some(data) => data
                };

                // extract uuid string from metadata
                let found_uuid_string = match response_metadata.keys().next() {
                    None => return Err(EventQueueError::DequeueError(std::format!("UUID string not found in metadata {:#?}", response_metadata))),
                    Some(uuid) => uuid.clone()
                };

                // check if we are looking for this string
                if found_uuid_string != target_uuid_string {
                    last_response_id = response_id;
                    continue;
                }

                // fetch the key we are looking for
                response_key = match response_metadata.get(&target_uuid_string) {
                    None => return Err(EventQueueError::DequeueError(std::format!("failed to get response key from metadata {:#?}", response_metadata))),
                    Some(key) => Some(key.clone())
                };
                
                // after extracting the key we are done with the loop, so early break
                // UUID is guaranteed unique with low collisions, so looking further will provide no benefit
                break;
            }

            // break polling if a response key is found
            if let Some(_) = response_key {
                break;
            }

            // update our current time to detect when timeout is done
            current_time = time::Instant::now();
        }

        // check if we found a response key
        let response_key = match response_key {
            None => return Err(EventQueueError::TimeoutExpired),
            Some(response) => response
        };

        // create a timestamped event from found data
        let response = self.get_service_event_by_key(&mut connection, &response_key, "response")?;
        let timestamp = Self::extract_timestamp_from_event_key(&response_key);

        Ok(TimestampedEvent(timestamp, response))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::thread;
    use super::*;

    #[test]
    fn create_ok() {
        let _interface = EventQueue::new(
            "test_queue",
            "redis://127.0.0.1"
        );
    }

    #[test]
    fn enqueue_dequeue_ok() {
        let mut interface = EventQueue::new(
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
        let mut interface = EventQueue::new(
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
            thread::sleep(Duration::from_secs(2));

            let mut local_interface = EventQueue::new(
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

    #[test]
    #[should_panic(expected="called `Result::unwrap()` on an `Err` value: EmptyQueue")]
    fn dequeue_blocking_timeout() {
        let mut interface = EventQueue::new(
            "test_event_dequeue_blocking_timeout",
            "redis://127.0.0.1"
        );

        interface.dequeue_blocking(1).unwrap();
    }

    #[test]
    fn await_ok() {
        let mut interface = EventQueue::new(
            "test_event_await",
            "redis://127.0.0.1"
        );

        let event = ServiceEvent::new(
            10,
            "await_test",
            Some(String::from("ping"))
        );

        let join_handle = thread::spawn(|| {
            let mut thread_interface = EventQueue::new(
                "test_event_await",
                "redis://127.0.0.1"
            );

            let event = thread_interface.dequeue_blocking(10).unwrap();
            let event = event.get_event();

            println!("{:#?}", event);

            assert_eq!(event.get_payload(), Some(String::from("ping")));

            let response = ServiceEvent::new_response(&event, "await_response", Some(String::from("pong")));
            thread_interface.enqueue_response(&response).unwrap();
        });

        let response = interface.await_response(&event).unwrap();
        let response = response.get_event();

        join_handle.join().unwrap();
        
        assert_eq!(response.get_action(), "await_response");
        assert_eq!(response.get_payload(), Some(String::from("pong")));
        assert_eq!(response.get_uuid(), event.get_uuid());
    }

    #[test]
    fn simultaneous_await_ok() {
        let mut interface = EventQueue::new(
            "test_event_await_sim",
            "redis://127.0.0.1"
        );

        let answer_thread = thread::spawn(|| {
            let mut thread_interface = EventQueue::new(
                "test_event_await_sim",
                "redis://127.0.0.1"
            );

            for _ in 0..2 {
                let event = thread_interface.dequeue_blocking(10).unwrap();
                let event = event.get_event();
                
                assert_eq!(event.get_payload(), Some(String::from("ping")));

                let response = ServiceEvent::new_response(&event, "await_response", Some(String::from("pong")));
                thread_interface.enqueue_response(&response).unwrap();
            }
        });

        let event_thread = thread::spawn(|| {
            let mut thread_interface = EventQueue::new(
                "test_event_await_sim",
                "redis://127.0.0.1"
            );

            let event = ServiceEvent::new(
                1,
                "await_test",
                Some(String::from("ping"))
            );

            let response = thread_interface.await_response(&event).unwrap();
            let response = response.get_event();

            assert_eq!(response.get_action(), "await_response");
            assert_eq!(response.get_payload(), Some(String::from("pong")));
            assert_eq!(response.get_uuid(), event.get_uuid());
        });

        let event = ServiceEvent::new(
            1,
            "await_test",
            Some(String::from("ping"))
        );

        let response = interface.await_response(&event).unwrap();
        let response = response.get_event();

        assert_eq!(response.get_action(), "await_response");
        assert_eq!(response.get_payload(), Some(String::from("pong")));
        assert_eq!(response.get_uuid(), event.get_uuid());

        answer_thread.join().unwrap();
        event_thread.join().unwrap();
    }

    #[test]
    #[should_panic(expected="called `Result::unwrap()` on an `Err` value: TimeoutExpired")]
    fn await_timeout() {
        let mut interface = EventQueue::new(
            "test_event_await_timeout",
            "redis://127.0.0.1"
        );

        let event = ServiceEvent::new(
            1,
            "await_test",
            Some(String::from("ping"))
        );

        interface.await_response(&event).unwrap();
    }
}
