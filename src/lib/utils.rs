use axum::extract::ws;
use nostr::message::RelayMessage;
use nostr::types::Timestamp;
use nostr::util::JsonUtil;
use std::time::SystemTime;

pub fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|x| x.as_secs())
        .unwrap_or(0)
}

pub fn timestamp_from_unix_time(unix_time: u64) -> Timestamp {
    Timestamp::from_secs(unix_time)
}

pub fn event_exceeds_max_length(content_length: usize, max_length: usize) -> bool {
    content_length > max_length
}

pub fn relay_message_to_ws_message(message: RelayMessage) -> ws::Message {
    let json_payload = message.as_json();
    let bytes_payload = ws::Utf8Bytes::from(json_payload);
    return ws::Message::Text(bytes_payload);
}
