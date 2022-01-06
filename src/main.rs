use actix_web::{error, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use anyhow::{anyhow, Result};
use colored::*;
use dunce::canonicalize;
use git2::{Commit, ObjectType, Oid, Repository, Tree};
use handlebars::Handlebars;
use regex::Regex;
use serde::Serialize;
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    convert::{TryFrom, TryInto},
    env,
    ffi::OsString,
    path::{Path, PathBuf},
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
        short = "o",
        long,
        help = "Turn off showing matches to a file only once; the default behavior is that if the same file with the same name has different versions that matches, they will not be printed."
    )]
    no_once_file: bool,
    #[structopt(
        short = "c",
        long,
        help = "Disable color coding for the output, default is to use colors in terminal"
    )]
    no_color_code: bool,
    #[structopt(
        short = "g",
        long,
        help = "Disable output grouping. Better for machine inputs"
    )]
    no_output_grouping: bool,
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

async fn index(data: web::Data<MyData>) -> HttpResponse {
    let home_path = &data.home_path;
    let reg = Handlebars::new();

    if let Ok(result) = process_files_git(home_path, &data.settings) {
        HttpResponse::Ok().content_type("text/html").body(
            reg.render_template(
                include_str!("../index.html"),
                &json!({
                    "commits": result,
                }),
            )
            .unwrap(),
            // format!(
            // "<html><body><h1>Git, world! {:?}</h1><ul>{}</ul></body></html>",
            // home_path,
            // result
            //     .iter()
            //     .fold("".to_string(), |acc, cur| acc + &format!("<li>{}", cur)
        )
    } else {
        HttpResponse::InternalServerError().body("Internal server error!")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let settings: Settings = Opt::from_args()
        .try_into()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let data = web::Data::new(MyData {
        home_path: canonicalize(PathBuf::from(&settings.repo))?,
        settings,
    });
    let result = HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/", web::get().to(index))
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
    once_file: bool,
    color_code: bool,
    output_grouping: bool,
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
            once_file: !src.no_once_file,
            color_code: !src.no_color_code,
            output_grouping: !src.no_output_grouping,
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

struct ProcessTree<'a> {
    settings: &'a Settings,
    repo: &'a Repository,
    checked_paths: HashSet<PathBuf>,
    checked_blobs: HashSet<Oid>,
    checked_trees: HashSet<Oid>,
    walked: usize,
    skipped_blobs: usize,
    all_matches: Vec<MatchEntry>,
}

impl<'a> ProcessTree<'a> {
    fn process(&mut self, tree: &Tree, commit: &Commit, path: &Path, visited: &mut bool) {
        if self.checked_trees.contains(&tree.id()) {
            return;
        }
        self.checked_trees.insert(tree.id());
        self.walked += 1;
    }
}

#[derive(Serialize)]
struct CommitData {
    message: String,
    insertions: usize,
    deletions: usize,
}

fn process_files_git(_root: &Path, settings: &Settings) -> Result<Vec<CommitData>> {
    let repo = Repository::open(&settings.repo)?;
    let reference = if let Some(ref branch) = settings.branch {
        repo.resolve_reference_from_short_name(&branch)?
    } else {
        repo.head()?
    };

    let mut process_tree = ProcessTree {
        settings,
        repo: &repo,
        checked_paths: HashSet::new(),
        checked_blobs: HashSet::new(),
        checked_trees: HashSet::new(),
        walked: 0,
        skipped_blobs: 0,
        all_matches: vec![],
    };
    let mut checked_commits = HashMap::new();
    let mut iter = 0;

    let mut next_refs = if settings.all {
        repo.references()?
            .map(|refs| refs.and_then(|refb| refb.peel_to_commit()))
            .collect::<std::result::Result<Vec<_>, _>>()?
    } else {
        vec![reference.peel_to_commit()?]
    };

    let mut ret = vec![];
    let mut prev_tree = None;

    loop {
        for commit in &next_refs {
            if checked_commits.contains_key(&commit.id()) {
                continue;
            }
            let entry = checked_commits.entry(commit.id()).or_insert(false);

            let tree = if let Ok(tree) = commit.tree() {
                tree
            } else {
                continue;
            };

            process_tree.process(&tree, commit, &PathBuf::from(""), entry);

            if let Some((message, diff_stats)) = commit.message().zip(
                prev_tree
                    .and_then(|prev_tree| {
                        repo.diff_tree_to_tree(Some(&prev_tree), Some(&tree), None)
                            .ok()
                    })
                    .and_then(|diff| diff.stats().ok()),
            ) {
                ret.push(CommitData {
                    message: message.to_owned(),
                    insertions: diff_stats.insertions(),
                    deletions: diff_stats.deletions(),
                });
            }
            prev_tree = Some(tree);
        }
        next_refs = next_refs
            .iter()
            .map(|reference| reference.parent_ids())
            .flatten()
            .filter(|reference| !checked_commits.contains_key(reference))
            .map(|id| repo.find_commit(id))
            .collect::<std::result::Result<Vec<_>, git2::Error>>()?;

        if settings.verbose {
            eprintln!(
                "[{}] {} Matches in {} files {} skipped blobs... Next round has {} refs...",
                iter,
                process_tree.all_matches.len(),
                process_tree.walked,
                process_tree.skipped_blobs,
                next_refs.len()
            );
        }
        iter += 1;
        if next_refs.is_empty() || settings.depth.map(|depth| depth <= iter).unwrap_or(false) {
            break;
        }
    }
    Ok(ret)
}
