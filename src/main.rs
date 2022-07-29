/* Copyright (C) 2018 Jonathan Lebon <jonathan@jlebon.com>
 * SPDX-License-Identifier: MIT
 * */

use ansi_term::{Colour, Style};
use clap::clap_app;
use std::error::Error;
use std::ffi::OsStr;

mod utils;

#[derive(Clone, Copy, PartialEq)]
enum OutputMode {
    Human,
    Listing,
    ListingCommits,
    NameOnly,
}

#[derive(Clone, Copy, PartialEq)]
enum BranchFilter {
    Recent,
    All,
    Merged,
    Unmerged,
}

struct BranchInfo {
    name: String,
    active: bool,
    timestamp: u64,
    timestamp_rel: String,
    summary: String,
    ahead: usize,
    oid: git2::Oid,
    upstream: Option<String>,
}

struct BranchesInfo {
    branches: Vec<BranchInfo>,
    n_merged: usize,
    n_unmerged: usize,
}

// XXX: make into flag/config
const RECENT_N: usize = 5;
const LOCAL_BRANCH_REF_PREFIX: &str = "refs/heads/";

fn main() {
    let matches = clap::clap_app!((clap::crate_name!()) =>
            (version: clap::crate_version!())
            (author: clap::crate_authors!())
            (about: clap::crate_description!())
            (@arg REPO: --repo +takes_value "Git repo to target")
            (@arg BRANCH: ... "Branches to list (or substrings)")
            (@arg verbose: -v --verbose "List added commits")
            (@arg all: -a --all "List all branches")
            (@arg merged: -m --merged "List only merged branches")
            (@arg unmerged: -u --unmerged "List only unmerged branches")
            (@arg reverse: -r --reverse "Reverse listing order")
            (@arg name_only: -n --("name-only") "Print branch names only")
    )
    .get_matches();

    /* just collapse to vector now for later */
    let maybe_patterns = matches.values_of("BRANCH").map(|values| values.collect());

    let filter = if matches.is_present("all")
        || (matches.is_present("merged") && matches.is_present("unmerged"))
    {
        BranchFilter::All
    } else if matches.is_present("merged") {
        BranchFilter::Merged
    } else if matches.is_present("unmerged") {
        BranchFilter::Unmerged
    } else {
        BranchFilter::Recent
    };

    let output_mode = if matches.is_present("verbose") {
        OutputMode::ListingCommits
    } else if matches.is_present("name_only") {
        OutputMode::NameOnly
    } else if filter != BranchFilter::Recent || maybe_patterns != None {
        OutputMode::Listing
    } else {
        OutputMode::Human
    };

    if let Err(e) = run(
        matches.value_of_os("REPO"),
        &maybe_patterns,
        output_mode,
        filter,
        matches.is_present("reverse"),
    ) {
        eprintln!("{} {}", Colour::Red.bold().paint("error:"), e);
        std::process::exit(1);
    }
}

fn run(
    repo_path: Option<&OsStr>,
    maybe_patterns: &Option<Vec<&str>>,
    output_mode: OutputMode,
    filter: BranchFilter,
    reverse: bool,
) -> Result<(), Box<dyn Error>> {
    let repo = match repo_path {
        Some(s) => git2::Repository::discover(s)?,
        None => git2::Repository::discover(std::env::current_dir()?)?,
    };

    let info = scan_branches(&repo, maybe_patterns, filter, reverse)?;

    match output_mode {
        OutputMode::Human => print_human(&repo, &info)?,
        OutputMode::NameOnly => info.branches.iter().for_each(|b| println!("{}", b.name)),
        _ => print_listing(
            &repo,
            &info.branches,
            output_mode == OutputMode::ListingCommits,
        )?,
    }

    Ok(())
}

fn scan_branches(
    repo: &git2::Repository,
    maybe_patterns: &Option<Vec<&str>>,
    filter: BranchFilter,
    reverse: bool,
) -> Result<BranchesInfo, Box<dyn Error>> {
    let default_sha = find_default_sha(repo)?;

    let mut n_merged: usize = 0;
    let mut n_unmerged: usize = 0;
    let mut branches: Vec<BranchInfo> = Vec::new();
    for branch in repo.branches(Some(git2::BranchType::Local))? {
        let (branch, branchtype) = branch?;
        assert!(branchtype == git2::BranchType::Local);

        let name = branch.name()?.unwrap();

        if let Some(ref patterns) = maybe_patterns {
            if patterns.iter().all(|&p| !name.contains(p)) {
                continue;
            }
        }

        let commit = branch.get().peel_to_commit()?;
        let oid = commit.id();

        // use upstream branch if defined, otherwise fallback to default
        let (upstream, upstream_sha) = if let Ok(b) = branch.upstream() {
            (
                Some(b.name()?.unwrap().into()),
                b.get().peel_to_commit()?.id(),
            )
        } else {
            (None, default_sha)
        };

        let (ahead, _) = repo.graph_ahead_behind(oid, upstream_sha)?;

        let merged = ahead == 0;
        if merged {
            n_merged += 1;
        } else {
            n_unmerged += 1;
        }

        if (filter == BranchFilter::Merged && !merged)
            || (filter == BranchFilter::Unmerged && merged)
        {
            continue;
        }

        assert!(commit.time().seconds() >= 0);
        let timestamp = commit.time().seconds() as u64;

        branches.push(BranchInfo {
            active: branch.is_head(),
            name: name.into(),
            summary: commit.summary().unwrap().into(),
            timestamp_rel: utils::epoch_to_relative_str(timestamp),
            timestamp,
            ahead,
            oid,
            upstream,
        });
    }

    // sort by timestamp (most recent first)
    branches.sort_unstable_by_key(|b| std::u64::MAX - b.timestamp);

    if filter == BranchFilter::Recent {
        branches.truncate(RECENT_N);
    }

    if reverse {
        branches.reverse();
    }

    Ok(BranchesInfo {
        branches,
        n_merged,
        n_unmerged,
    })
}

/// Get the default SHA against which comparisons should be made to determine +ahead number.
/// This is usually "master", or the default branch to check out after cloning.
fn find_default_sha(repo: &git2::Repository) -> Result<git2::Oid, Box<dyn Error>> {
    // go through all the remotes, and find which has a HEAD branch
    // then resolve that to the local branch
    let mut head_ref: Option<git2::Reference> = None;
    if let Ok(maybe_refs) = repo.references_glob("refs/remotes/*/HEAD") {
        for maybe_ref in maybe_refs {
            let r = maybe_ref?;
            if !r.is_remote() {
                continue;
            }

            let is_origin = r.name().unwrap().starts_with("refs/remotes/origin");

            // this ensures that we prefer "origin"; otherwise, we just fallback to whatever
            // the last remote with a HEAD is (normally, only one remote --the one used to
            // clone-- has it)
            head_ref = Some(r);
            if is_origin {
                break;
            }
        }
    }

    if let Some(hr) = head_ref {
        let r = hr.resolve()?;

        let name = r.name().unwrap();
        let remote_and_ref = &name["refs/remotes/".len()..];
        let v: Vec<&str> = remote_and_ref.splitn(2, '/').collect();
        assert!(v.len() == 2);
        let branch = v[1];

        // now find the local branch of the same name
        if let Ok(b) = repo.find_branch(branch, git2::BranchType::Local) {
            return Ok(b.get().peel_to_commit()?.id());
        }
    }

    // no HEAD remote ref, or not connected to a local branch, just guess "master" or "main", and
    // if that's not it, throw
    if let Ok(b) = repo.find_branch("master", git2::BranchType::Local) {
        return Ok(b.get().peel_to_commit()?.id());
    } else if let Ok(b) = repo.find_branch("main", git2::BranchType::Local) {
        return Ok(b.get().peel_to_commit()?.id());
    }

    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Couldn't find default branch",
    )))
}

fn print_human(repo: &git2::Repository, info: &BranchesInfo) -> Result<(), Box<dyn Error>> {
    let head = repo.head()?;
    if head.is_branch() {
        let name = head.name().unwrap();
        assert!(name.starts_with(LOCAL_BRANCH_REF_PREFIX));
        println!("On branch {}", &name[LOCAL_BRANCH_REF_PREFIX.len()..]);
    } else {
        println!("HEAD detached at {:.8}", head.peel_to_commit()?.id());
    }

    println!(
        "\
Recently active branches:
  (use \"git bstatus -a\" to list all branches)
  (use \"git bstatus -v\" to list commits)
"
    );

    if info.branches.len() < RECENT_N {
        print_branches(repo, &info.branches, false, true)?;
    } else {
        print_branches(repo, &info.branches[..RECENT_N], false, true)?;
    }

    // not worth printing if there's only master
    if info.n_unmerged > 0 || info.n_merged > 1 {
        println!(
            "
There are {} local branches ({} merged, {} unmerged).
  (use \"git bstatus -m\" or \"git bstatus -u\" to list them)\
",
            info.n_merged + info.n_unmerged,
            info.n_merged,
            info.n_unmerged
        );
    }

    Ok(())
}

fn print_listing(
    repo: &git2::Repository,
    branches: &[BranchInfo],
    commits: bool,
) -> Result<(), Box<dyn Error>> {
    print_branches(repo, branches, commits, false)?;

    Ok(())
}

fn print_branches(
    repo: &git2::Repository,
    branches: &[BranchInfo],
    list_commits: bool,
    tab: bool,
) -> Result<(), Box<dyn Error>> {
    if branches.is_empty() {
        return Ok(());
    }

    // super wasteful, but meh
    let max_name_len = branches.iter().map(|b| b.name.len()).max().unwrap();
    let max_timestamp_len = branches
        .iter()
        .map(|b| b.timestamp_rel.len())
        .max()
        .unwrap();
    let max_ahead = branches.iter().map(|b| b.ahead).max().unwrap();
    let max_ahead_len = utils::count_digits(max_ahead);

    // use prefix/suffix since regular paint() conflicts with branch_width
    let (green_prefix, green_suffix) = (Colour::Green.prefix(), Colour::Green.suffix());
    let (inert_prefix, inert_suffix) = {
        let s = Style::default();
        (s.prefix(), s.suffix())
    };

    for branch in branches {
        print!(
            "{star:>star_width$} {bp}{branch:branch_width$}{bs}  \
             {ago:>ago_width$} {gp}{ahead:+ahead_width$}{gs}",
            star = if branch.active { "*" } else { " " },
            star_width = if tab { 4 } else { 1 },
            branch = branch.name,
            branch_width = max_name_len,
            bp = if branch.active {
                green_prefix
            } else {
                inert_prefix
            },
            bs = if branch.active {
                green_suffix
            } else {
                inert_suffix
            },
            gp = green_prefix,
            gs = green_suffix,
            ago = branch.timestamp_rel,
            ago_width = max_timestamp_len,
            ahead = branch.ahead,
            ahead_width = max_ahead_len + 1, // add 1 for the + sign
        );

        if let Some(ref b) = branch.upstream {
            print!(
                " {gp}({branch}){gs}",
                gp = green_prefix,
                gs = green_suffix,
                branch = b
            );
        }

        if !list_commits {
            println!(" {}", branch.summary);
        } else {
            println!();

            let mut revwalk = repo.revwalk()?;
            revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
            revwalk.push(branch.oid)?;
            for (i, maybe_oid) in revwalk.enumerate() {
                let oid = maybe_oid?;
                let commit = repo.find_commit(oid)?;
                let summary = commit.summary().unwrap();
                println!("    {:.8} {}", oid, summary);
                if i >= branch.ahead {
                    break;
                }
            }
        }
    }

    Ok(())
}
