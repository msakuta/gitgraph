use std::sync::Arc;

use crate::{AnyhowError, ServerState};
// use actix_web::{get, http, web, HttpResponse, Responder};
use anyhow::Result;
use axum::{
    extract::{Path, State},
    Json,
};
use git2::{DiffStatsFormat, Oid, Repository};
use serde::Serialize;

// #[get("/diff_summary/{commit_a}/{commit_b}")]
// pub(crate) async fn get_diff_summary(
//     data: web::Data<ServerState>,
//     web::Path((commit_a, commit_b)): web::Path<(String, String)>,
// ) -> std::result::Result<impl Responder, AnyhowError> {
//     let get_diff_int = || -> Result<git2::DiffStats> {
//         let repo = Repository::open(&data.settings.repo)?;
//         let commit_a = repo.find_commit(Oid::from_str(&commit_a)?)?.tree()?;
//         let commit_b = repo.find_commit(Oid::from_str(&commit_b)?)?.tree()?;
//         let diff = repo.diff_tree_to_tree(Some(&commit_a), Some(&commit_b), None)?;
//         Ok(diff.stats()?)
//     };
//     let stats = get_diff_int()?;
//     Ok(HttpResponse::Ok()
//         .content_type("application/json")
//         // Keep cache for 1 week since git hash guarantees uniqueness
//         .header(http::header::CACHE_CONTROL, "max-age=604800")
//         .body(format!("[{},{}]", stats.insertions(), stats.deletions())))
// }

pub(crate) async fn get_diff_summary(
    State(data): State<Arc<ServerState>>,
    Path((commit_a, commit_b)): Path<(String, String)>,
) -> std::result::Result<Json<String>, AnyhowError> {
    let get_diff_int = || -> Result<git2::DiffStats> {
        let repo = Repository::open(&data.settings.repo)?;
        let commit_a = repo.find_commit(Oid::from_str(&commit_a)?)?.tree()?;
        let commit_b = repo.find_commit(Oid::from_str(&commit_b)?)?.tree()?;
        let diff = repo.diff_tree_to_tree(Some(&commit_a), Some(&commit_b), None)?;
        Ok(diff.stats()?)
    };
    let stats = get_diff_int()?;
    Ok(
        // Keep cache for 1 week since git hash guarantees uniqueness
        // .header(http::header::CACHE_CONTROL, "max-age=604800")
        Json(format!("[{},{}]", stats.insertions(), stats.deletions())),
    )
}

// #[get("/diff_stats/{commit_a}/{commit_b}")]
// pub(crate) async fn get_diff_stats(
//     data: web::Data<ServerState>,
//     web::Path((commit_a, commit_b)): web::Path<(String, String)>,
// ) -> std::result::Result<impl Responder, AnyhowError> {
//     let get_diff_int = || -> Result<_> {
//         let repo = Repository::open(&data.settings.repo)?;
//         let commit_a = repo.find_commit(Oid::from_str(&commit_a)?)?.tree()?;
//         let commit_b = repo.find_commit(Oid::from_str(&commit_b)?)?.tree()?;
//         let diff = repo.diff_tree_to_tree(Some(&commit_a), Some(&commit_b), None)?;
//         let diff_stats = diff.stats()?;
//         let buf = diff_stats.to_buf(DiffStatsFormat::FULL, 80)?;
//         Ok(buf.to_owned())
//     };
//     let buf = get_diff_int()?;
//     Ok(HttpResponse::Ok()
//         .content_type("text/plain")
//         // Keep cache for 1 week since git hash guarantees uniqueness
//         .header(http::header::CACHE_CONTROL, "max-age=604800")
//         .body(buf))
// }

#[derive(Serialize, Debug)]
pub(crate) struct FileDiff {
    file: String,
    hunks: Vec<String>,
}

pub(crate) async fn get_diff(
    State(data): State<Arc<ServerState>>,
    Path((commit_a, commit_b)): Path<(String, String)>,
) -> Result<Json<Vec<FileDiff>>, AnyhowError> {
    let get_diff_int = || -> Result<_> {
        let repo = Repository::open(&data.settings.repo)?;
        let commit_a = repo.find_commit(Oid::from_str(&commit_a)?)?.tree()?;
        let commit_b = repo.find_commit(Oid::from_str(&commit_b)?)?.tree()?;
        let diff = repo.diff_tree_to_tree(Some(&commit_a), Some(&commit_b), None)?;

        let mut ret = vec![];
        let mut hunk_header: Option<(String, String)> = None;
        let mut hunk_accum = String::new();
        let mut file_accum = None;
        let mut lines = 0;

        fn is_new_file(
            file_accum: &Option<FileDiff>,
            hunk_header: &Option<(String, String)>,
        ) -> bool {
            let file_accum = if let Some(file_accum) = file_accum {
                file_accum
            } else {
                return false;
            };
            let hunk_header = if let Some(hunk_header) = hunk_header {
                hunk_header
            } else {
                return false;
            };
            file_accum.file != hunk_header.0
        }

        let mut flush_header = |file_accum: &mut Option<FileDiff>,
                                hunk_header: &mut Option<(String, String)>,
                                hunk_accum: &mut String| {
            if let Some(hunk_header) = std::mem::take(hunk_header) {
                if is_new_file(&file_accum, &Some(hunk_header)) {
                    if let Some(file_accum) = std::mem::take(file_accum) {
                        ret.push(file_accum);
                    }
                } else if let Some(file_accum) = file_accum {
                    file_accum.hunks.push(std::mem::take(hunk_accum));
                }

                hunk_accum.clear();
            }
        };

        diff.print(git2::DiffFormat::Patch, |delta, hunk, line| {
            lines += 1;
            if file_accum.is_none() {
                if let Some(file) = delta.new_file().path().and_then(|s| s.to_str()) {
                    file_accum = Some(FileDiff {
                        file: file.to_owned(),
                        hunks: vec![],
                    })
                }
            }
            if let Some(hunk) = hunk {
                if let (Some(path), Ok(header), Ok(content)) = (
                    delta.new_file().path().and_then(|s| s.to_str()),
                    std::str::from_utf8(hunk.header()),
                    std::str::from_utf8(line.content()),
                ) {
                    if hunk_header
                        .as_ref()
                        .map(|h: &(String, String)| h.0 != path || h.1 != header)
                        .unwrap_or(true)
                    {
                        flush_header(&mut file_accum, &mut hunk_header, &mut hunk_accum);
                        hunk_header = Some((path.to_owned(), header.to_owned()));
                    }
                    let line = format!(
                        " {} {} {}",
                        line.old_lineno()
                            .map(|l| format!("{:>4}", l))
                            .unwrap_or_else(|| "    ".to_string()),
                        line.new_lineno()
                            .map(|l| format!("{:>4}", l))
                            .unwrap_or_else(|| "    ".to_string()),
                        content
                    );
                    print!(
                        "{} {}: {}",
                        path,
                        is_new_file(&file_accum, &hunk_header),
                        line
                    );
                    hunk_accum += &line;
                }
            }
            true
        })?;

        flush_header(&mut file_accum, &mut hunk_header, &mut hunk_accum);

        println!("lines: {}", lines);

        Ok(ret)
    };
    let ret = get_diff_int()?;
    Ok(
        // Cache is annoying for debugging for now
        // .header(http::header::CACHE_CONTROL, "max-age=604800")
        Json(ret),
    )
}
