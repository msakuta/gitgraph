use super::map_err;
use crate::ServerState;
use actix_web::{get, http, web, HttpResponse, Responder};
use anyhow::Result;
use git2::{Oid, Repository, Signature};
use serde::Serialize;
use serde_json::json;

#[get("/commits/{commit}/message")]
pub(crate) async fn get_message(
    data: web::Data<ServerState>,
    web::Path(commit): web::Path<String>,
) -> actix_web::Result<impl Responder> {
    let message = (|| -> Result<_> {
        let repo = Repository::open(&data.settings.repo)?;
        let commit = repo.find_commit(Oid::from_str(&commit)?)?;
        commit
            .message()
            .map(|s| s.to_owned())
            .ok_or(anyhow::anyhow!("Missing message"))
    })()
    .map_err(map_err)?;
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        // Keep cache for 1 week since git hash guarantees uniqueness
        .header(http::header::CACHE_CONTROL, "max-age=604800")
        .body(message))
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
struct MetaResponse {
    author: EditStamp,
    committer: EditStamp,
    message: String,
}

#[get("/commits/{commit}/meta")]
pub(crate) async fn get_meta(
    data: web::Data<ServerState>,
    web::Path(commit): web::Path<String>,
) -> actix_web::Result<impl Responder> {
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
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        // Keep cache for 1 week since git hash guarantees uniqueness
        .header(http::header::CACHE_CONTROL, "max-age=604800")
        .body(json!(signature)))
}
