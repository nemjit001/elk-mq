# ElkMQ (Event Log driven Kompakt Message Queue)

ElkMQ is a pure rust implementation of the Message Queue pattern over Redis.

## What can I do with ElkMQ?

ElkMQ allows asynchronous message passing over a Redis instance. If a secure Redis instance is already deployed,
ElkMQ can connect to it and use it to communicate.

Each message is guaranteed to be delivered only once. This allows for safe N:M client communication. Awaiting
responses is also allowed by ElkMQ. This makes ElkMQ usable as a communication backbone for independent services.

Using ElkMQ in existing rust programs is as easy as including it in your dependency list. Using ElkMQ as a python module is as easy as installing it with Pip.

## Minumum requirements

- Redis 6.0
- Rust 1.65
- Python 3+ (for the python module)

## Examples

### Rust examples

The examples below show the usage of ElkMQ for rust clients. They show how events can be enqueued, dequeued, and
awaited. Some examples also show their output.

Enqueueing and dequeueing an event in a newly created queue is straightforward, see the following example:

```rust
use elk_mq::{ EventQueue, ServiceEvent, Timestamp, TimestampedEvent };

fn main() {
    // Create a queue with a name
    let mut queue: EventQueue = EventQueue::new("foo", "redis://127.0.0.1");

    // Create an event
    let event: ServiceEvent = ServiceEvent::new(10, "test_event", Some("serialized_data".to_string()));

    // Enqueueing returns the event timestamp registered by the Redis instance
    let _timestamp: Timestamp = queue.enqueue(&event).unwrap();

    // Dequeueing returns the timestamp of the event, and the event itself as a timestamped event type
    let timestamped_event: TimestampedEvent = queue.dequeue().unwrap();

    println!("{:#?}", timestamped_event)
}
```

The output this example generates is listed below:

```out
TimestampedEvent(
    1669887505996,
    ServiceEvent {
        request_uuid: 215842608724208526221701166594411877883,
        timeout: 10,
        action: "test_event",
        payload: Some(
            "serialized_data",
        ),
    },
)
```

Awaiting a response for an event is similar to enqueueing the event, save for the fact that the await can time
out. If this happens an EventQueueError is returned, and the program continues. Any service dequeueing events should
take care to verify if the timeout of an event is already expired before handling it. If a response is enqueued,
but the original event timeout has already expired, then the response will never be handled. This is not an issue
in regular use, but it will take up space in your Redis instance.

```rust
    // -- snip -- //

    // an event with a 20 second timeout
    let event: ServiceEvent = ServiceEvent::new(
        20, // timeout in seconds
        "test_awaited_event", // event name
        Some("serialized_data".to_string())
    );

    // after 20 seconds this either panics with a TimeoutExpired error
    // or returns the received response as a TimestampedEvent
    let response: TimestampedEvent = queue.await_response(&event).unwrap();

    // -- snip -- //
```

### Notes on the python module

The python module functions exactly the same as the Rust library. All python types mirror their rust counterparts,
except for TimestampedEvent. In python this is a `(int, ServiceEvent)` tuple to allow for easy destructuring of data.

## Authors

- Tijmen Verhoef, <tijmenmenno@gmail.com>
