mod commits;
mod diff;
mod sessions;

use crate::{
    commits::{
        get_commits, get_commits_hash, get_commits_multi, get_commits_session, get_message,
        get_meta,
    },
    diff::{get_diff_stats, get_diff_summary},
    sessions::{Session, SessionId},
};
#[cfg(debug_assertions)]
use actix_files::NamedFile;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use dunce::canonicalize;
use git2::Repository;
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
        short = "P",
        long,
        help = "Number of commits in a page",
        default_value = "50"
    )]
    page_size: usize,
    #[structopt(short, long, help = "Verbose flag")]
    verbose: bool,
    #[structopt(
        short,
        long,
        help = "The address to listen to.",
        default_value = "0.0.0.0"
    )]
    listen_address: String,
    #[structopt(
        short = "p",
        long,
        help = "The port to listen to.",
        default_value = "8084"
    )]
    listen_port: u16,
    #[structopt(
        short,
        long,
        help = "Add an entry to list of directory names to ignore"
    )]
    ignore_dirs: Vec<String>,
}

struct ServerState {
    settings: Settings,
    sessions: Mutex<HashMap<SessionId, Session>>,
}

#[cfg(debug_assertions)]
macro_rules! get_static_file {
    ($file:expr, $_mime_type:expr) => {{
        async fn f() -> actix_web::Result<NamedFile> {
            (|| -> anyhow::Result<NamedFile> {
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
    ($file:expr, $mime_type:expr) => {
        || async {
            HttpResponse::Ok()
                .content_type($mime_type)
                .body(include_str!($file))
        }
    };
}

async fn get_refs(data: web::Data<ServerState>) -> HttpResponse {
    if let Ok(repo) = Repository::open(&data.settings.repo) {
        if let Ok(refs) = repo.references() {
            HttpResponse::Ok().content_type("application/json").body(
                &json!(refs
                    .filter_map(|r| {
                        let r = r.ok()?;
                        let name = r.name()?;
                        let hash = r.peel_to_commit().ok()?.id().to_string();
                        Some((name.to_owned(), hash))
                    })
                    .collect::<HashMap<_, _>>())
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

async fn get_index(_data: web::Data<ServerState>) -> impl Responder {
    let file = include_str!("../public/index.html");
    #[cfg(debug_assertions)]
    let file_name = "http://localhost:8080/bundle.js";
    #[cfg(not(debug_assertions))]
    let file_name = "./js/bundle.js";
    let file = file.replace("{{source}}", file_name);
    HttpResponse::Ok()
        .content_type("text/html")
        .header("Access-Control-Allow-Origin", "*")
        .body(file)
}

impl actix_web::error::ResponseError for AnyhowError {}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let settings: Settings = Opt::from_args()
        .try_into()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let listen_address = settings.listen_address.clone();
    let listen_port = settings.listen_port;

    let data = web::Data::new(ServerState {
        settings,
        sessions: Mutex::new(HashMap::new()),
    });
    println!("Serving at {}:{}", listen_address, listen_port);
    let result = HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/", web::get().to(get_index))
            .service(get_commits)
            .service(get_commits_hash)
            .service(get_commits_multi)
            .service(get_commits_session)
            .service(get_message)
            .service(get_meta)
            .route("/refs", web::get().to(get_refs))
            .service(get_diff_summary)
            .service(get_diff_stats)
            .route(
                "/js/jquery-3.1.0.min.js",
                web::get().to(get_static_file!(
                    "../js/jquery-3.1.0.min.js",
                    "text/javascript"
                )),
            )
            .route(
                "/js/bundle.js",
                web::get().to(get_static_file!("../dist/bundle.js", "text/javascript")),
            )
    })
    .bind((listen_address, listen_port))?
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
    listen_address: String,
    listen_port: u16,
    ignore_dirs: HashSet<OsString>,
}

// It's a bit awkward to convert from Opt to Settings, but some settings are hard to write
// conversion code inside structopt annotations.
impl TryFrom<Opt> for Settings {
    type Error = anyhow::Error;

    fn try_from(src: Opt) -> std::result::Result<Self, Self::Error> {
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
            listen_address: src.listen_address,
            listen_port: src.listen_port,
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
