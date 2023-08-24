use std::sync::Arc;

use super::map_err;
use crate::{AnyhowError, ServerState};
// use actix_web::{get, http, web, HttpResponse, Responder};
use anyhow::Result;
use axum::{
    extract::{Path, State},
    Json,
};
use git2::{Oid, Repository, Signature};
use serde::Serialize;
use serde_json::json;

pub(crate) async fn get_message(
    State(data): State<Arc<ServerState>>,
    Path(commit): Path<String>,
) -> Result<Json<String>, AnyhowError> {
    let message = (|| -> Result<_> {
        let repo = Repository::open(&data.settings.repo)?;
        let commit = repo.find_commit(Oid::from_str(&commit)?)?;
        commit
            .message()
            .map(|s| s.to_owned())
            .ok_or(anyhow::anyhow!("Missing message"))
    })()
    .map_err(map_err)?;
    Ok(
        // Keep cache for 1 week since git hash guarantees uniqueness
        // .header(http::header::CACHE_CONTROL, "max-age=604800")
        Json(message),
    )
}

#[derive(Serialize)]
struct EditStamp {
    name: Option<String>,
    email: Option<String>,
    date: i64,
}

impl From<Signature<'_>> for EditStamp {
    fn from(s: Signature) -> Self {
        Self {
            name: s.name().map(|s| s.to_owned()),
            email: s.email().map(|s| s.to_owned()),
            date: s.when().seconds(),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct MetaResponse {
    author: EditStamp,
    committer: EditStamp,
    message: String,
}

pub(crate) async fn get_meta(
    State(data): State<Arc<ServerState>>,
    Path(commit): Path<String>,
) -> Result<Json<MetaResponse>, AnyhowError> {
    let signature = (|| -> Result<_> {
        let repo = Repository::open(&data.settings.repo)?;
        let commit = repo.find_commit(Oid::from_str(&commit)?)?;
        Ok(MetaResponse {
            author: commit.author().into(),
            committer: commit.committer().into(),
            message: commit.message().unwrap_or("").to_owned(),
        })
    })()
    .map_err(map_err)?;
    Ok(
        // Keep cache for 1 week since git hash guarantees uniqueness
        // .header(http::header::CACHE_CONTROL, "max-age=604800")
        Json(signature),
    )
}
