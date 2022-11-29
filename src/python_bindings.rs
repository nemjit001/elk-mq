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

use std::cell::RefCell;
use uuid::Uuid;
use cpython::{ py_class, py_module_initializer, PyResult, PyNone, PyDict };

py_class!(class ServiceEvent | py | {
    data event: crate::ServiceEvent;

    def __new__(_cls, timeout: u16, action: &str, payload: Option<String>) -> PyResult<ServiceEvent> {
        ServiceEvent::create_instance(py, crate::ServiceEvent::new(timeout, action, payload))
    }

    def __repr__(&self) -> PyResult<String> {
        Ok(
            format!("{:#?}", self.event(py))
        )
    }

    def __dict__(&self) -> PyResult<PyDict> {
        let dict = PyDict::new(py);

        let uuid_str = Uuid::from_u128(
            self.event(py).uuid()
        ).to_string();

        dict.set_item(py, "uuid", uuid_str)?;
        dict.set_item(py, "timeout", self.event(py).timeout())?;
        dict.set_item(py, "action", self.event(py).action())?;
        dict.set_item(py, "payload", self.event(py).payload())?;

        Ok(dict)
    }

    def uuid(&self) -> PyResult<String> {
        Ok(
            Uuid::from_u128(
                self.event(py).uuid()
            ).to_string()
        )
    }

    def timeout(&self) -> PyResult<u16> {
        Ok(
            self.event(py).timeout()
        )
    }

    def action(&self) -> PyResult<String> {
        Ok(
            String::from(self.event(py).action())
        )
    }

    def payload(&self) -> PyResult<Option<String>> {
        Ok(
            self.event(py).payload()
        )
    }

    @classmethod
    def create_response(_cls, event: ServiceEvent, action: &str, payload: Option<String>) -> PyResult<ServiceEvent> {
        ServiceEvent::create_instance(py, crate::ServiceEvent::new_response(event.event(py), action, payload))
    }
});

py_class!(class EventQueue | py | {
    data event_queue: RefCell<crate::EventQueue>;

    def __new__(_cls, queue_name: &str, connection_url: &str) -> PyResult<EventQueue> {
        EventQueue::create_instance(
            py,
            RefCell::new(
                crate::EventQueue::new(queue_name, connection_url)
            )
        )
    }

    def enqueue(&self, event: ServiceEvent) -> PyResult<u64> {
        let mut queue = self.event_queue(py).borrow_mut();

        let timestamp = queue.enqueue(event.event(py)).unwrap();
        Ok(timestamp)
    }

    def dequeue(&self) -> PyResult<(u64, ServiceEvent)> {
        let mut queue = self.event_queue(py).borrow_mut();

        let timestamped_event = queue.dequeue().unwrap();

        let py_event = ServiceEvent::create_instance(py, timestamped_event.event().clone())?;

        Ok((timestamped_event.timestamp(), py_event))
    }

    def dequeue_blocking(&self, timeout: u16) -> PyResult<(u64, ServiceEvent)> {
        let mut queue = self.event_queue(py).borrow_mut();

        let timestamped_event = queue.dequeue_blocking(timeout).unwrap();
        let py_event = ServiceEvent::create_instance(py, timestamped_event.event().clone())?;

        Ok((timestamped_event.timestamp(), py_event)) 
    }

    def enqueue_response(&self, event: ServiceEvent) -> PyResult<PyNone> {
        let mut queue = self.event_queue(py).borrow_mut();

        queue.enqueue_response(event.event(py)).unwrap();

        Ok(PyNone)
    }

    def await_response(&self, event: ServiceEvent) -> PyResult<(u64, ServiceEvent)> {
        let mut queue = self.event_queue(py).borrow_mut();

        let timestamped_event = queue.await_response(event.event(py)).unwrap();

        let py_event = ServiceEvent::create_instance(py, timestamped_event.event().clone())?;

        Ok((timestamped_event.timestamp(), py_event))
    }
});

py_module_initializer!(
    elk_mq,
    | py, module | {
        module.add(py, "__doc__", "ElkMQ python module")?;

        module.add_class::<ServiceEvent>(py)?;
        module.add_class::<EventQueue>(py)?;

        Ok(())
    }
);
