//! API implementations for commits request

use actix_web::{web, HttpResponse, Responder};
use anyhow::Result;
use git2::{Commit, Oid, Repository};
use serde::Serialize;
use serde_json::json;
use std::{collections::HashSet, time::Instant};

use super::{AnyhowError, MyData, Settings};

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

/// Default commit query (head or all, depending on settings)
#[actix_web::get("/commits")]
async fn get_commits(data: web::Data<MyData>) -> actix_web::Result<impl Responder> {
    let time_load = Instant::now();

    let result = (|| -> Result<Vec<CommitData>> {
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
        process_files_git(&repo, &data.settings, &head)
    })()
    .map_err::<AnyhowError, _>(|err| err.into())?;

    println!(
        "git history with {} commits analyzed in {} ms",
        result.len(),
        time_load.elapsed().as_micros() as f64 / 1000.
    );

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(&json!(result).to_string()))
}

/// Single commit query
#[actix_web::get("/commits/{id}")]
async fn get_commits_hash(
    data: web::Data<MyData>,
    web::Path(id): web::Path<String>,
) -> actix_web::Result<impl Responder> {
    let time_load = Instant::now();

    let result = (|| -> Result<_> {
        let repo = Repository::open(&data.settings.repo)?;
        let commit = [repo.find_commit(Oid::from_str(&id)?)?];
        process_files_git(&repo, &data.settings, &commit)
    })()
    .map_err::<AnyhowError, _>(|err| err.into())?;

    println!(
        "git history with {} commits from {}analyzed in {} ms",
        result.len(),
        id,
        time_load.elapsed().as_micros() as f64 / 1000.
    );

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(&json!(result).to_string()))
}

/// Multiple commits in request body
#[actix_web::post("/commits")]
async fn get_commits_multi(
    data: web::Data<MyData>,
    request: web::Json<Vec<String>>,
) -> actix_web::Result<impl Responder> {
    let time_load = Instant::now();

    let result = (|| -> Result<Vec<CommitData>> {
        let repo = Repository::open(&data.settings.repo)?;

        let commits = request
            .iter()
            .map(|name| repo.find_commit(Oid::from_str(name)?))
            .collect::<std::result::Result<Vec<_>, git2::Error>>()?;

        process_files_git(&repo, &data.settings, &commits)
    })()
    .map_err::<AnyhowError, _>(|err| err.into())?;

    println!(
        "git history from {} commits results {} analyzed in {} ms",
        request.len(),
        result.len(),
        time_load.elapsed().as_micros() as f64 / 1000.
    );

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(&json!(result).to_string()))
}

fn process_files_git(
    repo: &Repository,
    settings: &Settings,
    head: &[Commit],
) -> Result<Vec<CommitData>> {
    let mut checked_commits = HashSet::new();
    let mut iter = 0;

    let mut next_refs = head.to_vec();

    let mut ret = vec![];

    loop {
        for commit in &next_refs {
            if !checked_commits.insert(commit.id()) {
                continue;
            }

            if let Some(message) = commit.summary() {
                let mut iter = commit.parent_ids();
                iter.next();
                ret.push(CommitData {
                    hash: commit.id().to_string(),
                    message: message.to_owned(),
                    parents: commit.parent_ids().map(|id| id.to_string()).collect(),
                });
                if settings.page_size <= ret.len() {
                    return Ok(ret);
                }
            }
        }
        next_refs = next_refs
            .iter()
            .map(|reference| reference.parent_ids())
            .flatten()
            .filter(|reference| !checked_commits.contains(reference))
            .map(|id| repo.find_commit(id))
            .collect::<std::result::Result<Vec<_>, git2::Error>>()?;

        if settings.verbose {
            eprintln!("[{}] Next round has {} refs...", iter, next_refs.len());
        }
        iter += 1;
        if next_refs.is_empty() || settings.depth.map(|depth| depth <= iter).unwrap_or(false) {
            break;
        }
    }
    Ok(ret)
}
