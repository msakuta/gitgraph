use crate::{AnyhowError, MyData};
use actix_web::{get, web, HttpResponse, Responder};
use anyhow::Result;
use git2::{Oid, Repository};

#[get("/diff_stats/{commit_a}/{commit_b}")]
pub(crate) async fn get_diff_stats(
    data: web::Data<MyData>,
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
        .body(format!("[{},{}]", stats.insertions(), stats.deletions())))
}
