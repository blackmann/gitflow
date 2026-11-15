# Gitflow

A native GitHub workspace built with **GPUI** (Zed's renderer) and GPUI Component, in the same design language as CloudWatch v2. Everything GitHub goes through the **`gh` CLI**: the app uses your `gh auth` login and never stores a token. A local clone adds `git` for the all-branch graph, fetch and PR checkout.

## Run

Requires macOS, Rust 1.88+, and a signed-in `gh` (`gh auth login`).

```sh
cargo run
```

Bundle an app: `./scripts/build-app.sh`, then `open build/Gitflow.app`. Development runs can use a scratch config with `GITFLOW_SUPPORT_DIR=/some/dir cargo run`.

## Layout

- **Sidebar**: repositories, added from a local clone (⌘O; its GitHub remote is detected) or a GitHub URL / `owner/name` (⇧⌘O). Right-click for new tab, Finder, GitHub, link a clone, or remove.
- **Tabs**: clicking a repository focuses its tab; ⌘-click or the + menu opens another. **Duplicate tab** (⇧⌘D, or right-click) copies the view and its selection: branch, folder/file, commit, PR and sub-page, run and job.
- **View bar**: Code, Commits, Pull requests, Actions, Releases (⌘1–⌘5).
- **Brief panel** (⌥⌘B): description, topics, stats, language, license, clone URLs, latest releases, recent runs and contributors.

## Views

- **Code**: branch popover, breadcrumbs, folder listing with README (Markdown), virtualized file viewer.
- **Commits**: a commit graph with lanes, Bezier merges, ref badges and HEAD ring. Local clones show every branch (or one); remote-only repos show one branch's history from the API. The search dims non-matching rows so the graph stays intact. Pick a commit to see its message, files and diff.
- **Pull requests**: open/merged/closed/all, search, checks, review state. The detail page has the conversation (Markdown), commits, checks and files changed. You can comment (⌘↵), approve, request changes, merge (merge/squash/rebase, delete branch), close/reopen, toggle draft, or check out locally.
- **Actions**: filter by workflow, run a workflow on the current branch, re-run all or failed jobs, cancel. Runs in progress refresh every 8 s. Job steps and full logs.
- **Releases**: notes and assets.

Merging, closing and dispatching ask for confirmation first.

## Preferences (⌘,)

General: starting view, graph depth, auto-refresh, relative dates. GitHub: account, `gh` path, default merge method, delete branch after merge, avatars. Appearance: system/light/dark, monospace font and size.

## Development

```sh
cargo test                     # unit tests: graph layout, diff parser, slugs, formatting
cargo test -- --ignored live   # every fetcher against real GitHub data via your gh
cargo clippy --all-targets
```

`gh.rs` wraps `gh`/`git`, `graph.rs` is the lane layout and painter, and `workspace/` holds the UI. Zed's own git graph (`crates/git_ui/src/git_graph.rs`) is GPL-3 and tied to Zed's workspace crates, so it can't be imported as a module. This graph is an independent implementation with the same visual approach.
