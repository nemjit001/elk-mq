//  Copyright 2022 Tijmen Menno Verhoef

//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at

//      http://www.apache.org/licenses/LICENSE-2.0

//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

use uuid::Uuid;
use serde::{Serialize, Deserialize};

/// A ServiceEvent contains information that is passed to other services by the communication backbone
/// 
/// - The [`request_uuid`] can be assumed to be unique between services, but collissions may happen, although this chance is very low
/// - The [`timeout`] is specified in seconds since queueing the request
/// - The [`action`] is an arbitrary string
/// - The [`payload`] is serialized data in an agreed upon format (commonly JSON)

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ServiceEvent {
    request_uuid: u128,
    timeout: u16,
    action: String,
    payload: Option<String>
}

impl ServiceEvent {
    /// Create a service event
    /// 
    /// A new event is created with a timeout, payload, and action.
    /// On creating an event, a new uuid is generated and assigned. When creating a response from an event, the old event's uuid is reused to identify the response
    /// - `timeout` must be non-zero, timeout is specified in seconds
    /// - `action` is an arbitrary string meaningful to consumers
    /// - `payload` is serialized data in a common format such as JSON. This format may differ between services, but should be decided upon when designing them.
    /// 
    /// Example:
    /// ```
    /// use elk_mq::ServiceEvent;
    /// 
    /// let event = ServiceEvent::new(10, "my_event", Some("{ \"foo\": \"bar\" }".to_string()));
    /// ```
    /// 
    pub fn new(timeout: u16, action: &str, payload: Option<String>) -> Self {
        let request_uuid = Uuid::new_v4();
        let request_uuid = request_uuid.as_u128();

        if timeout == 0 {
            panic!("timeout may not be zero")
        }

        ServiceEvent {
            request_uuid,
            timeout,
            action: String::from(action),
            payload
        }
    }

    /// Create a service event as response on another response
    /// 
    /// A response reuses the event uuid to identify it. Other than reusing a uuid, this functions acts the same as `ServiceEvent::new()`
    ///  
    pub fn new_response(event: &ServiceEvent, action: &str, payload: Option<String>) -> Self {
        let mut new_event = ServiceEvent::new(event.timeout, action, payload);

        // take over old uuid
        new_event.request_uuid = event.request_uuid;

        new_event
    }

    pub fn get_uuid(&self) -> u128 {
        self.request_uuid
    }

    pub fn get_timeout(&self) -> u16 {
        self.timeout
    }

    pub fn get_action(&self) -> &str {
        &self.action
    }

    pub fn get_payload(&self) -> Option<String> {
        self.payload.as_ref().map(| str | str.to_string())
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
    }
    
    #[test]
    fn create_response_ok() {
        let event_a = ServiceEvent::new(
            10,
            "test_event_create",
            None
        );

        let event_b = ServiceEvent::new_response(
            &event_a,
            "test_event_response",
            None
        );

        assert_eq!(event_a.get_uuid(), event_b.get_uuid());
    }
}
