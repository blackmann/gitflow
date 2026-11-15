//! Small building blocks shared by every view, in CloudWatch v2's visual roles.
use super::*;
use crate::{
    gh::CheckState,
    icons::Glyph,
    menu::Entry,
    util,
};
use gpui_component::{
    Icon, IconName, Sizable,
    button::{Button, ButtonVariants},
    kbd::Kbd,
    spinner::Spinner,
    text::TextView,
};

/// Shared layout rhythm: tab strip, nav bar and panel headers line up.
pub const BAR: f32 = 34.;
pub const ROW: f32 = 28.;

pub fn row() -> Div {
    div().flex().items_center()
}
pub fn col() -> Div {
    div().flex().flex_col()
}
pub fn label(colors: Palette, text: impl Into<SharedString>) -> Div {
    div()
        .text_color(rgb(colors.muted))
        .text_size(px(12.))
        .child(text.into())
}
pub fn icon(name: impl Into<Icon>) -> Icon {
    name.into().size(px(14.))
}
pub fn button(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Button {
    Button::new(id).label(text).ghost().small()
}
pub fn icon_button(id: impl Into<ElementId>, name: impl Into<Icon>, tip: &'static str) -> Button {
    Button::new(id).icon(name).ghost().small().tooltip(tip)
}
/// An icon toggle that reads as "on" through color.
pub fn toggle(colors: Palette, id: &'static str, name: impl Into<Icon>, tip: &'static str, on: bool) -> Button {
    Button::new(id)
        .icon(icon(name).text_color(rgb(if on { colors.blue } else { colors.muted })))
        .ghost()
        .xsmall()
        .tooltip(tip)
}
pub fn divider(colors: Palette) -> Div {
    div()
        .w(px(1.))
        .h(px(16.))
        .mx_1()
        .flex_shrink_0()
        .bg(rgb(colors.border))
}
pub fn kbd(keys: &str) -> Kbd {
    Kbd::new(Keystroke::parse(keys).expect("valid keystroke"))
}
pub fn pill(text: impl Into<SharedString>, fg: u32, bg: u32) -> Div {
    div()
        .flex_shrink_0()
        .px(px(6.))
        .h(px(18.))
        .flex()
        .items_center()
        .gap(px(4.))
        .rounded(px(4.))
        .bg(rgb(bg))
        .text_color(rgb(fg))
        .text_size(px(11.))
        .font_weight(FontWeight::MEDIUM)
        .child(text.into())
}
pub fn count_badge(colors: Palette, n: usize) -> Div {
    div()
        .flex_shrink_0()
        .min_w(px(18.))
        .h(px(16.))
        .px(px(4.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .bg(rgb(colors.hover))
        .text_size(px(11.))
        .text_color(rgb(colors.muted))
        .child(n.to_string())
}
/// A branch or ref name, in mono on a quiet chip.
pub fn chip(colors: Palette, mono: SharedString, text: impl Into<SharedString>) -> Div {
    div()
        .flex_shrink_0()
        .max_w(px(260.))
        .px(px(6.))
        .h(px(18.))
        .flex()
        .items_center()
        .rounded(px(4.))
        .bg(rgb(colors.blue_tint))
        .text_color(rgb(colors.blue))
        .font_family(mono)
        .text_size(px(11.5))
        .truncate()
        .child(text.into())
}
pub fn spinner(colors: Palette) -> Spinner {
    Spinner::new().xsmall().color(rgb(colors.amber).into())
}

/// A centered placeholder for empty, loading and failed states.
pub fn empty(colors: Palette, glyph: impl Into<Icon>, title: impl Into<SharedString>, detail: impl Into<SharedString>) -> Div {
    col()
        .flex_1()
        .size_full()
        .items_center()
        .justify_center()
        .gap_2()
        .p_6()
        .child(icon(glyph).size(px(22.)).text_color(rgb(colors.quiet)))
        .child(div().font_weight(FontWeight::MEDIUM).child(title.into()))
        .child(
            label(colors, detail)
                .max_w(px(420.))
                .text_center()
                .line_height(relative(1.45)),
        )
}
pub fn loading(colors: Palette, text: &str) -> Div {
    row()
        .flex_1()
        .size_full()
        .justify_center()
        .gap_2()
        .text_color(rgb(colors.muted))
        .child(Spinner::new().small())
        .child(text.to_string())
}
pub fn failed(colors: Palette, error: &str) -> Div {
    empty(colors, IconName::TriangleAlert, "Couldn’t load this", error.to_string())
}

/// Ready content, or a placeholder for loading / failure.
#[allow(clippy::result_large_err)]
pub fn state<T>(colors: Palette, l: Option<&Load<T>>, what: &str) -> Result<(), Div> {
    match l {
        Some(Load { data: Some(_), .. }) => Ok(()),
        Some(Load {
            error: Some(e), ..
        }) => Err(failed(colors, e)),
        _ => Err(loading(colors, what)),
    }
}

/// Run and check outcomes as a glyph and a color.
pub fn run_glyph(colors: Palette, status: &str, conclusion: &str) -> (Icon, u32) {
    let status = status.to_ascii_lowercase();
    if status != "completed" {
        return if status == "in_progress" {
            (Glyph::Activity.into(), colors.amber)
        } else {
            (Glyph::CircleDashed.into(), colors.amber)
        };
    }
    match conclusion.to_ascii_lowercase().as_str() {
        "success" => (IconName::CircleCheck.into(), colors.green),
        "failure" | "timed_out" | "startup_failure" => (IconName::CircleX.into(), colors.red),
        "cancelled" => (Glyph::CircleSlash.into(), colors.quiet),
        "action_required" => (IconName::TriangleAlert.into(), colors.amber),
        _ => (Glyph::CircleSlash.into(), colors.quiet),
    }
}
pub fn run_icon(colors: Palette, status: &str, conclusion: &str) -> AnyElement {
    let (glyph, color) = run_glyph(colors, status, conclusion);
    if status.eq_ignore_ascii_case("in_progress") {
        return spinner(colors).into_any_element();
    }
    icon(glyph).text_color(rgb(color)).into_any_element()
}
pub fn check_icon(colors: Palette, state: CheckState) -> Icon {
    match state {
        CheckState::Pass => icon(IconName::CircleCheck).text_color(rgb(colors.green)),
        CheckState::Fail => icon(IconName::CircleX).text_color(rgb(colors.red)),
        CheckState::Pending => icon(Glyph::CircleDashed).text_color(rgb(colors.amber)),
        CheckState::Skipped => icon(Glyph::CircleSlash).text_color(rgb(colors.quiet)),
    }
}
/// Pull request state as glyph, color and tint: open, draft, merged, closed.
pub fn pr_glyph(colors: Palette, state: &str, draft: bool) -> (Glyph, u32, u32, &'static str) {
    match state {
        "MERGED" => (Glyph::Merge, colors.purple, colors.purple_tint, "Merged"),
        "CLOSED" => (Glyph::PullRequestClosed, colors.red, colors.red_tint, "Closed"),
        _ if draft => (Glyph::PullRequestDraft, colors.muted, colors.hover, "Draft"),
        _ => (Glyph::PullRequest, colors.green, colors.green_tint, "Open"),
    }
}

/// Drop HTML comments that PR templates leave behind before rendering Markdown.
pub fn clean_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}
pub fn markdown(id: impl Into<SharedString>, text: &str, window: &mut Window, cx: &mut App) -> AnyElement {
    let text = clean_markdown(text);
    TextView::markdown(ElementId::Name(id.into()), text, window, cx)
        .selectable(true)
        .text_size(px(13.))
        .into_any_element()
}

impl Workspace {
    /// A round avatar, or initials on a lane color until the image arrives.
    pub(super) fn avatar(&self, login: Option<&str>, name: &str, size: f32, cx: &App) -> AnyElement {
        let colors = palette(cx);
        let login = login.filter(|l| !l.is_empty());
        if settings::get(cx).avatars
            && let Some(image) = login.and_then(|l| self.avatars.get(l))
        {
            return img(ImageSource::Image(image))
                .size(px(size))
                .flex_shrink_0()
                .rounded_full()
                .into_any_element();
        }
        let seed = login.unwrap_or(name);
        div()
            .size(px(size))
            .flex_shrink_0()
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(Hsla::from(rgb(colors.lane(util::hash(seed)))).opacity(0.18))
            .text_color(rgb(colors.lane(util::hash(seed))))
            .text_size(px((size * 0.42).max(8.)))
            .font_weight(FontWeight::SEMIBOLD)
            .child(util::initials(if name.is_empty() { seed } else { name }))
            .into_any_element()
    }
    /// Wrap a popover trigger so its painted bounds are known when it is clicked.
    pub(super) fn anchor(&self, id: &'static str, trigger: impl IntoElement) -> Div {
        let anchors = self.anchors.clone();
        div().relative().flex_shrink_0().child(trigger).child(
            canvas(
                move |bounds, _, _| {
                    anchors.borrow_mut().insert(id, bounds);
                },
                |_, _, _, _| {},
            )
            .absolute()
            .inset_0(),
        )
    }
    /// A button that opens a popover built from live workspace state.
    pub(super) fn popover_button(
        &self,
        id: &'static str,
        button: Button,
        search: Option<&'static str>,
        width: f32,
        build: impl Fn(&Workspace, WeakEntity<Workspace>, &App) -> Vec<Entry> + 'static,
        cx: &mut Context<Self>,
    ) -> Div {
        let weak = cx.weak_entity();
        let build: popover::Builder = Rc::new(move |cx: &App| {
            weak.upgrade()
                .map(|w| build(w.read(cx), weak.clone(), cx))
                .unwrap_or_default()
        });
        self.anchor(
            id,
            button.on_click(cx.listener(move |s, e: &ClickEvent, w, cx| {
                // Mouse-down outside already closed this trigger's menu: treat the click as a toggle.
                if s.dismissed
                    .take()
                    .is_some_and(|(a, at)| a == id && at.elapsed() < Duration::from_millis(400))
                {
                    return;
                }
                s.open_popover(id, e.position(), search, width, build.clone(), w, cx)
            })),
        )
    }
    pub(super) fn when(&self, ts: i64, cx: &App) -> String {
        util::when(ts, settings::get(cx).relative_dates)
    }
    pub(super) fn busy_with(&self, prefix: &str) -> bool {
        self.busy.as_deref().is_some_and(|b| b.starts_with(prefix))
    }
}

/// A titled panel with an optional trailing control, used by detail pages.
pub fn section(colors: Palette, title: impl Into<SharedString>) -> Div {
    row()
        .h(px(30.))
        .gap_2()
        .text_size(px(12.))
        .text_color(rgb(colors.muted))
        .font_weight(FontWeight::MEDIUM)
        .child(title.into())
}
pub fn card(colors: Palette) -> Div {
    col()
        .rounded(px(8.))
        .border_1()
        .border_color(rgb(colors.border))
        .bg(rgb(colors.background))
        .overflow_hidden()
}
#[cfg(test)]
mod tests {
    #[test]
    fn strips_template_comments() {
        assert_eq!(super::clean_markdown("a<!-- hidden -->b\n<!-- x"), "ab");
        assert_eq!(super::clean_markdown("plain"), "plain");
    }
}
