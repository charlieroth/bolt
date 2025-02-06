use crate::app::AppState;
use axum::extract::{ws, State, WebSocketUpgrade};
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use nostr::event::Event;
use nostr::filter::Filter;
use nostr::message::{ClientMessage, RelayMessage, SubscriptionId};
use nostr::util::JsonUtil;
use nostr_database::{NostrEventsDatabase, RejectedReason, SaveEventStatus};
use std::sync::Arc;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.write_buffer_size(2 << 17) // 128KiB
        .max_frame_size(state.config.limits.max_content_length as usize)
        .max_message_size(state.config.limits.max_message_length as usize)
        .accept_unmasked_frames(false)
        .on_upgrade(move |socket| handle_socket(socket, state))
}

pub async fn handle_socket(stream: ws::WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = stream.split();

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            ws::Message::Text(text) => {
                let message = serde_json::from_str::<ClientMessage>(&text).unwrap();
                match message {
                    ClientMessage::Event(event) => {
                        let event_message = handle_event_client_message(event, state.clone()).await;
                        sender.send(event_message).await.unwrap();
                    }
                    ClientMessage::Req {
                        subscription_id,
                        filter,
                    } => {
                        let messages =
                            handle_req_client_message(filter, subscription_id, state.clone()).await;
                        for message in messages {
                            sender.send(message).await.unwrap();
                        }
                    }
                    ClientMessage::Close(subscription_id) => {
                        let message = handle_close_client_message(subscription_id).await;
                        sender.send(message).await.unwrap();
                    }
                    _ => {}
                }
            }
            ws::Message::Ping(_) | ws::Message::Pong(_) => {}
            ws::Message::Binary(_) => {
                let notice = RelayMessage::notice("binary messages are not supported");
                let json_payload = notice.as_json();
                let bytes_payload = ws::Utf8Bytes::from(json_payload);
                sender.send(ws::Message::Text(bytes_payload)).await.unwrap();
            }
            ws::Message::Close(_) => {}
        }
    }
}

async fn handle_event_client_message(event: Box<Event>, state: Arc<AppState>) -> ws::Message {
    let save_event_status = state.db.save_event(&event).await.unwrap();
    match save_event_status {
        SaveEventStatus::Success => {
            let event_message = RelayMessage::ok(event.id, true, event.content);
            let json_payload = event_message.as_json();
            let bytes_payload = ws::Utf8Bytes::from(json_payload);
            ws::Message::Text(bytes_payload)
        }
        SaveEventStatus::Rejected(reason) => {
            let reason_str = match reason {
                RejectedReason::Ephemeral => "ephemeral event",
                RejectedReason::Duplicate => "duplicate event",
                RejectedReason::Deleted => "deleted event",
                RejectedReason::Expired => "expired event",
                RejectedReason::Replaced => "replaced event",
                RejectedReason::InvalidDelete => "invalid delete",
                RejectedReason::Other => "other",
            };
            let event_message = RelayMessage::notice(reason_str);
            let json_payload = serde_json::to_string(&event_message).unwrap();
            let bytes_payload = ws::Utf8Bytes::from(json_payload);
            ws::Message::Text(bytes_payload)
        }
    }
}

async fn handle_req_client_message(
    filter: Box<Filter>,
    subscription_id: SubscriptionId,
    state: Arc<AppState>,
) -> Vec<ws::Message> {
    let mut messages: Vec<ws::Message> = Vec::new();
    // Send events requested by clients
    let events = state.db.query(*filter).await.unwrap();
    for event in events.into_iter() {
        let event_message = RelayMessage::event(subscription_id.clone(), event);
        let json_payload = event_message.as_json();
        let bytes_payload = ws::Utf8Bytes::from(json_payload);
        messages.push(ws::Message::Text(bytes_payload));
    }
    // Indicate the end of stored events
    let eose_message = RelayMessage::eose(subscription_id.clone());
    let json_payload = eose_message.as_json();
    let bytes_payload = ws::Utf8Bytes::from(json_payload);
    messages.push(ws::Message::Text(bytes_payload));

    // Indicate that the subscription is closed
    let close_message = RelayMessage::closed(subscription_id, "");
    let json_payload = close_message.as_json();
    let bytes_payload = ws::Utf8Bytes::from(json_payload);
    messages.push(ws::Message::Text(bytes_payload));

    messages
}

async fn handle_close_client_message(subscription_id: SubscriptionId) -> ws::Message {
    let close_message = RelayMessage::closed(subscription_id, "");
    let json_payload = close_message.as_json();
    let bytes_payload = ws::Utf8Bytes::from(json_payload);
    ws::Message::Text(bytes_payload)
}
