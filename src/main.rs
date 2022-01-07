use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::Result;
use dunce::canonicalize;
use git2::{Oid, Repository};
use serde::Serialize;
use serde_json::json;
use std::{
    collections::HashSet,
    convert::{TryFrom, TryInto},
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    time::Instant,
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
    home_path: PathBuf,
    settings: Settings,
}

async fn index(_data: web::Data<MyData>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(include_str!("../index.html"))
}

async fn get_commits(data: web::Data<MyData>) -> HttpResponse {
    let home_path = &data.home_path;

    let time_load = Instant::now();

    if let Ok(result) = process_files_git(home_path, &data.settings) {
        println!(
            "git history with {} commits analyzed in {} ms",
            result.len(),
            time_load.elapsed().as_micros() as f64 / 1000.
        );

        HttpResponse::Ok()
            .content_type("application/json")
            .body(&json!(result).to_string())
    } else {
        HttpResponse::InternalServerError().body("Internal server error!")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let settings: Settings = Opt::from_args()
        .try_into()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    println!("page_size: {}", settings.page_size);

    let data = web::Data::new(MyData {
        home_path: canonicalize(PathBuf::from(&settings.repo))?,
        settings,
    });
    let result = HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/", web::get().to(index))
            .route("/commits", web::get().to(get_commits))
            .route(
                "/js/jquery-3.1.0.min.js",
                web::get().to(|| async { include_str!("../js/jquery-3.1.0.min.js") }),
            )
            .route(
                "/js/gitgraph.js",
                web::get().to(|| async { include_str!("../js/gitgraph.js") }),
            )
            .route(
                "testlog.txt",
                web::get().to(|| async { include_str!("../testlog.txt") }),
            )
            .route(
                "testrefs.txt",
                web::get().to(|| async { include_str!("../testrefs.txt") }),
            )
    })
    .bind(("127.0.0.1", 8084))?
    .run()
    .await;

    result
}

#[allow(dead_code)]
struct MatchEntry {
    commit: Oid,
    path: PathBuf,
    start: usize,
    end: usize,
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

#[derive(Serialize)]
struct Stats {
    insertions: usize,
    deletions: usize,
}

#[derive(Serialize)]
struct CommitData {
    hash: String, // String is not the most efficient representation of the hash, but it's easy to serialize into a JSON
    message: String,
    stat: Option<Stats>,
    parents: Vec<String>,
}

fn process_files_git(_root: &Path, settings: &Settings) -> Result<Vec<CommitData>> {
    let repo = Repository::open(&settings.repo)?;
    let reference = if let Some(ref branch) = settings.branch {
        repo.resolve_reference_from_short_name(&branch)?
    } else {
        repo.head()?
    };

    let mut checked_commits = HashSet::new();
    let mut iter = 0;

    let mut next_refs = if settings.all {
        repo.references()?
            .map(|refs| refs.and_then(|refb| refb.peel_to_commit()))
            .collect::<std::result::Result<Vec<_>, _>>()?
    } else {
        vec![reference.peel_to_commit()?]
    };

    let mut ret = vec![];

    loop {
        for commit in &next_refs {
            if !checked_commits.insert(commit.id()) {
                continue;
            }

            let tree = if let Ok(tree) = commit.tree() {
                tree
            } else {
                continue;
            };

            if let Some(message) = commit.summary() {
                let mut iter = commit.parent_ids();
                iter.next();
                let multi_parents = iter.next().is_some();
                ret.push(CommitData {
                    hash: commit.id().to_string(),
                    message: message.to_owned(),
                    stat: if !multi_parents {
                        commit
                            .parents()
                            .next()
                            .and_then(|parent| parent.tree().ok())
                            .and_then(|parent| {
                                repo.diff_tree_to_tree(Some(&parent), Some(&tree), None)
                                    .ok()
                            })
                            .and_then(|diff| diff.stats().ok())
                            .and_then(|diff_stats| {
                                Some(Stats {
                                    insertions: diff_stats.insertions(),
                                    deletions: diff_stats.deletions(),
                                })
                            })
                    } else {
                        None
                    },
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
