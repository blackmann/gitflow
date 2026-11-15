//! A unified diff as one virtualized list: a header per file, then numbered lines
//! tinted by change. Used by commit details and pull request "Files".
use super::ui::*;
use super::*;
use crate::{
    gh::{FileStatus, LineKind},
    icons::Glyph,
};
use gpui::prelude::FluentBuilder;
use gpui_component::IconName;

const LINE: f32 = 22.;

pub fn status_glyph(colors: Palette, status: FileStatus) -> (gpui_component::Icon, u32) {
    match status {
        FileStatus::Added => (IconName::Plus.into(), colors.green),
        FileStatus::Deleted => (IconName::Minus.into(), colors.red),
        FileStatus::Renamed => (IconName::ArrowRight.into(), colors.blue),
        FileStatus::Modified => (Glyph::FileDiff.into(), colors.amber),
    }
}
pub fn counts(colors: Palette, additions: u32, deletions: u32) -> Div {
    row()
        .gap(px(6.))
        .flex_shrink_0()
        .text_size(px(12.))
        .child(div().text_color(rgb(colors.green)).child(format!("+{additions}")))
        .child(div().text_color(rgb(colors.red)).child(format!("−{deletions}")))
}

impl Workspace {
    /// The changed-file list; clicking a file scrolls the diff to it.
    pub(super) fn diff_files(&self, colors: Palette, diff: &Arc<DiffView>, scroll: UniformListScrollHandle, cx: &mut Context<Self>) -> AnyElement {
        let mut list = col().py_1();
        for (i, f) in diff.files.iter().enumerate() {
            let (glyph, color) = status_glyph(colors, f.status);
            let (name, dir) = match f.path.rsplit_once('/') {
                Some((d, n)) => (n.to_string(), d.to_string()),
                None => (f.path.clone(), String::new()),
            };
            let target = diff.header_row(i);
            let scroll = scroll.clone();
            list = list.child(
                row()
                    .id(("diff-file", i))
                    .h(px(26.))
                    .px_2()
                    .gap_2()
                    .cursor_pointer()
                    .rounded(px(4.))
                    .hover(|s| s.bg(rgb(colors.hover)))
                    .child(glyph.size(px(12.)).text_color(rgb(color)))
                    .child(
                        row()
                            .flex_1()
                            .min_w_0()
                            .gap(px(5.))
                            .child(div().flex_shrink_0().max_w(relative(0.8)).truncate().child(name))
                            .child(div().min_w_0().truncate().text_size(px(11.)).text_color(rgb(colors.quiet)).child(dir)),
                    )
                    .child(counts(colors, f.additions, f.deletions))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        if let Some(r) = target {
                            scroll.scroll_to_item(r, ScrollStrategy::Top);
                        }
                        cx.notify();
                    })),
            );
        }
        div()
            .id("diff-files")
            .size_full()
            .overflow_y_scroll()
            .px_1()
            .child(list)
            .into_any_element()
    }

    pub(super) fn diff_list(&self, id: (&'static str, u64), diff: Arc<DiffView>, scroll: UniformListScrollHandle, cx: &mut Context<Self>) -> AnyElement {
        let colors = palette(cx);
        if diff.files.is_empty() {
            return empty(colors, Glyph::FileDiff, "No file changes", "This change has no textual diff.").into_any_element();
        }
        let mono = settings::mono(cx);
        let size = settings::get(cx).mono_size as f32;
        let longest = diff
            .files
            .iter()
            .flat_map(|f| f.lines.iter().take(DiffView::MAX_LINES))
            .map(|l| l.text.len())
            .max()
            .unwrap_or(0)
            .min(1200);
        let width = 120. + longest as f32 * size * 0.62;
        let rows = diff.rows.len();
        let d = diff.clone();
        let list = uniform_list(
            id,
            rows,
            cx.processor(move |_, range: std::ops::Range<usize>, _, cx| {
                let colors = palette(cx);
                range
                    .map(|i| {
                        let (f, line) = d.rows[i];
                        let file = &d.files[f];
                        match line {
                            None => {
                                let (glyph, color) = status_glyph(colors, file.status);
                                let more = file.lines.len().saturating_sub(DiffView::MAX_LINES);
                                row()
                                    .h(px(LINE))
                                    .w_full()
                                    .px_2()
                                    .gap_2()
                                    .bg(rgb(colors.chrome))
                                    .border_y_1()
                                    .border_color(rgb(colors.border))
                                    .font_family(".SystemUIFont")
                                    .text_size(px(12.5))
                                    .child(glyph.size(px(13.)).text_color(rgb(color)))
                                    .child(
                                        div().font_weight(FontWeight::MEDIUM).child(match &file.old_path {
                                            Some(old) => format!("{old} → {}", file.path),
                                            None => file.path.clone(),
                                        }),
                                    )
                                    .child(counts(colors, file.additions, file.deletions))
                                    .when(file.binary, |s| s.child(label(colors, "binary")))
                                    .when(more > 0, |s| s.child(label(colors, format!("{more} more lines not shown"))))
                                    .into_any_element()
                            }
                            Some(l) => {
                                let line = &file.lines[l];
                                let (bg, fg, mark) = match line.kind {
                                    LineKind::Add => (Some(colors.green_tint), colors.text, "+"),
                                    LineKind::Del => (Some(colors.red_tint), colors.text, "−"),
                                    LineKind::Hunk => (Some(colors.blue_tint), colors.muted, ""),
                                    LineKind::Context => (None, colors.text, ""),
                                };
                                let num = |n: Option<u32>| {
                                    div()
                                        .w(px(44.))
                                        .flex_shrink_0()
                                        .pr_2()
                                        .text_right()
                                        .text_color(rgb(colors.quiet))
                                        .child(n.map(|n| n.to_string()).unwrap_or_default())
                                };
                                row()
                                    .h(px(LINE))
                                    .w_full()
                                    .when_some(bg, |s, bg| s.bg(rgb(bg)))
                                    .child(num(line.old))
                                    .child(num(line.new))
                                    .child(
                                        div()
                                            .w(px(16.))
                                            .flex_shrink_0()
                                            .text_color(rgb(match line.kind {
                                                LineKind::Add => colors.green,
                                                LineKind::Del => colors.red,
                                                _ => colors.quiet,
                                            }))
                                            .child(mark),
                                    )
                                    .child(div().pr_4().whitespace_nowrap().text_color(rgb(fg)).child(line.text.replace('\t', "    ")))
                                    .into_any_element()
                            }
                        }
                    })
                    .collect()
            }),
        )
        .track_scroll(scroll)
        // Inside a horizontal scroller: keep sideways swipes from also scrolling rows.
        .map(|mut list| {
            list.style().restrict_scroll_to_axis = Some(true);
            list
        })
        .h_full()
        .min_w(px(width))
        .w_full();
        div()
            .id(SharedString::from(format!("{}-x-{}", id.0, id.1)))
            .size_full()
            .overflow_x_scroll()
            .map(|mut d| {
                d.style().restrict_scroll_to_axis = Some(true);
                d
            })
            .font_family(mono)
            .text_size(px(size))
            .child(list)
            .into_any_element()
    }
}
