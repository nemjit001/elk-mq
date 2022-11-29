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

mod name_generator;
mod event_queue;

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
