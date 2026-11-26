//! A run's jobs as a graph, the way GitHub draws a run's summary: one column per
//! stage, each job waiting on the ones it `needs:` to its left, joined by curves.
//!
//! GitHub's jobs API doesn't say what a job needs, so the workflow file (read at
//! the run's commit) supplies it. API jobs are matched to the file's jobs by
//! name; a matrix job's instances (`test (ubuntu, 1.80)`) and a reusable
//! workflow's jobs (`call / build`) group under one node. Jobs that can't be
//! matched still show, in the first column.
use super::ui::*;
use super::*;
use crate::util;
use gpui::prelude::FluentBuilder;
use gpui_component::{IconName, Sizable, tooltip::Tooltip};

/// A box on the graph: one planned job and the run's jobs for it.
#[derive(Debug, PartialEq)]
pub struct Node {
    pub title: String,
    /// Indexes into the run's jobs.
    pub jobs: Vec<usize>,
    pub level: usize,
    /// Instances of a matrix or a reusable workflow: drawn as a titled group.
    pub group: bool,
}
#[derive(Debug, Default, PartialEq)]
pub struct Layout {
    pub nodes: Vec<Node>,
    /// Node `from` must finish before node `to`.
    pub edges: Vec<(usize, usize)>,
}

/// What a planned job's runs are called, up to any `${{ }}` expression: the
/// whole name when it has none, otherwise the fixed text before it.
fn label(p: &gh::PlannedJob) -> (String, bool) {
    let name = p.name.clone().unwrap_or_else(|| p.id.clone());
    match name.find("${{") {
        Some(at) => (name[..at].trim().to_string(), false),
        None => (name, true),
    }
}
/// Whether the run's job `name` belongs to planned job `p`.
fn belongs(p: &gh::PlannedJob, name: &str, exact_only: bool) -> bool {
    let (label, whole) = label(p);
    if whole && (name == label || name == p.id) {
        return true;
    }
    if exact_only || label.is_empty() {
        return false;
    }
    if !whole {
        return name.starts_with(&label);
    }
    name.starts_with(&format!("{label} (")) || name.starts_with(&format!("{label} / "))
}

pub fn layout(jobs: &[gh::Job], plan: &[gh::PlannedJob]) -> Layout {
    // Each job goes to the first planned job it matches, exact names first.
    let mut owner: Vec<Option<usize>> = vec![None; jobs.len()];
    for exact in [true, false] {
        for (j, job) in jobs.iter().enumerate() {
            if owner[j].is_none() {
                owner[j] = plan.iter().position(|p| belongs(p, &job.name, exact));
            }
        }
    }
    // Column: one past the deepest job waited on. Cycles (invalid YAML) stop at 0.
    fn depth(i: usize, plan: &[gh::PlannedJob], memo: &mut Vec<Option<usize>>, seen: &mut Vec<bool>) -> usize {
        if let Some(d) = memo[i] {
            return d;
        }
        if seen[i] {
            return 0;
        }
        seen[i] = true;
        let d = plan[i]
            .needs
            .iter()
            .filter_map(|n| plan.iter().position(|p| &p.id == n))
            .map(|n| depth(n, plan, memo, seen) + 1)
            .max()
            .unwrap_or(0);
        memo[i] = Some(d);
        d
    }
    let mut memo = vec![None; plan.len()];
    let mut seen = vec![false; plan.len()];
    let mut out = Layout::default();
    let mut node_of_plan = vec![None; plan.len()];
    for (i, p) in plan.iter().enumerate() {
        let mine: Vec<usize> = (0..jobs.len()).filter(|&j| owner[j] == Some(i)).collect();
        if mine.is_empty() {
            continue;
        }
        let (label, _) = label(p);
        let group = mine.len() > 1 || jobs[mine[0]].name != label && jobs[mine[0]].name != p.id;
        let title = if group {
            if label.is_empty() { p.id.clone() } else { label }
        } else {
            jobs[mine[0]].name.clone()
        };
        node_of_plan[i] = Some(out.nodes.len());
        out.nodes.push(Node {
            title,
            jobs: mine,
            level: depth(i, plan, &mut memo, &mut seen),
            group,
        });
    }
    for (j, job) in jobs.iter().enumerate() {
        if owner[j].is_none() {
            out.nodes.push(Node { title: job.name.clone(), jobs: vec![j], level: 0, group: false });
        }
    }
    // A job may need one that never ran here; its column closes the gap.
    for (i, p) in plan.iter().enumerate() {
        let Some(to) = node_of_plan[i] else { continue };
        for n in &p.needs {
            if let Some(from) = plan.iter().position(|q| &q.id == n).and_then(|f| node_of_plan[f]) {
                out.edges.push((from, to));
            }
        }
    }
    let mut levels: Vec<usize> = out.nodes.iter().map(|n| n.level).collect();
    levels.sort_unstable();
    levels.dedup();
    for n in &mut out.nodes {
        n.level = levels.iter().position(|l| *l == n.level).unwrap_or(0);
    }
    out
}

const PAD: f32 = 24.;
const NODE_W: f32 = 250.;
const COL_GAP: f32 = 64.;
const NODE_GAP: f32 = 16.;
const JOB_H: f32 = 34.;
const GROUP_HEAD: f32 = 32.;
/// The radius where a connector turns.
const CORNER: f32 = 8.;
/// How long after a drag a mouse-up is still part of it, not a click.
const DRAG_GRACE: std::time::Duration = std::time::Duration::from_millis(150);

/// Trace a connector from `from` to `to` the way circuit diagrams route wires:
/// across, a rounded turn, along the column gap, another turn, then across.
fn elbow(b: &mut PathBuilder, from: Point<Pixels>, to: Point<Pixels>, radius: Pixels) {
    b.move_to(from);
    let dy = to.y - from.y;
    if dy.abs() < px(0.5) {
        b.line_to(to);
        return;
    }
    let mid = from.x + (to.x - from.x) / 2.;
    let down = if dy > px(0.) { 1. } else { -1. };
    let r = radius.min(dy.abs() / 2.).min((mid - from.x).max(px(0.)));
    b.line_to(point(mid - r, from.y));
    b.curve_to(point(mid, from.y + r * down), point(mid, from.y));
    b.line_to(point(mid, to.y - r * down));
    b.curve_to(point(mid + r, to.y), point(mid, to.y));
    b.line_to(to);
}

/// A drag on the graph in progress: where it started, and the scroll then.
#[derive(Clone, Copy)]
pub struct Pan {
    run: u64,
    start: Point<Pixels>,
    offset: Point<Pixels>,
}

/// How much of the graph stays on screen however far it's pushed away.
const KEEP_VISIBLE: f32 = 80.;
/// Limit a pan so the graph can't be lost off the edge of the view.
fn clamp_pan(offset: Point<Pixels>, content: Size<Pixels>, view: Size<Pixels>) -> Point<Pixels> {
    let keep = px(KEEP_VISIBLE);
    let axis = |v: Pixels, content: Pixels, view: Pixels| v.clamp((keep - content).min(px(0.)), (view - keep).max(px(0.)));
    point(axis(offset.x, content.width, view.width), axis(offset.y, content.height, view.height))
}

fn node_height(n: &Node) -> f32 {
    if n.group { GROUP_HEAD + n.jobs.len() as f32 * JOB_H + 4. } else { JOB_H + 2. }
}

impl Workspace {
    fn drag_graph(&mut self, e: &MouseMoveEvent, cx: &mut Context<Self>) {
        let Some(pan) = self.pan else { return };
        if e.pressed_button != Some(MouseButton::Left) {
            self.end_pan(cx);
            return;
        }
        let delta = e.position - pan.start;
        // Small jitter while clicking a job isn't a drag.
        if self.panned.is_none() && delta.x.abs() < px(3.) && delta.y.abs() < px(3.) {
            return;
        }
        self.move_graph(pan.run, pan.offset + delta, cx);
        self.panned = Some(std::time::Instant::now());
    }
    fn move_graph(&mut self, run: u64, to: Point<Pixels>, cx: &mut Context<Self>) {
        let (content, view) = self.graph_extent.get();
        self.graph_offsets.insert(run, clamp_pan(to, content, view));
        cx.notify();
    }
    fn end_pan(&mut self, cx: &mut Context<Self>) {
        if self.pan.take().is_some() {
            cx.notify();
        }
    }

    /// The run's summary: every job on the graph. Clicking a job opens it.
    pub(super) fn run_graph(&self, t: &Tab, run: u64, jobs: &[gh::Job], cx: &mut Context<Self>) -> AnyElement {
        let colors = palette(cx);
        let plan = self
            .data(&t.repo)
            .and_then(|d| d.plan.get(&run))
            .and_then(|l| l.data.clone())
            .unwrap_or_default();
        let g = layout(jobs, &plan);
        // Place the boxes: columns left to right, top-aligned like GitHub's.
        let mut tops = vec![PAD; g.nodes.iter().map(|n| n.level + 1).max().unwrap_or(1)];
        let mut boxes: Vec<Bounds<Pixels>> = vec![];
        for n in &g.nodes {
            let h = node_height(n);
            let x = PAD + n.level as f32 * (NODE_W + COL_GAP);
            boxes.push(Bounds::new(point(px(x), px(tops[n.level])), size(px(NODE_W), px(h))));
            tops[n.level] += h + NODE_GAP;
        }
        let width = PAD * 2. + tops.len() as f32 * NODE_W + (tops.len().saturating_sub(1)) as f32 * COL_GAP;
        let height = tops.iter().cloned().fold(PAD, f32::max) + PAD;
        // Curves leave a box's first row and arrive at the next one's.
        let links: Vec<(Point<Pixels>, Point<Pixels>)> = g
            .edges
            .iter()
            .map(|&(a, b)| {
                let (a, b) = (boxes[a], boxes[b]);
                let mid = px(JOB_H / 2. + 1.);
                (point(a.right(), a.top() + mid), point(b.left(), b.top() + mid))
            })
            .collect();
        let line = colors.quiet;
        let surface = colors.background;
        let edges = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let o = bounds.origin;
                for (from, to) in &links {
                    let (from, to) = (o + *from, o + *to);
                    let mut b = PathBuilder::stroke(px(1.5));
                    elbow(&mut b, from, to, px(CORNER));
                    if let Ok(path) = b.build() {
                        window.paint_path(path, rgb(line));
                    }
                    // Ringed dots where a line meets a box, as GitHub draws them.
                    for p in [from, to] {
                        let r = px(4.);
                        window.paint_quad(
                            fill(Bounds::new(point(p.x - r, p.y - r), size(r * 2., r * 2.)), rgb(surface))
                                .corner_radii(r)
                                .border_widths(px(1.5))
                                .border_color(rgb(line)),
                        );
                    }
                }
            },
        )
        .absolute()
        .inset_0();
        let selected = t.job;
        let job_row = |idx: usize, short: bool, cx: &mut Context<Self>| {
            let j = &jobs[idx];
            let jid = j.database_id;
            let name = if short {
                // In a group, the matrix values say which instance this is.
                j.name
                    .rfind(" (")
                    .map(|at| j.name[at + 2..].trim_end_matches(')').to_string())
                    .or_else(|| j.name.split_once(" / ").map(|(_, rest)| rest.to_string()))
                    .unwrap_or_else(|| j.name.clone())
            } else {
                j.name.clone()
            };
            let tip = j.name.clone();
            row()
                .id(("graph-job", jid))
                .h(px(JOB_H))
                .px_3()
                .gap_2()
                .rounded(px(6.))
                .cursor_pointer()
                .when(selected == Some(jid), |s| s.bg(rgb(colors.selected)))
                .hover(|s| s.bg(rgb(colors.hover)))
                .child(run_icon(colors, &j.status, &j.conclusion))
                .child(div().flex_1().min_w_0().truncate().child(name))
                .when_some(util::span(j.started_at.as_deref(), j.completed_at.as_deref()), |s, d| {
                    s.child(div().flex_shrink_0().text_size(px(12.)).text_color(rgb(colors.muted)).child(d))
                })
                .tooltip(move |w, cx| Tooltip::new(tip.clone()).build(w, cx))
                .on_click(cx.listener(move |s, _, _, cx| {
                    if s.panned.is_some_and(|at| at.elapsed() < DRAG_GRACE) {
                        return;
                    }
                    if let Some(t) = s.current_mut() {
                        t.job = Some(jid);
                        t.log_open = false;
                    }
                    cx.notify();
                }))
        };
        let offset = self.graph_offsets.get(&run).copied().unwrap_or_default();
        let content = size(px(width), px(height));
        let mut canvas_area = div().absolute().left(offset.x).top(offset.y).w(px(width)).h(px(height)).child(edges);
        for (i, n) in g.nodes.iter().enumerate() {
            let b = boxes[i];
            let mut card = col()
                .absolute()
                .left(b.left())
                .top(b.top())
                .w(b.size.width)
                .p(px(1.))
                .rounded(px(8.))
                .border_1()
                .border_color(rgb(colors.border))
                .bg(rgb(colors.chrome))
                .shadow_sm();
            if n.group {
                let done = n.jobs.iter().filter(|&&j| jobs[j].status.eq_ignore_ascii_case("completed")).count();
                card = card.child(
                    row()
                        .h(px(GROUP_HEAD))
                        .px_3()
                        .gap_2()
                        .border_b_1()
                        .border_color(rgb(colors.row_border))
                        .text_size(px(12.))
                        .child(div().flex_1().min_w_0().truncate().font_weight(FontWeight::MEDIUM).child(n.title.clone()))
                        .child(div().text_color(rgb(colors.muted)).child(format!("{done}/{}", n.jobs.len()))),
                );
                for &j in &n.jobs {
                    card = card.child(job_row(j, true, cx));
                }
            } else {
                card = card.child(job_row(n.jobs[0], false, cx));
            }
            canvas_area = canvas_area.child(card);
        }
        let summary = {
            let done = jobs.iter().filter(|j| j.status.eq_ignore_ascii_case("completed")).count();
            let failed = jobs.iter().filter(|j| matches!(j.conclusion.as_str(), "failure" | "timed_out" | "cancelled")).count();
            row()
                .h(px(BAR))
                .flex_shrink_0()
                .px_3()
                .gap_2()
                .border_b_1()
                .border_color(rgb(colors.row_border))
                .text_size(px(12.5))
                .text_color(rgb(colors.muted))
                .child(div().font_weight(FontWeight::MEDIUM).text_color(rgb(colors.text)).child("Summary"))
                .child(format!("{done} of {} jobs finished", jobs.len()))
                .when(failed > 0, |s| s.child(div().text_color(rgb(colors.red)).child(format!("· {failed} failed"))))
                .child(div().flex_1())
                .when(plan.is_empty() && !jobs.is_empty(), |s| {
                    s.child(div().text_color(rgb(colors.quiet)).child("Job dependencies unavailable"))
                })
                .when(offset != Point::default(), |s| {
                    s.child(
                        icon_button("graph-recenter", IconName::Maximize, "Reset view")
                            .xsmall()
                            .on_click(cx.listener(move |s, _, _, cx| {
                                s.graph_offsets.remove(&run);
                                cx.notify();
                            })),
                    )
                })
        };
        let extent = self.graph_extent.clone();
        // Measures the view, so pans know how far the graph may go.
        let measure = canvas(move |bounds, _, _| extent.set((content, bounds.size)), |_, _, _, _| {})
            .absolute()
            .inset_0();
        col()
            .size_full()
            .child(summary)
            .child(
                // A free canvas: drag or scroll in any direction, even when it all fits.
                div()
                    .id(("run-graph", t.id))
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .bg(rgb(colors.panel))
                    .cursor(if self.pan.is_some() { CursorStyle::ClosedHand } else { CursorStyle::OpenHand })
                    .child(measure)
                    .child(canvas_area)
                    .on_mouse_down(MouseButton::Left, cx.listener(move |s, e: &MouseDownEvent, _, _| {
                        s.pan = Some(Pan { run, start: e.position, offset });
                        s.panned = None;
                    }))
                    .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| s.drag_graph(e, cx)))
                    .on_mouse_up(MouseButton::Left, cx.listener(|s, _, _, cx| s.end_pan(cx)))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(|s, _, _, cx| s.end_pan(cx)))
                    .on_scroll_wheel(cx.listener(move |s, e: &ScrollWheelEvent, _, cx| {
                        cx.stop_propagation();
                        let now = s.graph_offsets.get(&run).copied().unwrap_or_default();
                        s.move_graph(run, now + e.delta.pixel_delta(px(20.)), cx);
                    })),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{gh, layout};

    fn job(name: &str) -> gh::Job {
        gh::Job {
            database_id: 0,
            name: name.into(),
            status: "completed".into(),
            conclusion: "success".into(),
            started_at: None,
            completed_at: None,
            steps: vec![],
        }
    }

    #[test]
    fn reads_needs_from_the_workflow() {
        let plan = gh::planned_jobs(
            "on: push\njobs:\n  lint:\n    runs-on: x\n  test:\n    name: Test\n    needs: lint\n    strategy: {}\n  deploy:\n    needs: [lint, test]\n",
        );
        assert_eq!(plan.len(), 3);
        assert_eq!(plan[1].name.as_deref(), Some("Test"));
        assert_eq!(plan[2].needs, vec!["lint", "test"]);
    }

    #[test]
    fn lays_out_stages_and_groups_matrices() {
        let plan = gh::planned_jobs(
            "jobs:\n  lint: {}\n  test:\n    name: Test ${{ matrix.os }}\n    needs: lint\n  deploy:\n    needs: test\n",
        );
        let jobs = [job("lint"), job("Test ubuntu"), job("Test macos"), job("deploy"), job("stray")];
        let g = layout(&jobs, &plan);
        let titles: Vec<_> = g.nodes.iter().map(|n| (n.title.as_str(), n.level, n.group)).collect();
        assert_eq!(titles, vec![("lint", 0, false), ("Test", 1, true), ("deploy", 2, false), ("stray", 0, false)]);
        assert_eq!(g.nodes[1].jobs, vec![1, 2]);
        assert_eq!(g.edges, vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn matrix_suffixes_and_missing_needs() {
        let plan = gh::planned_jobs("jobs:\n  build:\n    needs: setup\n  build-docs: {}\n");
        let jobs = [job("build (linux)"), job("build (mac)"), job("build-docs")];
        let g = layout(&jobs, &plan);
        // `setup` never ran: build still starts the graph.
        assert_eq!(g.nodes[0].level, 0);
        assert!(g.nodes[0].group);
        assert_eq!(g.nodes[1].title, "build-docs");
        assert!(g.edges.is_empty());
    }

    #[test]
    fn pans_freely_but_keeps_the_graph_in_view() {
        use gpui::{point, px, size};
        let (content, view) = (size(px(600.), px(100.)), size(px(1000.), px(500.)));
        // Fits entirely, yet still moves down and right.
        assert_eq!(super::clamp_pan(point(px(50.), px(120.)), content, view), point(px(50.), px(120.)));
        // Pushed far away: 80px of it stays on screen.
        assert_eq!(super::clamp_pan(point(px(5000.), px(-5000.)), content, view), point(px(920.), px(-20.)));
    }

    #[test]
    fn elbows_route_without_panicking() {
        use gpui::{PathBuilder, point, px};
        for (a, b) in [((0., 10.), (64., 10.)), ((0., 10.), (64., 200.)), ((0., 200.), (64., 12.)), ((0., 10.), (64., 13.))] {
            let mut path = PathBuilder::stroke(px(1.5));
            super::elbow(&mut path, point(px(a.0), px(a.1)), point(px(b.0), px(b.1)), px(8.));
            assert!(path.build().is_ok());
        }
    }

    #[test]
    fn reusable_workflow_release_pipeline() {
        let plan = gh::planned_jobs(
            "jobs:
  set-version-number:
    name: Set version number
  code-quality:
    name: Code Quality
    uses: ./.github/workflows/quality.yaml
  build:
    name: Build Backend
    needs: code-quality
  create-release:
    name: Create GitHub Release
    needs:
      - set-version-number
      - code-quality
      - build
  deploy-backend-prd:
    name: Backend - Release - Production
    needs:
      - build
      - create-release
",
        );
        let jobs = [
            job("Set version number"),
            job("Code Quality / Lint, Build, Test"),
            job("Build Backend"),
            job("Create GitHub Release"),
            job("Backend - Release - Production / Deploy Backend"),
        ];
        let g = layout(&jobs, &plan);
        let cols: Vec<_> = g.nodes.iter().map(|n| (n.title.as_str(), n.level)).collect();
        assert_eq!(
            cols,
            vec![
                ("Set version number", 0),
                ("Code Quality", 0),
                ("Build Backend", 1),
                ("Create GitHub Release", 2),
                ("Backend - Release - Production", 3),
            ]
        );
        assert_eq!(g.edges.len(), 6);
    }
}
