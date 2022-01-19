use anyhow::Result;
use git2::{Commit, Oid, Repository};
use std::{
    collections::{BinaryHeap, HashSet},
    iter::FromIterator,
};

use super::{CommitData, Settings};
struct CommitWrap<'a>(Commit<'a>);

impl std::cmp::PartialEq for CommitWrap<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.time() == other.0.time()
    }
}

impl std::cmp::Eq for CommitWrap<'_> {}

impl std::cmp::PartialOrd for CommitWrap<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.time().partial_cmp(&other.0.time())
    }
}

impl std::cmp::Ord for CommitWrap<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        println!("Comparing {} and {}", self.0.id(), other.0.id());
        self.0.time().cmp(&other.0.time())
    }
}

pub(super) struct ProcessCommitsResult {
    pub commits: Vec<CommitData>,
    pub checked: HashSet<Oid>,
    pub continue_: HashSet<Oid>,
}

pub(super) fn process_commits(
    repo: &Repository,
    settings: &Settings,
    head: &[Commit],
    checked_commits: Option<HashSet<Oid>>,
) -> Result<ProcessCommitsResult> {
    let mut checked_commits = checked_commits.unwrap_or_else(HashSet::new);

    let mut next_refs = BinaryHeap::from_iter(head.iter().cloned().map(CommitWrap));

    let mut ret = vec![];

    while let Some(CommitWrap(commit)) = next_refs.pop() {
        if !checked_commits.insert(commit.id()) {
            continue;
        }

        for parent in commit.parent_ids() {
            if let Ok(parent) = repo.find_commit(parent) {
                next_refs.push(CommitWrap(parent));
            }
        }

        if let Some(message) = commit.summary() {
            ret.push(CommitData {
                hash: commit.id().to_string(),
                message: message.to_owned(),
                parents: commit.parent_ids().map(|id| id.to_string()).collect(),
            });
            if settings.page_size <= ret.len() {
                return Ok(ProcessCommitsResult {
                    commits: ret,
                    checked: checked_commits,
                    continue_: next_refs.into_iter().map(|commit| commit.0.id()).collect(),
                });
            }
        }
    }
    Ok(ProcessCommitsResult {
        commits: ret,
        checked: checked_commits,
        continue_: next_refs.into_iter().map(|commit| commit.0.id()).collect(),
    })
}
