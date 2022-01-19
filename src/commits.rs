//! API implementations for commits request
mod meta;
mod process_commits;

use actix_web::{get, post, web, HttpResponse, Responder};
use anyhow::Result;
use git2::{Oid, Repository};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{convert::From, time::Instant};

use super::{AnyhowError, ServerState, SessionId, Settings};

pub use meta::{get_message, get_meta};
use process_commits::{process_commits, ProcessCommitsResult};

#[derive(Serialize)]
struct Stats {
    insertions: usize,
    deletions: usize,
}

#[derive(Serialize)]
struct CommitData {
    hash: String, // String is not the most efficient representation of the hash, but it's easy to serialize into a JSON
    message: String,
    parents: Vec<String>,
}

#[derive(Serialize)]
struct CommitResponse {
    commits: Vec<CommitData>,
    session: Option<SessionId>,
}

fn map_err(err: impl ToString) -> actix_web::Error {
    actix_web::error::ErrorInternalServerError(err.to_string())
}

fn new_session(
    data: &ServerState,
    result: ProcessCommitsResult,
) -> actix_web::Result<CommitResponse> {
    let session = if !result.continue_.is_empty() {
        let session = SessionId(random());

        let mut sessions = data.sessions.lock().map_err(map_err)?;
        sessions.insert(
            session,
            crate::Session {
                checked_commits: result.checked,
                continue_commits: result.continue_,
                sent_pages: 0,
            },
        );
        Some(session)
    } else {
        None
    };

    Ok(CommitResponse {
        commits: result.commits,
        session,
    })
}

/// Default commit query (head or all, depending on settings)
#[get("/commit-query")]
pub(crate) async fn get_commits(data: web::Data<ServerState>) -> actix_web::Result<impl Responder> {
    let time_load = Instant::now();

    let result = (|| -> Result<ProcessCommitsResult> {
        let repo = Repository::open(&data.settings.repo)?;

        let reference = if let Some(ref branch) = data.settings.branch {
            repo.resolve_reference_from_short_name(branch)?
        } else {
            repo.head()?
        };

        let head = if data.settings.all {
            repo.references()?
                .map(|refs| refs.and_then(|refb| refb.peel_to_commit()))
                .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            vec![reference.peel_to_commit()?]
        };

        process_commits(&repo, &data.settings, &head, None)
    })()
    .map_err::<AnyhowError, _>(|err| err.into())?;

    println!(
        "git history with {} commits analyzed in {} ms",
        result.commits.len(),
        time_load.elapsed().as_micros() as f64 / 1000.
    );

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(&json!(new_session(&data, result)?).to_string()))
}

/// Single ref query
#[get("/commit-query/{id:.*}")]
async fn get_commits_hash(
    data: web::Data<ServerState>,
    web::Path(id): web::Path<String>,
) -> actix_web::Result<impl Responder> {
    let time_load = Instant::now();

    let result = (|| -> Result<_> {
        let repo = Repository::open(&data.settings.repo)?;
        let commit = if let Ok(reference) = repo.find_reference(&id) {
            reference.peel_to_commit()?
        } else {
            repo.find_commit(Oid::from_str(&id)?)?
        };
        let commits = [commit];
        process_commits(&repo, &data.settings, &commits, None)
    })()
    .map_err::<AnyhowError, _>(|err| err.into())?;

    println!(
        "git history with {} commits from {}analyzed in {} ms",
        result.commits.len(),
        id,
        time_load.elapsed().as_micros() as f64 / 1000.
    );

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(&json!(new_session(&data, result)?).to_string()))
}

/// Multiple commits in request body
#[post("/commits")]
async fn get_commits_multi(
    data: web::Data<ServerState>,
    request: web::Json<Vec<String>>,
) -> actix_web::Result<impl Responder> {
    let time_load = Instant::now();

    let result = (|| -> Result<_> {
        let repo = Repository::open(&data.settings.repo)?;

        let commits = request
            .iter()
            .map(|name| repo.find_commit(Oid::from_str(name)?))
            .collect::<std::result::Result<Vec<_>, git2::Error>>()?;

        process_commits(&repo, &data.settings, &commits, None)
    })()
    .map_err::<AnyhowError, _>(|err| err.into())?;

    println!(
        "git history from {} commits results {} analyzed in {} ms",
        request.len(),
        result.commits.len(),
        time_load.elapsed().as_micros() as f64 / 1000.
    );

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(&json!(new_session(&data, result)?).to_string()))
}

#[derive(Deserialize)]
struct SessionRequest {
    session_id: String,
}

#[actix_web::post("/sessions")]
async fn get_commits_session(
    data: web::Data<ServerState>,
    request: web::Json<SessionRequest>,
) -> actix_web::Result<impl Responder> {
    let time_load = Instant::now();

    let repo = Repository::open(&data.settings.repo).map_err(map_err)?;

    let session_id = SessionId::from(&request.session_id as &str);
    let mut sessions = data.sessions.lock().map_err(map_err)?;

    let session = if let Some(session) = sessions.get_mut(&session_id) {
        session
    } else {
        println!("Failed session? {:?}", session_id.to_string());
        return Ok(HttpResponse::BadRequest().body("Session not found"));
    };

    println!(
        "session: checked_commits: {}, continue_commits: {}",
        session.checked_commits.len(),
        session.continue_commits.len()
    );

    if session.continue_commits.is_empty() {
        sessions.remove(&session_id);
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(&json!([]).to_string()));
    }

    let commits = session
        .continue_commits
        .iter()
        .map(|oid| repo.find_commit(*oid))
        .collect::<std::result::Result<Vec<_>, git2::Error>>()
        .map_err(map_err)?;

    let ProcessCommitsResult {
        commits,
        checked: checked_commits,
        continue_: continue_commits,
    } = process_commits(
        &repo,
        &data.settings,
        &commits,
        Some(std::mem::take(&mut session.checked_commits)),
    )
    .map_err(map_err)?;

    session.checked_commits = checked_commits;
    session.continue_commits = continue_commits;
    session.sent_pages += 1;

    println!(
        "git history from session {} results {} continues with {} commits, {}th page, analyzed in {} ms",
        request.session_id,
        commits.len(),
        session.continue_commits.len(),
        session.sent_pages,
        time_load.elapsed().as_micros() as f64 / 1000.
    );

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(&json!(commits).to_string()))
}
