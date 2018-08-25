`git bstatus` is like `git status` for your branches:

```
$ git bstatus
On branch librpm-compat
Recently active branches:
  (use "git bstatus -a" to list all branches)
  (use "git bstatus -v" to list commits)

   * librpm-compat   1 day +1 build: Tweak rpm version check for HAVE_NEW_RPM_VERIFY
     master          1 day +0 (upstream/master) app: Add support for passing URLs to RPMs
     install-url    2 days +2 fixup! app: Add support for passing URLs to RPMs
     reuse-rpmdb    2 days +1 upgrader: Reuse existing rpmdb checkout if available
     rust-goop      2 days +2 fixup! build: Fix building rust in debug mode

There are 53 local branches (6 merged, 47 unmerged).
  (use "git bstatus -m" or "git bstatus -u" to list them)
```

(Though I alias it to `git lsb` for faster typing).

The main difference from `git branch` is that it's designed
to be friendlier to humans, in the way `git status` is:
- quick view of your recently active branches to make
  finding and switching between them a breeze
- column for human-formatted relative timestamp
- column for number of commits on that branch
- list added commits across branches with `-v`
- list only (un)merged branches with `-m/-u`

You may find that something like
`git branch -v --sort=-committerdate | head -n5` is
good enough for your purposes. In `git bstatus`, `-v` lists
all the commits that aren't merged into master (or the
tracked branch, if any):

```
$ git lsb -v
* librpm-compat   1 day +1
    e98a5941 build: Tweak rpm version check for HAVE_NEW_RPM_VERIFY
    04c0678f app: Add support for passing URLs to RPMs
  master          1 day +0 (upstream/master)
    04c0678f app: Add support for passing URLs to RPMs
  install-url    2 days +2
    2391b6d3 fixup! app: Add support for passing URLs to RPMs
    7f7b54b2 app: Add support for passing URLs to RPMs
    6a274b83 build-sys: Hard require Rust
  reuse-rpmdb    2 days +1
    eddafb00 upgrader: Reuse existing rpmdb checkout if available
    6a274b83 build-sys: Hard require Rust
  rust-goop      2 days +2
    c7a1d9f8 fixup! build: Fix building rust in debug mode
    f393131c build: Fix building rust in debug mode
    6a274b83 build-sys: Hard require Rust
```

In the above output, `+2` means that there are two commits
on that branch; the last commit listed is the first one that
is shared with the master branch.

# Installation

```
$ cargo install --path .
$ which git-bstatus
~/.cargo/bin/git-bstatus
```

To uninstall:

```
$ cargo uninstall git-bstatus
```
