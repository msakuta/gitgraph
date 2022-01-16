mod commits;
mod sessions;

use crate::{
    commits::{get_commits, get_commits_hash, get_commits_multi, get_commits_session},
    sessions::{Session, SessionId},
};
#[cfg(debug_assertions)]
use actix_files::NamedFile;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use dunce::canonicalize;
use git2::{Oid, Repository};
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    convert::{TryFrom, TryInto},
    env,
    ffi::OsString,
    path::PathBuf,
    sync::Mutex,
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(help = "Root repo to grep")]
    repo: Option<PathBuf>,
    #[structopt(short, long, help = "Branch name")]
    branch: Option<String>,
    #[structopt(
        short,
        long,
        help = "Search from all branches. Ignores -b option if given"
    )]
    all: bool,
    #[structopt(short, long, help = "Depth to search into git commit history")]
    depth: Option<usize>,
    #[structopt(
        short,
        long,
        help = "Number of commits in a page",
        default_value = "50"
    )]
    page_size: usize,
    #[structopt(short, long, help = "Verbose flag")]
    verbose: bool,
    #[structopt(short, long, help = "Add an entry to list of extensions to search")]
    extensions: Vec<String>,
    #[structopt(
        short,
        long,
        help = "Add an entry to list of directory names to ignore"
    )]
    ignore_dirs: Vec<String>,
}

struct MyData {
    settings: Settings,
    sessions: Mutex<HashMap<SessionId, Session>>,
}

#[cfg(debug_assertions)]
macro_rules! get_static_file {
    ($file:expr) => {{
        async fn f() -> actix_web::Result<NamedFile> {
            (|| -> Result<NamedFile> {
                let path = if &$file[..3] == "../" {
                    &$file[3..]
                } else {
                    $file
                };
                let abs_path = std::env::current_dir()?.join(path);
                println!("path: {:?}", abs_path);
                Ok(NamedFile::open(abs_path)?)
            })()
            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))
        }
        f
    }};
}

#[cfg(not(debug_assertions))]
macro_rules! get_static_file {
    ($file:expr) => {
        || async {
            HttpResponse::Ok()
                .content_type("text/html")
                .body(include_str!($file))
        }
    };
}

async fn get_refs(data: web::Data<MyData>) -> HttpResponse {
    if let Ok(repo) = Repository::open(&data.settings.repo) {
        if let Ok(refs) = repo.references() {
            HttpResponse::Ok().content_type("application/json").body(
                &json!(refs
                    .filter_map(|r| {
                        let r = r.ok()?;
                        let name = r.name()?;
                        let hash = r.peel_to_commit().ok()?.id().to_string();
                        Some([name.to_owned(), hash])
                    })
                    .collect::<Vec<_>>())
                .to_string(),
            )
        } else {
            HttpResponse::InternalServerError().body("Refs could not be acquired")
        }
    } else {
        HttpResponse::InternalServerError().body("Refs could not be acquired")
    }
}

/// Adapter error type that connects anyhow::Error and actix-web errors. Using newtype pattern to get around orphan rule.
#[derive(Debug)]
struct AnyhowError(anyhow::Error);

impl std::fmt::Display for AnyhowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<anyhow::Error> for AnyhowError {
    fn from(a: anyhow::Error) -> Self {
        Self(a)
    }
}

impl actix_web::error::ResponseError for AnyhowError {}

// #[get("/diff/{commit_a}/{commit_b}")]
async fn get_diff(
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let settings: Settings = Opt::from_args()
        .try_into()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    println!("page_size: {}", settings.page_size);

    let data = web::Data::new(MyData {
        settings,
        sessions: Mutex::new(HashMap::new()),
    });
    let result = HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/", web::get().to(get_static_file!("../index.html")))
            .service(get_commits)
            .service(get_commits_hash)
            .service(get_commits_multi)
            .service(get_commits_session)
            .route("/refs", web::get().to(get_refs))
            .route("/diff_stats/{commit_a}/{commit_b}", web::get().to(get_diff))
            .route(
                "/js/jquery-3.1.0.min.js",
                web::get().to(get_static_file!("../js/jquery-3.1.0.min.js")),
            )
            .route(
                "/js/gitgraph.js",
                web::get().to(get_static_file!("../js/gitgraph.js")),
            )
    })
    .bind(("127.0.0.1", 8084))?
    .run()
    .await;

    result
}

#[derive(Debug)]
struct Settings {
    repo: PathBuf,
    branch: Option<String>,
    all: bool,
    depth: Option<usize>,
    page_size: usize,
    verbose: bool,
    extensions: HashSet<OsString>,
    ignore_dirs: HashSet<OsString>,
}

// It's a bit awkward to convert from Opt to Settings, but some settings are hard to write
// conversion code inside structopt annotations.
impl TryFrom<Opt> for Settings {
    type Error = anyhow::Error;

    fn try_from(src: Opt) -> std::result::Result<Self, Self::Error> {
        let default_exts = [
            ".sh", ".js", ".tcl", ".pl", ".py", ".rb", ".c", ".cpp", ".h", ".rc", ".rci", ".dlg",
            ".pas", ".dpr", ".cs", ".rs",
        ];
        let default_ignore_dirs = [".hg", ".svn", ".git", ".bzr", "node_modules", "target"]; // Probably we could ignore all directories beginning with a dot.

        Ok(Self {
            repo: canonicalize(
                src.repo.unwrap_or_else(|| {
                    PathBuf::from(env::current_dir().unwrap().to_str().unwrap())
                }),
            )
            .expect("Canonicalized path"),
            branch: src.branch,
            all: src.all,
            depth: src.depth,
            page_size: src.page_size,
            verbose: src.verbose,
            extensions: if src.extensions.is_empty() {
                default_exts.iter().map(|ext| ext[1..].into()).collect()
            } else {
                default_exts
                    .iter()
                    .map(|ext| ext[1..].into())
                    .chain(src.extensions.iter().map(|ext| ext[1..].into()))
                    .collect()
            },
            ignore_dirs: if src.ignore_dirs.is_empty() {
                default_ignore_dirs.iter().map(|ext| ext.into()).collect()
            } else {
                default_ignore_dirs
                    .iter()
                    .map(|ext| ext.into())
                    .chain(src.ignore_dirs.iter().map(|ext| ext.into()))
                    .collect()
            },
        })
    }
}
