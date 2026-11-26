//! Per-tab navigation history, like a browser's: each tab keeps the places it
//! has been (a folder, a file, a pull request, a run…) on a back and a forward
//! stack.
//!
//! Views change a tab's location by writing its fields directly, so nothing has
//! to announce a move: every frame the workspace calls [`History::record`] with
//! where the tab is now, and a change from the last place recorded becomes a
//! step back. Going back or forward updates the last place first, so applying
//! it isn't recorded again.
use super::{PrPage, Tab};
use crate::{gh::PrFilter, settings::View, util};

/// Where a tab is: everything that decides what it shows.
#[derive(Default, Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Place {
    pub view: View,
    pub branch: Option<String>,
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
}

impl Place {
    pub fn of(t: &Tab) -> Self {
        Self {
            view: t.view,
            branch: t.branch.clone(),
            graph_ref: t.graph_ref.clone(),
            path: t.path.clone(),
            file: t.file.clone(),
            commit: t.commit.clone(),
            pr_filter: t.pr_filter,
            pr: t.pr,
            pr_page: t.pr_page,
            workflow: t.workflow,
            run: t.run,
            job: t.job,
            log_open: t.log_open,
        }
    }
    /// Move `t` here. A half-written line comment belongs to the page left behind.
    pub fn apply(&self, t: &mut Tab) {
        let p = self.clone();
        t.view = p.view;
        t.branch = p.branch;
        t.graph_ref = p.graph_ref;
        t.path = p.path;
        t.file = p.file;
        t.commit = p.commit;
        t.pr_filter = p.pr_filter;
        t.pr = p.pr;
        t.pr_page = p.pr_page;
        t.workflow = p.workflow;
        t.run = p.run;
        t.job = p.job;
        t.log_open = p.log_open;
        t.line_target = None;
        t.line_start = None;
    }
    /// A short description for the history menus.
    pub fn label(&self) -> String {
        let name = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
        match self.view {
            View::Code => match (&self.file, self.path.as_str()) {
                (Some(f), _) => name(f),
                (None, "") => "Code".into(),
                (None, p) => format!("{}/", name(p)),
            },
            View::Commits => match &self.commit {
                Some(sha) => format!("Commit {}", util::short_sha(sha)),
                None => "Commits".into(),
            },
            View::Pulls => match self.pr {
                Some(n) => format!("Pull request #{n}"),
                None => "Pull requests".into(),
            },
            View::Actions => match (self.run, self.log_open) {
                (Some(r), true) => format!("Run {r} · log"),
                (Some(r), false) => format!("Run {r}"),
                (None, _) => "Actions".into(),
            },
            View::Releases => "Releases".into(),
        }
    }
}

#[derive(Default)]
pub struct History {
    /// The last place recorded: where the tab was on the previous frame.
    current: Option<Place>,
    /// Older places, most recent last.
    back: Vec<Place>,
    /// Places stepped back from, most recent last.
    forward: Vec<Place>,
}

impl History {
    /// How far back a tab remembers.
    const LIMIT: usize = 100;

    /// Note where the tab is now. Moving somewhere new starts a fresh forward path.
    pub fn record(&mut self, now: Place) {
        match self.current.take() {
            Some(prev) if prev != now => {
                self.back.push(prev);
                if self.back.len() > Self::LIMIT {
                    self.back.remove(0);
                }
                self.forward.clear();
            }
            _ => {}
        }
        self.current = Some(now);
    }
    /// Step `steps` places back (1 is the previous place); returns where to go.
    pub fn back(&mut self, steps: usize) -> Option<Place> {
        Self::step(&mut self.back, &mut self.forward, &mut self.current, steps)
    }
    pub fn forward(&mut self, steps: usize) -> Option<Place> {
        Self::step(&mut self.forward, &mut self.back, &mut self.current, steps)
    }
    fn step(from: &mut Vec<Place>, to: &mut Vec<Place>, current: &mut Option<Place>, steps: usize) -> Option<Place> {
        if steps == 0 || steps > from.len() {
            return None;
        }
        for _ in 0..steps {
            let next = from.pop()?;
            if let Some(cur) = current.replace(next) {
                to.push(cur);
            }
        }
        current.clone()
    }
    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }
    pub fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }
    /// The places behind, nearest first.
    pub fn back_list(&self) -> impl Iterator<Item = &Place> {
        self.back.iter().rev()
    }
    pub fn forward_list(&self) -> impl Iterator<Item = &Place> {
        self.forward.iter().rev()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(path: &str) -> Place {
        Place { path: path.into(), ..Default::default() }
    }

    #[test]
    fn records_moves_and_steps_both_ways() {
        let mut h = History::default();
        h.record(at(""));
        h.record(at(""));
        assert!(!h.can_back());
        h.record(at("src"));
        h.record(at("src/a"));
        assert_eq!(h.back(1), Some(at("src")));
        // Applying the place re-records it: no new step.
        h.record(at("src"));
        assert!(h.can_forward());
        assert_eq!(h.forward(1), Some(at("src/a")));
        assert_eq!(h.back(2), Some(at("")));
        assert!(!h.can_back());
        assert_eq!(h.forward_list().count(), 2);
    }

    #[test]
    fn a_new_move_drops_the_forward_path() {
        let mut h = History::default();
        h.record(at("a"));
        h.record(at("b"));
        h.back(1);
        h.record(at("a"));
        h.record(at("c"));
        assert!(!h.can_forward());
        assert_eq!(h.back_list().cloned().collect::<Vec<_>>(), vec![at("a")]);
        assert_eq!(h.back(5), None);
    }
}
