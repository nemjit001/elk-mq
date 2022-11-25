
pub fn generate_event_stream_name(name: &str) -> String {
    format!("{}(event_stream)", name)
}

pub fn generate_response_stream_name(name: &str) -> String {
    format!("{}(response_stream)", name)
}

pub fn generate_message_queue_name(name: &str) -> String {
    format!("{}(message_queue)", name)
}
