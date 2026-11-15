use crate::{
    avatars::Avatars,
    gh::{self, PrFilter},
    graph::{self, Graph},
    menu::{self as popover, Menu},
    settings::{self, Repo, Saved, Settings, View},
    theme::*,
};
use gpui::*;
use gpui_component::{input::InputState, resizable::ResizableState};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};
mod actions;
mod chrome;
mod code;
mod commits;
mod diff;
mod preferences;
mod pulls;
mod releases;
mod ui;

actions!(
    gitflow,
    [
        Refresh,
        ToggleSidebar,
        ToggleBrief,
        CloseTab,
        DuplicateTab,
        NextTab,
        PreviousTab,
        OpenPreferences,
        AddLocal,
        AddUrl,
        ShowCode,
        ShowCommits,
        ShowPulls,
        ShowActions,
        ShowReleases,
        Submit,
        Back,
        Quit
    ]
);

/// A request's lifecycle. Data stays visible while a refresh is in flight.
pub struct Load<T> {
    pub data: Option<T>,
    pub error: Option<String>,
    pub loading: bool,
}
impl<T> Default for Load<T> {
    fn default() -> Self {
        Self {
            data: None,
            error: None,
            loading: false,
        }
    }
}
impl<T> Load<T> {
    /// Still waiting for the first answer.
    pub fn pending(&self) -> bool {
        self.data.is_none() && self.error.is_none()
    }
}

pub struct CommitGraph {
    pub commits: Vec<gh::Commit>,
    pub graph: Graph,
}
pub enum BlobView {
    Text(Vec<SharedString>),
    Binary(u64),
    TooLarge(u64),
}
/// A diff flattened for a virtualized list: file headers, then numbered lines.
pub struct DiffView {
    pub files: Vec<gh::FileDiff>,
    pub rows: Vec<(usize, Option<usize>)>,
    pub additions: u32,
    pub deletions: u32,
}
impl DiffView {
    pub const MAX_LINES: usize = 1500;
    pub fn new(files: Vec<gh::FileDiff>) -> Self {
        let mut rows = vec![];
        for (f, file) in files.iter().enumerate() {
            rows.push((f, None));
            rows.extend((0..file.lines.len().min(Self::MAX_LINES)).map(|l| (f, Some(l))));
        }
        Self {
            additions: files.iter().map(|f| f.additions).sum(),
            deletions: files.iter().map(|f| f.deletions).sum(),
            files,
            rows,
        }
    }
    pub fn header_row(&self, file: usize) -> Option<usize> {
        self.rows.iter().position(|r| *r == (file, None))
    }
}
pub struct CommitInfo {
    pub message: String,
    pub diff: Arc<DiffView>,
}
/// A job log with a header row wherever the step changes.
pub struct LogView {
    pub rows: Vec<(bool, SharedString)>,
    pub truncated: bool,
}

/// Everything fetched for one repository, shared by all of its tabs.
#[derive(Default)]
pub struct RepoData {
    pub info: Load<gh::RepoInfo>,
    pub branches: Load<Vec<gh::Branch>>,
    pub contributors: Load<Vec<gh::Contributor>>,
    pub releases: Load<Vec<gh::Release>>,
    pub workflows: Load<Vec<gh::Workflow>>,
    pub runs: HashMap<Option<u64>, Load<Vec<gh::Run>>>,
    pub pulls: HashMap<PrFilter, Load<Vec<gh::Pull>>>,
    pub pull: HashMap<u64, Load<gh::PullDetail>>,
    pub pull_diff: HashMap<u64, Load<Arc<DiffView>>>,
    pub commits: HashMap<Option<String>, Load<Arc<CommitGraph>>>,
    pub commit: HashMap<String, Load<Arc<CommitInfo>>>,
    pub tree: HashMap<(String, String), Load<Vec<gh::Entry>>>,
    pub blob: HashMap<(String, String), Load<Arc<BlobView>>>,
    pub jobs: HashMap<u64, Load<Vec<gh::Job>>>,
    pub log: HashMap<u64, Load<Arc<LogView>>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PrPage {
    #[default]
    Conversation,
    Commits,
    Files,
    Checks,
}

pub struct Tab {
    pub id: u64,
    pub repo: String,
    pub view: View,
    /// Code view and remote history: the branch shown; None follows the default.
    pub branch: Option<String>,
    /// Local graph: a single ref, or None for every branch.
    pub graph_ref: Option<String>,
    pub path: String,
    pub file: Option<String>,
    pub commit: Option<String>,
    pub pr_filter: PrFilter,
    pub pr: Option<u64>,
    pub pr_page: PrPage,
    pub workflow: Option<u64>,
    pub run: Option<u64>,
    pub job: Option<u64>,
    pub log_open: bool,
    pub search: Entity<InputState>,
    pub composer: Entity<InputState>,
    pub list_scroll: UniformListScrollHandle,
    pub diff_scroll: UniformListScrollHandle,
    pub log_scroll: UniformListScrollHandle,
    pub page_scroll: ScrollHandle,
}

struct Notice {
    text: String,
    ok: bool,
}
impl Notice {
    fn warn(text: impl Into<String>) -> Option<Self> {
        Some(Self {
            text: text.into(),
            ok: false,
        })
    }
    fn ok(text: impl Into<String>) -> Option<Self> {
        Some(Self {
            text: text.into(),
            ok: true,
        })
    }
}

#[derive(Clone, Copy, PartialEq)]
enum PrefsPage {
    General,
    GitHub,
    Appearance,
}

pub struct Workspace {
    repos: Vec<Repo>,
    data: HashMap<String, RepoData>,
    tabs: Vec<Tab>,
    active: Option<usize>,
    next_id: u64,
    sidebar: bool,
    brief: bool,
    repo_search: Entity<InputState>,
    url_input: Entity<InputState>,
    add_url_open: bool,
    adding: bool,
    popup: Option<Entity<Menu>>,
    popup_position: Point<Pixels>,
    popup_anchor: Option<&'static str>,
    dismissed: Option<(&'static str, Instant)>,
    anchors: Rc<RefCell<HashMap<&'static str, Bounds<Pixels>>>>,
    font_names: Rc<std::cell::OnceCell<Vec<String>>>,
    notice: Option<Notice>,
    /// A user-started mutation in flight, e.g. "Merging #12".
    busy: Option<String>,
    /// Clear this tab's comment box on the next frame (needs a window).
    pending_clear: Option<u64>,
    user: Load<gh::User>,
    gh_version: Load<String>,
    prefs_open: bool,
    prefs_page: PrefsPage,
    prefs_search: Entity<InputState>,
    gh_path_input: Entity<InputState>,
    avatars: Avatars,
    sidebar_panels: Entity<ResizableState>,
    brief_panels: Entity<ResizableState>,
    last_refresh: Instant,
    last_poll: Instant,
    focus: FocusHandle,
}

fn input(value: &str, placeholder: &str, window: &mut Window, cx: &mut App) -> Entity<InputState> {
    cx.new(|cx| {
        let mut s = InputState::new(window, cx).placeholder(placeholder.to_string());
        s.set_value(value.to_string(), window, cx);
        s
    })
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let repo_search = input("", "Filter repositories…", window, cx);
        cx.observe(&repo_search, |_, _, cx| cx.notify()).detach();
        let prefs_search = input("", "Search settings…", window, cx);
        cx.observe(&prefs_search, |_, _, cx| cx.notify()).detach();
        let url_input = input("", "owner/name or https://github.com/owner/name", window, cx);
        let gh_path = settings::get(cx).gh_path.clone();
        gh::set_gh_path(&gh_path);
        let gh_path_input = input(&gh_path, "Search /opt/homebrew/bin, /usr/local/bin", window, cx);
        let sidebar_panels = cx.new(|_| ResizableState::default());
        let brief_panels = cx.new(|_| ResizableState::default());
        for panels in [&sidebar_panels, &brief_panels] {
            cx.observe(panels, |_, _, cx| cx.notify()).detach();
        }
        let focus = cx.focus_handle();
        focus.focus(window);
        let mut s = Self {
            repos: Saved::load().repos,
            data: HashMap::new(),
            tabs: vec![],
            active: None,
            next_id: 0,
            sidebar: true,
            brief: true,
            repo_search,
            url_input,
            add_url_open: false,
            adding: false,
            popup: None,
            popup_position: Point::default(),
            popup_anchor: None,
            dismissed: None,
            anchors: Rc::default(),
            font_names: Rc::default(),
            notice: None,
            busy: None,
            pending_clear: None,
            user: Load::default(),
            gh_version: Load::default(),
            prefs_open: false,
            prefs_page: PrefsPage::General,
            prefs_search,
            gh_path_input,
            avatars: Avatars::default(),
            sidebar_panels,
            brief_panels,
            last_refresh: Instant::now(),
            last_poll: Instant::now(),
            focus,
        };
        s.load_account(cx);
        if let Some(first) = s.repos.first().map(Repo::key) {
            s.open_repo(&first, window, cx);
        }
        cx.spawn_in(window, async move |this, cx| {
            loop {
                smol::Timer::after(Duration::from_millis(250)).await;
                if this.update(cx, |s, cx| s.tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        s
    }

    // ── Tabs ────────────────────────────────────────────────────────────────

    fn current(&self) -> Option<&Tab> {
        self.active.and_then(|i| self.tabs.get(i))
    }
    fn current_mut(&mut self) -> Option<&mut Tab> {
        self.active.and_then(|i| self.tabs.get_mut(i))
    }
    fn repo(&self, key: &str) -> Option<&Repo> {
        self.repos.iter().find(|r| r.key() == key)
    }
    fn new_tab(&mut self, repo: &str, view: View, window: &mut Window, cx: &mut Context<Self>) -> Tab {
        self.next_id += 1;
        let search = input("", "Filter…", window, cx);
        cx.observe(&search, |_, _, cx| cx.notify()).detach();
        let composer = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .auto_grow(3, 12)
                .placeholder("Leave a comment · Markdown supported")
        });
        Tab {
            id: self.next_id,
            repo: repo.to_string(),
            view,
            branch: None,
            graph_ref: None,
            path: String::new(),
            file: None,
            commit: None,
            pr_filter: PrFilter::Open,
            pr: None,
            pr_page: PrPage::Conversation,
            workflow: None,
            run: None,
            job: None,
            log_open: false,
            search,
            composer,
            list_scroll: UniformListScrollHandle::new(),
            diff_scroll: UniformListScrollHandle::new(),
            log_scroll: UniformListScrollHandle::new(),
            page_scroll: ScrollHandle::new(),
        }
    }
    /// Focus the repository's existing tab, or open one on the default view.
    fn open_repo(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(i) = self.tabs.iter().position(|t| t.repo == key) {
            self.active = Some(i);
        } else {
            let view = settings::get(cx).default_view;
            let tab = self.new_tab(key, view, window, cx);
            self.tabs.push(tab);
            self.active = Some(self.tabs.len() - 1);
        }
        cx.notify();
    }
    /// Always open a fresh tab, even when the repository already has one.
    fn open_repo_new_tab(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let view = settings::get(cx).default_view;
        let tab = self.new_tab(key, view, window, cx);
        let at = self.active.map(|i| i + 1).unwrap_or(self.tabs.len());
        self.tabs.insert(at, tab);
        self.active = Some(at);
        cx.notify();
    }
    /// Copy a tab with its view and selection; inputs keep their text.
    fn duplicate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(src) = self.tabs.get(index) else {
            return;
        };
        let (repo, view) = (src.repo.clone(), src.view);
        let search = src.search.read(cx).value().to_string();
        let mut tab = self.new_tab(&repo, view, window, cx);
        let src = &self.tabs[index];
        tab.branch = src.branch.clone();
        tab.graph_ref = src.graph_ref.clone();
        tab.path = src.path.clone();
        tab.file = src.file.clone();
        tab.commit = src.commit.clone();
        tab.pr_filter = src.pr_filter;
        tab.pr = src.pr;
        tab.pr_page = src.pr_page;
        tab.workflow = src.workflow;
        tab.run = src.run;
        tab.job = src.job;
        tab.log_open = src.log_open;
        tab.search
            .update(cx, |s, cx| s.set_value(search, window, cx));
        self.tabs.insert(index + 1, tab);
        self.active = Some(index + 1);
        cx.notify();
    }
    fn close(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        self.active = match self.active {
            _ if self.tabs.is_empty() => None,
            Some(a) if a > index => Some(a - 1),
            Some(a) if a == index => Some(index.min(self.tabs.len() - 1)),
            other => other,
        };
        cx.notify();
    }
    fn close_others(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.tabs.len() {
            let keep = self.tabs.remove(index);
            self.tabs = vec![keep];
            self.active = Some(0);
            cx.notify();
        }
    }
    fn set_view(&mut self, view: View, cx: &mut Context<Self>) {
        if let Some(t) = self.current_mut() {
            t.view = view;
        }
        cx.notify();
    }

    // ── Data ────────────────────────────────────────────────────────────────

    /// Start `job` unless its slot already has an answer (or `force`) or is loading.
    fn fetch<T: Send + 'static>(
        &mut self,
        repo: &str,
        slot: impl Fn(&mut RepoData) -> &mut Load<T> + 'static,
        force: bool,
        job: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        let l = slot(self.data.entry(repo.to_string()).or_default());
        if l.loading || (!force && !l.pending()) {
            return;
        }
        l.loading = true;
        let repo = repo.to_string();
        cx.spawn(async move |this, cx| {
            let result = smol::unblock(job).await;
            let _ = this.update(cx, |s, cx| {
                let l = slot(s.data.entry(repo).or_default());
                l.loading = false;
                match result {
                    Ok(v) => {
                        l.data = Some(v);
                        l.error = None;
                    }
                    Err(e) => l.error = Some(e.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn data(&self, repo: &str) -> Option<&RepoData> {
        self.data.get(repo)
    }
    fn default_branch(&self, repo: &str) -> Option<String> {
        self.data(repo)?.info.data.as_ref().map(|i| i.default_branch())
    }
    /// The branch a Code view or remote graph shows.
    fn branch_of(&self, t: &Tab) -> Option<String> {
        t.branch.clone().or_else(|| self.default_branch(&t.repo))
    }
    /// The graph's cache key: a ref, or None for all branches of a local clone.
    fn graph_key(&self, t: &Tab) -> Option<Option<String>> {
        let local = self.repo(&t.repo).is_some_and(|r| r.path.is_some());
        if local {
            Some(t.graph_ref.clone())
        } else {
            self.branch_of(t).map(Some)
        }
    }

    /// Request whatever the active tab shows. With `force`, re-fetch it.
    fn ensure(&mut self, force: bool, cx: &mut Context<Self>) {
        let Some(t) = self.current() else { return };
        let key = t.repo.clone();
        let Some(repo) = self.repo(&key).cloned() else {
            return;
        };
        let slug = repo.slug();
        let view = t.view;
        let s = slug.clone();
        self.fetch(&key, |d| &mut d.info, force, move || gh::repo_info(&s), cx);
        if self.brief {
            let s = slug.clone();
            self.fetch(&key, |d| &mut d.contributors, force, move || gh::contributors(&s), cx);
            let s = slug.clone();
            self.fetch(&key, |d| &mut d.releases, force, move || gh::releases(&s), cx);
            let s = slug.clone();
            self.fetch(
                &key,
                |d| d.runs.entry(None).or_default(),
                force,
                move || gh::runs(&s, None),
                cx,
            );
        }
        let Some(t) = self.current() else { return };
        let branch = self.branch_of(t);
        match view {
            View::Code => {
                let s = slug.clone();
                self.fetch(&key, |d| &mut d.branches, force, move || gh::branches(&s), cx);
                let Some(branch) = branch else { return };
                let t = self.current().unwrap();
                let (path, file) = (t.path.clone(), t.file.clone());
                let (s, b, p) = (slug.clone(), branch.clone(), path.clone());
                let k = (branch.clone(), path.clone());
                self.fetch(
                    &key,
                    move |d| d.tree.entry(k.clone()).or_default(),
                    force,
                    move || gh::tree(&s, &p, &b),
                    cx,
                );
                let readme = self
                    .data(&key)
                    .and_then(|d| d.tree.get(&(branch.clone(), path.clone())))
                    .and_then(|l| l.data.as_ref())
                    .and_then(|entries| {
                        entries
                            .iter()
                            .find(|e| !e.is_dir() && e.name.to_lowercase().starts_with("readme"))
                            .map(|e| e.path.clone())
                    });
                for target in file.into_iter().chain(readme) {
                    let (s, b, p) = (slug.clone(), branch.clone(), target.clone());
                    let k = (branch.clone(), target);
                    self.fetch(
                        &key,
                        move |d| d.blob.entry(k.clone()).or_default(),
                        force,
                        move || load_blob(&s, &p, &b),
                        cx,
                    );
                }
            }
            View::Commits => {
                let s = slug.clone();
                self.fetch(&key, |d| &mut d.branches, force, move || gh::branches(&s), cx);
                let Some(graph_key) = self.graph_key(self.current().unwrap()) else {
                    return;
                };
                let limit = settings::get(cx).commit_limit;
                let (s, path, r) = (slug.clone(), repo.path.clone(), graph_key.clone());
                let k = graph_key.clone();
                self.fetch(
                    &key,
                    move |d| d.commits.entry(k.clone()).or_default(),
                    force,
                    move || load_graph(&s, path.as_deref(), r.as_deref(), limit),
                    cx,
                );
                if let Some(sha) = self.current().unwrap().commit.clone() {
                    let (s, path, k) = (slug.clone(), repo.path.clone(), sha.clone());
                    self.fetch(
                        &key,
                        move |d| d.commit.entry(k.clone()).or_default(),
                        false,
                        move || {
                            let c = gh::commit_detail(&s, path.as_deref(), &sha)?;
                            Ok(Arc::new(CommitInfo {
                                message: c.message,
                                diff: Arc::new(DiffView::new(c.files)),
                            }))
                        },
                        cx,
                    );
                }
            }
            View::Pulls => {
                let t = self.current().unwrap();
                let (filter, pr, page) = (t.pr_filter, t.pr, t.pr_page);
                let s = slug.clone();
                self.fetch(
                    &key,
                    move |d| d.pulls.entry(filter).or_default(),
                    force,
                    move || gh::pulls(&s, filter),
                    cx,
                );
                if let Some(n) = pr {
                    let s = slug.clone();
                    self.fetch(
                        &key,
                        move |d| d.pull.entry(n).or_default(),
                        force,
                        move || gh::pull(&s, n),
                        cx,
                    );
                    if page == PrPage::Files {
                        let s = slug.clone();
                        self.fetch(
                            &key,
                            move |d| d.pull_diff.entry(n).or_default(),
                            force,
                            move || Ok(Arc::new(DiffView::new(gh::pull_diff(&s, n)?))),
                            cx,
                        );
                    }
                }
            }
            View::Actions => {
                let t = self.current().unwrap();
                let (workflow, run, log) = (t.workflow, t.run, t.log_open);
                let job = t.job.or_else(|| {
                    let jobs = self.data(&key)?.jobs.get(&run?)?.data.as_ref()?;
                    jobs.first().map(|j| j.database_id)
                });
                let s = slug.clone();
                self.fetch(&key, |d| &mut d.workflows, force, move || gh::workflows(&s), cx);
                let s = slug.clone();
                self.fetch(
                    &key,
                    move |d| d.runs.entry(workflow).or_default(),
                    force,
                    move || gh::runs(&s, workflow),
                    cx,
                );
                if let Some(run) = run {
                    let s = slug.clone();
                    self.fetch(
                        &key,
                        move |d| d.jobs.entry(run).or_default(),
                        force,
                        move || gh::jobs(&s, run),
                        cx,
                    );
                    if let (Some(job), true) = (job, log) {
                        let s = slug.clone();
                        self.fetch(
                            &key,
                            move |d| d.log.entry(job).or_default(),
                            force,
                            move || Ok(Arc::new(load_log(gh::job_log(&s, job)?))),
                            cx,
                        );
                    }
                }
            }
            View::Releases => {
                let s = slug.clone();
                self.fetch(&key, |d| &mut d.releases, force, move || gh::releases(&s), cx);
            }
        }
    }
    fn refresh(&mut self, _: &Refresh, _: &mut Window, cx: &mut Context<Self>) {
        self.last_refresh = Instant::now();
        // A refresh also re-reads the selected commit and the branch list.
        if let Some(t) = self.current() {
            let key = t.repo.clone();
            let sha = t.commit.clone();
            if let Some(d) = self.data.get_mut(&key)
                && let Some(sha) = sha
            {
                d.commit.remove(&sha);
            }
        }
        self.ensure(true, cx);
        if self.user.error.is_some() {
            self.load_account(cx);
        }
        cx.notify();
    }
    /// Periodic work: repaint for avatars, auto-refresh, and poll running Actions.
    fn tick(&mut self, cx: &mut Context<Self>) {
        if self.avatars.take_dirty() {
            cx.notify();
        }
        if self.prefs_open {
            return;
        }
        let every = settings::get(cx).refresh_secs;
        if every > 0 && self.last_refresh.elapsed() > Duration::from_secs(every as u64) {
            self.last_refresh = Instant::now();
            self.ensure(true, cx);
            return;
        }
        let Some(t) = self.current() else { return };
        if t.view != View::Actions || self.last_poll.elapsed() < Duration::from_secs(8) {
            return;
        }
        let active = self
            .data(&t.repo)
            .and_then(|d| d.runs.get(&t.workflow))
            .and_then(|l| l.data.as_ref())
            .is_some_and(|runs| runs.iter().any(gh::Run::active));
        if active {
            self.last_poll = Instant::now();
            self.ensure(true, cx);
        }
    }
    fn load_account(&mut self, cx: &mut Context<Self>) {
        self.user = Load {
            loading: true,
            ..Default::default()
        };
        cx.spawn(async move |this, cx| {
            let (user, version) = smol::unblock(|| (gh::current_user(), gh::version())).await;
            let _ = this.update(cx, |s, cx| {
                s.user.loading = false;
                match user {
                    Ok(u) => s.user.data = Some(u),
                    Err(e) => s.user.error = Some(e.to_string()),
                }
                match version {
                    Ok(v) => s.gh_version.data = Some(v),
                    Err(e) => s.gh_version.error = Some(e.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Run a user-initiated change, report it, then refresh what it touched.
    fn mutate(
        &mut self,
        label: String,
        job: impl FnOnce() -> anyhow::Result<String> + Send + 'static,
        done: impl FnOnce(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        if self.busy.is_some() {
            return;
        }
        self.busy = Some(label.clone());
        self.notice = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = smol::unblock(job).await;
            let _ = this.update(cx, |s, cx| {
                s.busy = None;
                match result {
                    Ok(out) => {
                        let out = out.trim();
                        s.notice = Notice::ok(if out.is_empty() || out.len() > 200 {
                            format!("{label} — done.")
                        } else {
                            out.lines().last().unwrap_or(out).trim_start_matches('✓').trim().to_string()
                        });
                        done(s, cx);
                    }
                    Err(e) => s.notice = Notice::warn(e.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn pr_action(&mut self, number: u64, action: gh::PrAction, cx: &mut Context<Self>) {
        let Some(t) = self.current() else { return };
        let Some(repo) = self.repo(&t.repo).cloned() else {
            return;
        };
        let key = repo.key();
        let slug = repo.slug();
        let label = match &action {
            gh::PrAction::Comment(_) => format!("Commenting on #{number}"),
            gh::PrAction::Approve => format!("Approving #{number}"),
            gh::PrAction::RequestChanges(_) => format!("Requesting changes on #{number}"),
            gh::PrAction::Merge { .. } => format!("Merging #{number}"),
            gh::PrAction::Close => format!("Closing #{number}"),
            gh::PrAction::Reopen => format!("Reopening #{number}"),
            gh::PrAction::Ready => format!("Marking #{number} ready"),
            gh::PrAction::Draft => format!("Converting #{number} to draft"),
            gh::PrAction::Checkout(_) => format!("Checking out #{number}"),
        };
        let clears_composer = matches!(
            action,
            gh::PrAction::Comment(_) | gh::PrAction::RequestChanges(_)
        );
        let tab = t.id;
        self.mutate(
            label,
            move || gh::pr_action(&slug, number, &action),
            move |s, cx| {
                if let Some(d) = s.data.get_mut(&key) {
                    d.pull.remove(&number);
                    d.pulls.clear();
                    d.pull_diff.remove(&number);
                    d.commits.clear();
                }
                if clears_composer {
                    s.pending_clear = Some(tab);
                }
                s.ensure(false, cx);
            },
            cx,
        );
    }
    fn run_action(&mut self, run: u64, action: gh::RunAction, cx: &mut Context<Self>) {
        let Some(t) = self.current() else { return };
        let Some(repo) = self.repo(&t.repo).cloned() else {
            return;
        };
        let (key, slug) = (repo.key(), repo.slug());
        let label = match &action {
            gh::RunAction::Rerun => "Re-running all jobs".to_string(),
            gh::RunAction::RerunFailed => "Re-running failed jobs".into(),
            gh::RunAction::Cancel => "Cancelling run".into(),
            gh::RunAction::Dispatch { reference, .. } => format!("Starting workflow on {reference}"),
        };
        self.mutate(
            label,
            move || gh::run_action(&slug, run, &action),
            move |s, cx| {
                if let Some(d) = s.data.get_mut(&key) {
                    d.runs.clear();
                    d.jobs.remove(&run);
                }
                s.last_poll = Instant::now();
                s.ensure(false, cx);
            },
            cx,
        );
    }
    fn fetch_local(&mut self, cx: &mut Context<Self>) {
        let Some(t) = self.current() else { return };
        let Some(repo) = self.repo(&t.repo).cloned() else {
            return;
        };
        let Some(path) = repo.path.clone() else { return };
        let key = repo.key();
        self.mutate(
            format!("Fetching {}", repo.name),
            move || gh::fetch(&path).map(|_| String::new()),
            move |s, cx| {
                if let Some(d) = s.data.get_mut(&key) {
                    d.commits.clear();
                    d.branches = Load::default();
                }
                s.ensure(false, cx);
            },
            cx,
        );
    }

    // ── Repositories ────────────────────────────────────────────────────────

    fn add_repo(&mut self, repo: Repo, window: &mut Window, cx: &mut Context<Self>) {
        let key = repo.key();
        match self.repos.iter_mut().find(|r| r.key() == key) {
            // Re-adding a remote repo from a clone upgrades it to local.
            Some(existing) => {
                if repo.path.is_some() {
                    existing.path = repo.path;
                    self.data.remove(&key);
                }
            }
            None => self.repos.push(repo),
        }
        self.save_repos();
        self.open_repo(&key, window, cx);
    }
    fn save_repos(&mut self) {
        if let Err(e) = Saved::save(&self.repos) {
            self.notice = Notice::warn(format!("Could not save repositories: {e}"));
        }
    }
    fn remove_repo(&mut self, key: &str, cx: &mut Context<Self>) {
        self.repos.retain(|r| r.key() != key);
        self.data.remove(key);
        while let Some(i) = self.tabs.iter().position(|t| t.repo == key) {
            self.close(i, cx);
        }
        self.save_repos();
        cx.notify();
    }
    fn add_local(&mut self, _: &AddLocal, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: true,
            prompt: Some("Add repository".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            for path in paths {
                let found = smol::unblock({
                    let path = path.clone();
                    move || {
                        let top = gh::git(&path, &["rev-parse", "--show-toplevel"])
                            .map(|p| PathBuf::from(p.trim()))
                            .unwrap_or(path);
                        gh::local_remote(&top).map(|(o, n)| (o, n, top))
                    }
                })
                .await;
                let _ = this.update_in(cx, |s, w, cx| match found {
                    Ok((owner, name, top)) => s.add_repo(
                        Repo {
                            owner,
                            name,
                            path: Some(top),
                        },
                        w,
                        cx,
                    ),
                    Err(e) => {
                        s.notice = Notice::warn(format!("{}: {e}", path.display()));
                        cx.notify();
                    }
                });
            }
        })
        .detach();
    }
    fn open_add_url(&mut self, _: &AddUrl, window: &mut Window, cx: &mut Context<Self>) {
        self.popup = None;
        self.add_url_open = true;
        self.url_input.update(cx, |s, cx| {
            s.set_value("", window, cx);
            s.focus(window, cx);
        });
        cx.notify();
    }
    fn submit_url(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.url_input.read(cx).value().to_string();
        let Some((owner, name)) = gh::parse_slug(&text) else {
            self.notice = Notice::warn("Enter owner/name or a github.com URL.");
            cx.notify();
            return;
        };
        self.adding = true;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let result = smol::unblock(move || gh::resolve(&owner, &name)).await;
            let _ = this.update_in(cx, |s, w, cx| {
                s.adding = false;
                match result {
                    Ok((owner, name)) => {
                        s.add_url_open = false;
                        s.add_repo(
                            Repo {
                                owner,
                                name,
                                path: None,
                            },
                            w,
                            cx,
                        );
                    }
                    Err(e) => s.notice = Notice::warn(e.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    // ── Preferences ─────────────────────────────────────────────────────────

    fn update_settings(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Settings)) {
        let mut next = settings::get(cx).clone();
        f(&mut next);
        let next = next.sanitized();
        if let Err(e) = next.save() {
            self.notice = Notice::warn(format!("Could not save settings: {e}"));
        }
        let commits_changed = next.commit_limit != settings::get(cx).commit_limit;
        gh::set_gh_path(&next.gh_path);
        cx.set_global(next);
        if commits_changed {
            for d in self.data.values_mut() {
                d.commits.clear();
            }
        }
        crate::theme::refresh(cx);
        cx.notify();
    }

    // ── Popovers (as in CloudWatch v2) ──────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    fn open_popover(
        &mut self,
        anchor: &'static str,
        fallback: Point<Pixels>,
        search: Option<&str>,
        width: f32,
        build: popover::Builder,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let menu = cx.new(|cx| Menu::new(build, search, px(width), window, cx));
        menu.update(cx, |m, cx| m.focus(window, cx));
        cx.subscribe_in(&menu, window, |s, _, _: &DismissEvent, window, cx| {
            s.popup = None;
            s.dismissed = s.popup_anchor.take().map(|a| (a, Instant::now()));
            if !s.prefs_open {
                s.focus.focus(window);
            }
            cx.notify();
        })
        .detach();
        self.popup_anchor = Some(anchor);
        self.popup_position = self
            .anchors
            .borrow()
            .get(anchor)
            .map(|b| b.bottom_left() + point(px(0.), px(4.)))
            .unwrap_or(fallback);
        self.popup = Some(menu);
        cx.notify();
    }

    // ── Keyboard actions ────────────────────────────────────────────────────

    fn toggle_sidebar(&mut self, _: &ToggleSidebar, _: &mut Window, cx: &mut Context<Self>) {
        self.sidebar = !self.sidebar;
        cx.notify();
    }
    fn toggle_brief(&mut self, _: &ToggleBrief, _: &mut Window, cx: &mut Context<Self>) {
        self.brief = !self.brief;
        cx.notify();
    }
    fn close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        if self.prefs_open {
            self.prefs_open = false;
        } else if self.add_url_open {
            self.add_url_open = false;
        } else if let Some(i) = self.active {
            self.close(i, cx);
        } else {
            window.remove_window();
        }
        cx.notify();
    }
    fn duplicate_tab(&mut self, _: &DuplicateTab, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(i) = self.active {
            self.duplicate(i, window, cx);
        }
    }
    fn cycle(&mut self, step: isize, cx: &mut Context<Self>) {
        if let Some(i) = self.active
            && !self.tabs.is_empty()
        {
            self.active =
                Some((i as isize + step).rem_euclid(self.tabs.len() as isize) as usize);
            cx.notify();
        }
    }
    fn open_prefs(&mut self, _: &OpenPreferences, window: &mut Window, cx: &mut Context<Self>) {
        self.popup = None;
        self.prefs_open = true;
        self.prefs_search.update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
    }
    /// Step back out of a detail page: file → folder, PR/run/commit → list.
    fn back(&mut self, _: &Back, _: &mut Window, cx: &mut Context<Self>) {
        let Some(t) = self.current_mut() else { return };
        match t.view {
            View::Code if t.file.is_some() => t.file = None,
            View::Code if !t.path.is_empty() => {
                t.path = t
                    .path
                    .rsplit_once('/')
                    .map(|(p, _)| p.to_string())
                    .unwrap_or_default()
            }
            View::Commits => t.commit = None,
            View::Pulls => t.pr = None,
            View::Actions if t.log_open => t.log_open = false,
            View::Actions => t.run = None,
            _ => {}
        }
        cx.notify();
    }
    fn submit(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        if self.add_url_open {
            self.submit_url(window, cx);
            return;
        }
        let Some(t) = self.current() else { return };
        if let (View::Pulls, Some(n)) = (t.view, t.pr) {
            let body = t.composer.read(cx).value().trim().to_string();
            if !body.is_empty() {
                self.pr_action(n, gh::PrAction::Comment(body), cx);
            }
        }
    }
}

fn load_blob(slug: &str, path: &str, branch: &str) -> anyhow::Result<Arc<BlobView>> {
    Ok(Arc::new(match gh::blob(slug, path, branch)? {
        gh::Blob::Text(t) => BlobView::Text(
            t.lines()
                .map(|l| SharedString::from(l.replace('\t', "    ")))
                .collect(),
        ),
        gh::Blob::Binary(n) => BlobView::Binary(n),
        gh::Blob::TooLarge(n) => BlobView::TooLarge(n),
    }))
}
fn load_graph(
    slug: &str,
    path: Option<&std::path::Path>,
    reference: Option<&str>,
    limit: u32,
) -> anyhow::Result<Arc<CommitGraph>> {
    let mut commits = match path {
        // A branch from GitHub may exist only as a remote-tracking ref locally.
        Some(p) => gh::local_log(p, reference, limit).or_else(|e| match reference {
            Some(r) => gh::local_log(p, Some(&format!("origin/{r}")), limit).map_err(|_| e),
            None => Err(e),
        })?,
        None => gh::remote_log(slug, reference.unwrap_or("HEAD"), limit)?,
    };
    if path.is_none() {
        // The REST history has no decorations; mark branch tips we know about.
        if let Ok(branches) = gh::branches(slug) {
            let tips = graph::tips(
                &branches
                    .into_iter()
                    .map(|b| (b.name, b.commit.sha))
                    .collect::<Vec<_>>(),
            );
            for c in &mut commits {
                if let Some(names) = tips.get(&c.sha) {
                    c.refs = names
                        .iter()
                        .map(|n| gh::Ref {
                            name: n.clone(),
                            kind: if Some(n.as_str()) == reference {
                                gh::RefKind::Head
                            } else {
                                gh::RefKind::Branch
                            },
                        })
                        .collect();
                }
            }
        }
    }
    let graph = graph::layout(commits.iter().map(|c| (c.sha.as_str(), c.parents.as_slice())));
    Ok(Arc::new(CommitGraph { commits, graph }))
}
fn load_log(lines: Vec<gh::LogLine>) -> LogView {
    let truncated = lines.len() >= gh::LOG_LIMIT;
    let mut rows = Vec::with_capacity(lines.len() + 32);
    let mut step: Option<String> = None;
    for l in lines {
        if step.as_deref() != Some(&l.step) {
            rows.push((true, SharedString::from(l.step.clone())));
            step = Some(l.step);
        }
        rows.push((false, SharedString::from(l.text)));
    }
    LogView { rows, truncated }
}
