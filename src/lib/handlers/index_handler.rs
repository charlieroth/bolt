use crate::app::AppState;
use crate::handlers::nip11_handler::nip11_handler;
use askama::Template;
use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};
use std::fmt;
use std::fmt::Display;
use std::sync::Arc;

pub struct SupportedNip {
    pub nip: String,
    pub url: String,
}

impl Display for SupportedNip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NIP-{}", self.nip)
    }
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub relay_name: String,
    pub relay_description: String,
    pub relay_url: String,
    pub supported_nips: Vec<SupportedNip>,
}

pub struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template: {}", e),
            )
                .into_response(),
        }
    }
}

pub async fn index_handler(State(state): State<Arc<AppState>>, req: Request) -> Response {
    if req.headers().get(header::ACCEPT).unwrap() == "application/nostr+json" {
        return nip11_handler(State(state)).await;
    }

    let supported_nips: Vec<SupportedNip> = state
        .config
        .supported_nips
        .iter()
        .map(|nip| SupportedNip {
            nip: if nip.0 < 10 {
                format!("NIP-0{}", nip.0)
            } else {
                format!("NIP-{}", nip.0)
            },
            url: nip.1.clone(),
        })
        .collect();

    HtmlTemplate(IndexTemplate {
        relay_name: state.config.name.clone(),
        relay_description: state.config.description.clone(),
        relay_url: "https://relay.bolt/ws".to_string(),
        supported_nips,
    })
    .into_response()
}
