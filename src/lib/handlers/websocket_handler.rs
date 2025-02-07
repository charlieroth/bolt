use crate::app::AppState;
use crate::utils;
use axum::extract::{ws, State, WebSocketUpgrade};
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use nostr::event::{Event, EventId, Kind};
use nostr::filter::{Filter, SingleLetterTag};
use nostr::message::{ClientMessage, RelayMessage, SubscriptionId};
use nostr::nips::nip19::FromBech32;
use nostr_database::{NostrEventsDatabase, RejectedReason, SaveEventStatus};
use std::sync::Arc;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.write_buffer_size(state.config.limits.write_buffer_size as usize)
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
                        // Check if the event content exceeds the max length
                        if utils::event_exceeds_max_length(
                            event.content.len(),
                            state.config.limits.max_content_length as usize,
                        ) {
                            let notice = RelayMessage::notice("event content exceeds max length");
                            let message = utils::relay_message_to_ws_message(notice);
                            return sender.send(message).await.unwrap();
                        }

                        let now = utils::unix_time();
                        let event_expired_in_future = utils::timestamp_from_unix_time(
                            now + state.config.reject_future_seconds,
                        );

                        // Check if the event is expired
                        if event.is_expired() {
                            let notice = RelayMessage::notice("event has expired");
                            let message = utils::relay_message_to_ws_message(notice);
                            return sender.send(message).await.unwrap();
                        }

                        // Check if the event is expired in the future
                        if event.is_expired_at(&event_expired_in_future) {
                            let notice = RelayMessage::notice(
                                "event created_at field is out of acceptable range",
                            );
                            let message = utils::relay_message_to_ws_message(notice);
                            return sender.send(message).await.unwrap();
                        }

                        // Verify the event
                        if let Err(_e) = event.verify() {
                            let notice = RelayMessage::notice("event verification failed");
                            let message = utils::relay_message_to_ws_message(notice);
                            return sender.send(message).await.unwrap();
                        }

                        // Handle NIP-09: Event Deletion Request
                        if event.kind == nostr::event::Kind::EventDeletion {
                            let event_messages =
                                handle_event_deletion_client_message(event, state.clone()).await;
                            for message in event_messages {
                                sender.send(message).await.unwrap();
                            }
                            return;
                        }

                        // Save the event
                        let event_message = handle_event_client_message(event, state.clone()).await;
                        return sender.send(event_message).await.unwrap();
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
                    _ => {
                        let message = handle_unsupported_message("unsupported message").await;
                        sender.send(message).await.unwrap();
                    }
                }
            }
            ws::Message::Ping(_) | ws::Message::Pong(_) => {}
            ws::Message::Binary(_) => {
                let message = handle_unsupported_message("binary messages are not supported").await;
                sender.send(message).await.unwrap();
            }
            ws::Message::Close(_) => {}
        }
    }
}

/// Handle event deletion client message
///
/// This function should follow the logic described in [NIP-09](https://github.com/nostr-protocol/nips/blob/master/09.md)
async fn handle_event_deletion_client_message(
    event: Box<Event>,
    state: Arc<AppState>,
) -> Vec<ws::Message> {
    let mut messages: Vec<ws::Message> = Vec::new();
    // Store deletion event
    let save_event_status = state.db.save_event(&event).await.unwrap();
    match save_event_status {
        SaveEventStatus::Success => {
            let event_message = RelayMessage::ok(event.id, true, "deletion event stored");
            messages.push(utils::relay_message_to_ws_message(event_message));
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
            // In the case that the deletion event cannot be stored
            // we send a notice to the client
            return vec![utils::relay_message_to_ws_message(event_message)];
        }
    }

    let author = event.pubkey.clone();
    let mut events: Vec<EventId> = Vec::new();
    let mut kinds: Vec<Kind> = Vec::new();
    // Process `e` and `a` tags for deletion
    for tag in event.tags {
        match tag.kind() {
            nostr::event::TagKind::SingleLetter(single_letter_tag) => match single_letter_tag {
                SingleLetterTag {
                    character,
                    uppercase,
                } if character == nostr::Alphabet::E && !uppercase => {
                    // ["e", "dcd59..464a2"]
                    let event_id_raw = tag.as_slice().get(1).unwrap();
                    let event_id = EventId::parse(event_id_raw).unwrap();
                    events.push(event_id);
                }
                SingleLetterTag {
                    character,
                    uppercase,
                } if character == nostr::Alphabet::K && !uppercase => {
                    // ["k", 1]
                    let kind_raw = tag.as_slice().get(1).unwrap();
                    let kind_u16 = kind_raw.parse::<u16>().unwrap();
                    let kind = Kind::from_u16(kind_u16);
                    kinds.push(kind);
                }
                SingleLetterTag {
                    character,
                    uppercase,
                } if character == nostr::Alphabet::A && !uppercase => {
                    todo!("handle `a` tag, `['a', '<kind>:<pubkey>:<d-identifier>']`")
                }
                _ => {}
            },
            _ => {}
        }
    }

    let filter = Filter::new().author(author).events(events);
    let _ = state.db.delete(filter).await.unwrap();
    return messages;
}

async fn handle_event_client_message(event: Box<Event>, state: Arc<AppState>) -> ws::Message {
    let save_event_status = state.db.save_event(&event).await.unwrap();
    match save_event_status {
        SaveEventStatus::Success => {
            let event_message = RelayMessage::ok(event.id, true, event.content);
            return utils::relay_message_to_ws_message(event_message);
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
            return utils::relay_message_to_ws_message(event_message);
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
        messages.push(utils::relay_message_to_ws_message(event_message));
    }
    // Indicate the end of stored events
    let eose_message = RelayMessage::eose(subscription_id.clone());
    messages.push(utils::relay_message_to_ws_message(eose_message));

    // Indicate that the subscription is closed
    let close_message = RelayMessage::closed(subscription_id, "");
    messages.push(utils::relay_message_to_ws_message(close_message));

    return messages;
}

async fn handle_close_client_message(subscription_id: SubscriptionId) -> ws::Message {
    let close_message = RelayMessage::closed(subscription_id, "");
    return utils::relay_message_to_ws_message(close_message);
}

async fn handle_unsupported_message(message: &str) -> ws::Message {
    let notice = RelayMessage::notice(message);
    return utils::relay_message_to_ws_message(notice);
}
