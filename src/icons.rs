//! App-specific glyphs (Lucide, ISC license) layered over GPUI Component's icon set.
use gpui::{AssetSource, Result, SharedString};
use gpui_component::IconNamed;
use std::borrow::Cow;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Glyph {
    Branch,
    Commit,
    PullRequest,
    PullRequestDraft,
    PullRequestClosed,
    Merge,
    Fork,
    Tag,
    Package,
    Code,
    History,
    Play,
    CirclePlay,
    Refresh,
    Message,
    CircleDot,
    CircleDashed,
    CircleSlash,
    Clock,
    FolderGit,
    Link,
    Scale,
    RotateCcw,
    Stop,
    Download,
    Fetch,
    Terminal,
    FileDiff,
    ListChecks,
    Workflow,
    Lock,
    Compare,
    Graph,
    Activity,
}

const PREFIX: &str = "app-icons/";

impl Glyph {
    fn file(self) -> &'static str {
        match self {
            Self::Branch => "git-branch.svg",
            Self::Commit => "git-commit.svg",
            Self::PullRequest => "git-pull-request.svg",
            Self::PullRequestDraft => "git-pull-request-draft.svg",
            Self::PullRequestClosed => "git-pull-request-closed.svg",
            Self::Merge => "git-merge.svg",
            Self::Fork => "git-fork.svg",
            Self::Tag => "tag.svg",
            Self::Package => "package.svg",
            Self::Code => "code.svg",
            Self::History => "history.svg",
            Self::Play => "play.svg",
            Self::CirclePlay => "circle-play.svg",
            Self::Refresh => "refresh.svg",
            Self::Message => "message.svg",
            Self::CircleDot => "circle-dot.svg",
            Self::CircleDashed => "circle-dashed.svg",
            Self::CircleSlash => "circle-slash.svg",
            Self::Clock => "clock.svg",
            Self::FolderGit => "folder-git.svg",
            Self::Link => "link.svg",
            Self::Scale => "scale.svg",
            Self::RotateCcw => "rotate-ccw.svg",
            Self::Stop => "stop.svg",
            Self::Download => "download.svg",
            Self::Fetch => "fetch.svg",
            Self::Terminal => "terminal.svg",
            Self::FileDiff => "file-diff.svg",
            Self::ListChecks => "list-checks.svg",
            Self::Workflow => "workflow.svg",
            Self::Lock => "lock.svg",
            Self::Compare => "git-compare.svg",
            Self::Graph => "git-graph.svg",
            Self::Activity => "activity.svg",
        }
    }
}

impl IconNamed for Glyph {
    fn path(self) -> SharedString {
        format!("{PREFIX}{}", self.file()).into()
    }
}

fn svg(file: &str) -> Option<&'static str> {
    Some(match file {
        "git-branch.svg" => {
            r#"<line x1="6" x2="6" y1="3" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/>"#
        }
        "git-commit.svg" => {
            r#"<circle cx="12" cy="12" r="3"/><line x1="3" x2="9" y1="12" y2="12"/><line x1="15" x2="21" y1="12" y2="12"/>"#
        }
        "git-pull-request.svg" => {
            r#"<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M13 6h3a2 2 0 0 1 2 2v7"/><line x1="6" x2="6" y1="9" y2="21"/>"#
        }
        "git-pull-request-draft.svg" => {
            r#"<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M18 6V5"/><path d="M18 11v-1"/><line x1="6" x2="6" y1="9" y2="21"/>"#
        }
        "git-pull-request-closed.svg" => {
            r#"<circle cx="6" cy="6" r="3"/><path d="M6 9v12"/><path d="m21 3-6 6"/><path d="m21 9-6-6"/><path d="M18 11.5V15"/><circle cx="18" cy="18" r="3"/>"#
        }
        "git-merge.svg" => {
            r#"<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M6 21V9a9 9 0 0 0 9 9"/>"#
        }
        "git-fork.svg" => {
            r#"<circle cx="12" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><circle cx="18" cy="6" r="3"/><path d="M18 9v2c0 .6-.4 1-1 1H7c-.6 0-1-.4-1-1V9"/><path d="M12 12v3"/>"#
        }
        "tag.svg" => {
            r#"<path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"/><circle cx="7.5" cy="7.5" r="1"/>"#
        }
        "package.svg" => {
            r#"<path d="m7.5 4.27 9 5.15"/><path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/>"#
        }
        "code.svg" => r#"<path d="m18 16 4-4-4-4"/><path d="m6 8-4 4 4 4"/><path d="m14.5 4-5 16"/>"#,
        "history.svg" => {
            r#"<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l4 2"/>"#
        }
        "play.svg" => r#"<polygon points="6 3 20 12 6 21 6 3"/>"#,
        "circle-play.svg" => {
            r#"<circle cx="12" cy="12" r="10"/><polygon points="10 8 16 12 10 16 10 8"/>"#
        }
        "users.svg" => {
            r#"<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>"#
        }
        "refresh.svg" => {
            r#"<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>"#
        }
        "message.svg" => {
            r#"<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>"#
        }
        "circle-dot.svg" => r#"<circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="1"/>"#,
        "circle-dashed.svg" => {
            r#"<path d="M10.1 2.182a10 10 0 0 1 3.8 0"/><path d="M13.9 21.818a10 10 0 0 1-3.8 0"/><path d="M17.609 3.721a10 10 0 0 1 2.69 2.7"/><path d="M2.182 13.9a10 10 0 0 1 0-3.8"/><path d="M20.279 17.609a10 10 0 0 1-2.7 2.69"/><path d="M21.818 10.1a10 10 0 0 1 0 3.8"/><path d="M3.721 6.391a10 10 0 0 1 2.7-2.69"/><path d="M6.391 20.279a10 10 0 0 1-2.69-2.7"/>"#
        }
        "circle-slash.svg" => {
            r#"<circle cx="12" cy="12" r="10"/><line x1="9" x2="15" y1="15" y2="9"/>"#
        }
        "clock.svg" => r#"<circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>"#,
        "folder-git.svg" => {
            r#"<path d="M9 20H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H20a2 2 0 0 1 2 2v5"/><circle cx="13" cy="12" r="2"/><path d="M18 19c-2.8 0-5-2.2-5-5v8"/><circle cx="20" cy="19" r="2"/>"#
        }
        "link.svg" => {
            r#"<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>"#
        }
        "scale.svg" => {
            r#"<path d="m16 16 3-8 3 8c-.87.65-1.92 1-3 1s-2.13-.35-3-1Z"/><path d="m2 16 3-8 3 8c-.87.65-1.92 1-3 1s-2.13-.35-3-1Z"/><path d="M7 21h10"/><path d="M12 3v18"/><path d="M3 7h2c2 0 5-1 7-2 2 1 5 2 7 2h2"/>"#
        }
        "rotate-ccw.svg" => {
            r#"<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/>"#
        }
        "stop.svg" => r#"<rect width="14" height="14" x="5" y="5" rx="2"/>"#,
        "download.svg" => {
            r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>"#
        }
        "fetch.svg" => r#"<path d="M12 17V3"/><path d="m6 11 6 6 6-6"/><path d="M19 21H5"/>"#,
        "terminal.svg" => {
            r#"<polyline points="4 17 10 11 4 5"/><line x1="12" x2="20" y1="19" y2="19"/>"#
        }
        "file-diff.svg" => {
            r#"<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M9 10h6"/><path d="M12 13V7"/><path d="M9 17h6"/>"#
        }
        "list-checks.svg" => {
            r#"<path d="m3 17 2 2 4-4"/><path d="m3 7 2 2 4-4"/><path d="M13 6h8"/><path d="M13 12h8"/><path d="M13 18h8"/>"#
        }
        "workflow.svg" => {
            r#"<rect width="8" height="8" x="3" y="3" rx="2"/><path d="M7 11v4a2 2 0 0 0 2 2h4"/><rect width="8" height="8" x="13" y="13" rx="2"/>"#
        }
        "lock.svg" => {
            r#"<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>"#
        }
        "archive.svg" => {
            r#"<rect width="20" height="5" x="2" y="3" rx="1"/><path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"/><path d="M10 12h4"/>"#
        }
        "git-compare.svg" => {
            r#"<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M13 6h3a2 2 0 0 1 2 2v7"/><path d="M11 18H8a2 2 0 0 1-2-2V9"/>"#
        }
        "git-graph.svg" => {
            r#"<circle cx="5" cy="6" r="3"/><path d="M5 9v6"/><circle cx="5" cy="18" r="3"/><path d="M12 3v18"/><circle cx="19" cy="6" r="3"/><path d="M16 15.7A9 9 0 0 0 19 9"/>"#
        }
        "activity.svg" => {
            r#"<path d="M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2"/>"#
        }
        "hash.svg" => {
            r#"<line x1="4" x2="20" y1="9" y2="9"/><line x1="4" x2="20" y1="15" y2="15"/><line x1="10" x2="8" y1="3" y2="21"/><line x1="16" x2="14" y1="3" y2="21"/>"#
        }
        "scroll-text.svg" => {
            r#"<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M16 13H8"/><path d="M16 17H8"/><path d="M10 9H8"/>"#
        }
        _ => return None,
    })
}

/// Serves app glyphs, then falls back to GPUI Component's embedded icons.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(body) = path.strip_prefix(PREFIX).and_then(svg) {
            return Ok(Some(Cow::Owned(
                format!(
                    r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"#
                )
                .into_bytes(),
            )));
        }
        gpui_component_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        gpui_component_assets::Assets.list(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_glyph_resolves() {
        use Glyph::*;
        for g in [
            Branch, Commit, PullRequest, PullRequestDraft, PullRequestClosed, Merge, Fork, Tag,
            Package, Code, History, Play, CirclePlay, Refresh, Message, CircleDot,
            CircleDashed, CircleSlash, Clock, FolderGit, Link, Scale, RotateCcw, Stop, Download,
            Fetch, Terminal, FileDiff, ListChecks, Workflow, Lock, Compare, Graph,
            Activity,
        ] {
            assert!(Assets.load(&g.path()).unwrap().is_some(), "{g:?}");
        }
        assert!(Assets.load("icons/search.svg").unwrap().is_some());
    }
}
