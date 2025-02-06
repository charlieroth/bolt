use crate::app::AppState;
use crate::config;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct RelayInformationDocument {
    pub name: String,
    pub description: String,
    pub banner: String,
    pub icon: String,
    pub pubkey: String,
    pub contact: String,
    pub supported_nips: Vec<u16>,
    pub software: String,
    pub version: String,
    pub limitations: config::Limitations,
}

pub struct SupportedNip {
    pub nip: String,
    pub url: String,
}

pub async fn nip11_handler(State(state): State<Arc<AppState>>) -> Response {
    let supported_nips: Vec<u16> = state
        .config
        .supported_nips
        .iter()
        .map(|nip| nip.0)
        .collect();

    let relay_info = RelayInformationDocument {
        name: state.config.name.clone(),
        description: state.config.description.clone(),
        banner: state.config.banner.clone(),
        icon: state.config.icon.clone(),
        pubkey: state.config.pubkey.clone(),
        contact: state.config.contact.clone(),
        supported_nips,
        software: state.config.software.clone(),
        version: state.config.version.clone(),
        limitations: state.config.limits.clone(),
    };
    let json_payload = serde_json::to_string(&relay_info).unwrap();
    (StatusCode::OK, json_payload).into_response()
}
