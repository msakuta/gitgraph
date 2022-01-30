use crate::{AnyhowError, ServerState};
use actix_web::{get, http, web, HttpResponse, Responder};
use anyhow::Result;
use git2::{DiffStatsFormat, Oid, Repository};
use std::path::PathBuf;

#[get("/diff_summary/{commit_a}/{commit_b}")]
pub(crate) async fn get_diff_summary(
    data: web::Data<ServerState>,
    web::Path((commit_a, commit_b)): web::Path<(String, String)>,
) -> std::result::Result<impl Responder, AnyhowError> {
    let get_diff_int = || -> Result<git2::DiffStats> {
        let repo = Repository::open(&data.settings.repo)?;
        let commit_a = repo.find_commit(Oid::from_str(&commit_a)?)?.tree()?;
        let commit_b = repo.find_commit(Oid::from_str(&commit_b)?)?.tree()?;
        let diff = repo.diff_tree_to_tree(Some(&commit_a), Some(&commit_b), None)?;
        Ok(diff.stats()?)
    };
    let stats = get_diff_int()?;
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        // Keep cache for 1 week since git hash guarantees uniqueness
        .header(http::header::CACHE_CONTROL, "max-age=604800")
        .body(format!("[{},{}]", stats.insertions(), stats.deletions())))
}

#[get("/diff_stats/{commit_a}/{commit_b}")]
pub(crate) async fn get_diff_stats(
    data: web::Data<ServerState>,
    web::Path((commit_a, commit_b)): web::Path<(String, String)>,
) -> std::result::Result<impl Responder, AnyhowError> {
    let get_diff_int = || -> Result<_> {
        let repo = Repository::open(&data.settings.repo)?;
        let commit_a = repo.find_commit(Oid::from_str(&commit_a)?)?.tree()?;
        let commit_b = repo.find_commit(Oid::from_str(&commit_b)?)?.tree()?;
        let diff = repo.diff_tree_to_tree(Some(&commit_a), Some(&commit_b), None)?;
        let diff_stats = diff.stats()?;
        let buf = diff_stats.to_buf(DiffStatsFormat::FULL, 80)?;
        Ok(buf.to_owned())
    };
    let buf = get_diff_int()?;
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        // Keep cache for 1 week since git hash guarantees uniqueness
        .header(http::header::CACHE_CONTROL, "max-age=604800")
        .body(buf))
}

#[get("/diff/{commit_a}/{commit_b}")]
pub(crate) async fn get_diff(
    data: web::Data<ServerState>,
    web::Path((commit_a, commit_b)): web::Path<(String, String)>,
) -> std::result::Result<impl Responder, AnyhowError> {
    let get_diff_int = || -> Result<_> {
        let repo = Repository::open(&data.settings.repo)?;
        let commit_a = repo.find_commit(Oid::from_str(&commit_a)?)?.tree()?;
        let commit_b = repo.find_commit(Oid::from_str(&commit_b)?)?.tree()?;
        let diff = repo.diff_tree_to_tree(Some(&commit_a), Some(&commit_b), None)?;

        let mut ret = vec![];
        let mut hunk_header: Option<(PathBuf, String)> = None;
        let mut hunk_accum = String::new();

        let mut flush_header = |hunk_header: &mut Option<(PathBuf, String)>,
                                hunk_accum: &mut String| {
            if let Some((path, _header)) = std::mem::take(hunk_header) {
                ret.push(format!(
                    "Path: {}\n{}",
                    path.to_str().unwrap_or(""),
                    hunk_accum
                ));
                hunk_accum.clear();
            }
        };

        diff.print(git2::DiffFormat::Patch, |delta, hunk, line| {
            if let Some(hunk) = hunk {
                if let (Some(path), Ok(header), Ok(content)) = (
                    delta.new_file().path(),
                    std::str::from_utf8(hunk.header()),
                    std::str::from_utf8(line.content()),
                ) {
                    if hunk_header
                        .as_ref()
                        .map(|h: &(PathBuf, String)| h.0 != path || h.1 != header)
                        .unwrap_or(true)
                    {
                        flush_header(&mut hunk_header, &mut hunk_accum);
                        hunk_header = Some((path.to_owned(), header.to_owned()));
                    }
                    hunk_accum += &format!(
                        " {} {} {}",
                        line.old_lineno()
                            .map(|l| format!("{:>4}", l))
                            .unwrap_or_else(|| "    ".to_string()),
                        line.new_lineno()
                            .map(|l| format!("{:>4}", l))
                            .unwrap_or_else(|| "    ".to_string()),
                        content
                    );
                }
            }
            true
        })?;

        flush_header(&mut hunk_header, &mut hunk_accum);

        Ok(ret)
    };
    let ret = get_diff_int()?;
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        // Cache is annoying for debugging for now
        // .header(http::header::CACHE_CONTROL, "max-age=604800")
        .json(ret))
}
