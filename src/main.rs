mod commits;
mod diff;
mod sessions;

use crate::{
    commits::{
        get_commits,
        //  get_commits_hash, get_commits_multi, get_commits_session, get_message,
        get_meta,
    },
    diff::{
        get_diff,
        // get_diff_stats,
        get_diff_summary,
    },
    sessions::{Session, SessionId},
};
// #[cfg(debug_assertions)]
// use actix_files::NamedFile;
// use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use dunce::canonicalize;
use git2::Repository;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    convert::{TryFrom, TryInto},
    env,
    ffi::OsString,
    io::Read,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
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
        async fn f() -> Result<String, AnyhowError> {
            Ok((|| -> anyhow::Result<String> {
                let path = if &$file[..3] == "../" {
                    &$file[3..]
                } else {
                    $file
                };
                let abs_path = std::env::current_dir()?.join(path);
                println!("path: {:?}", abs_path);
                let content = std::fs::read_to_string(abs_path)?;
                Ok(content)
            })()?)
        }
        f
    }};
}

#[cfg(not(debug_assertions))]
macro_rules! get_static_file {
    ($file:expr, $mime_type:expr) => {{
        async fn f() -> impl IntoResponse {
            ([(header::CONTENT_TYPE, $mime_type)], include_str!($file))
        }
        f
    }};
}

// async fn get_refs(data: web::Data<ServerState>) -> HttpResponse {
//     if let Ok(repo) = Repository::open(&data.settings.repo) {
//         if let Ok(refs) = repo.references() {
//             HttpResponse::Ok().content_type("application/json").body(
//                 &json!(refs
//                     .filter_map(|r| {
//                         let r = r.ok()?;
//                         let name = r.name()?;
//                         let hash = r.peel_to_commit().ok()?.id().to_string();
//                         Some((name.to_owned(), hash))
//                     })
//                     .collect::<HashMap<_, _>>())
//                 .to_string(),
//             )
//         } else {
//             HttpResponse::InternalServerError().body("Refs could not be acquired")
//         }
//     } else {
//         HttpResponse::InternalServerError().body("Refs could not be acquired")
//     }
// }

async fn get_refs(State(data): State<Arc<ServerState>>) -> (StatusCode, Json<Value>) {
    if let Ok(repo) = Repository::open(&data.settings.repo) {
        if let Ok(refs) = repo.references() {
            (
                StatusCode::OK,
                Json(json!(refs
                    .filter_map(|r| {
                        let r = r.ok()?;
                        let name = r.name()?;
                        let hash = r.peel_to_commit().ok()?.id().to_string();
                        Some((name.to_owned(), hash))
                    })
                    .collect::<HashMap<_, _>>())),
            )
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!("Refs could not be acquired")),
            )
        }
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!("Refs could not be acquired")),
        )
    }
}

/// Adapter error type that connects anyhow::Error and actix-web errors. Using newtype pattern to get around orphan rule.
#[derive(Debug)]
struct AnyhowError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AnyhowError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

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

// async fn get_index(_data: web::Data<ServerState>) -> actix_web::Result<impl Responder> {
//     #[cfg(debug_assertions)]
//     let file = {
//         let path = "./public/index.html";
//         let abs_path = std::env::current_dir()?.join(path);
//         println!("path: {:?}", abs_path);
//         let mut file = String::new();
//         std::fs::File::open(abs_path)?.read_to_string(&mut file)?;
//         println!("source? {:?}", file.find("{{source}}"));
//         let file_name = "http://localhost:8080/bundle.js";
//         file.replace("{{source}}", file_name)
//     };
//     #[cfg(not(debug_assertions))]
//     let file = {
//         let file = include_str!("../public/index.html");
//         let file_name = "./js/bundle.js";
//         file.replace("{{source}}", file_name)
//     };
//     Ok(HttpResponse::Ok().content_type("text/html").body(&file))
// }

async fn get_index() -> Html<String> {
    #[cfg(debug_assertions)]
    let file = (|| -> anyhow::Result<_> {
        let path = "./public/index.html";
        let abs_path = std::env::current_dir()?.join(path);
        println!("path: {:?}", abs_path);
        let mut file = String::new();
        std::fs::File::open(abs_path)?.read_to_string(&mut file)?;
        println!("source? {:?}", file.find("{{source}}"));
        let file_name = "http://localhost:8080/bundle.js";
        Ok(file.replace("{{source}}", file_name))
    })()
    .unwrap();
    #[cfg(not(debug_assertions))]
    let file = {
        let file = include_str!("../public/index.html");
        let file_name = "./js/bundle.js";
        file.replace("{{source}}", file_name)
    };
    Html(file)
}

// impl actix_web::error::ResponseError for AnyhowError {}

// #[actix_web::main]
// async fn main() -> std::io::Result<()> {
//     let settings: Settings = Opt::from_args()
//         .try_into()
//         .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

//     let listen_address = settings.listen_address.clone();
//     let listen_port = settings.listen_port;

//     let data = web::Data::new(ServerState {
//         settings,
//         sessions: Mutex::new(HashMap::new()),
//     });
//     println!("Serving at {}:{}", listen_address, listen_port);
//     let result = HttpServer::new(move || {
//         App::new()
//             .app_data(data.clone())
//             .route("/", web::get().to(get_index))
//             .service(get_commits)
//             .service(get_commits_hash)
//             .service(get_commits_multi)
//             .service(get_commits_session)
//             .service(get_message)
//             .service(get_meta)
//             .route("/refs", web::get().to(get_refs))
//             .service(get_diff_summary)
//             .service(get_diff_stats)
//             .service(get_diff)
//             .route(
//                 "/js/bundle.js",
//                 web::get().to(get_static_file!("../dist/bundle.js", "text/javascript")),
//             )
//     })
//     .bind((listen_address, listen_port))?
//     .run()
//     .await;

//     result
// }

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let settings: Settings = Opt::from_args()
        .try_into()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let data = std::sync::Arc::new(ServerState {
        settings,
        sessions: Mutex::new(HashMap::new()),
    });

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(get_index))
        .route("/refs", get(get_refs))
        .route("/commit-query/refs/heads/:refname", get(get_commits))
        .route("/diff_summary/:id1/:id2", get(get_diff_summary))
        .route("/diff/:id1/:id2", get(get_diff))
        .route("/commits/:id/meta", get(get_meta))
        .route(
            "/js/bundle.js",
            get(get_static_file!("../dist/bundle.js", "text/javascript")),
        )
        .with_state(data);

    // run our app with hyper, listening globally on port 3000
    // let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
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
