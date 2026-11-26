//! Releases: a timeline of release cards with notes and downloadable assets.
use super::ui::*;
use super::*;
use crate::{icons::Glyph, util};
use gpui::prelude::FluentBuilder;
use gpui_component::{
    IconName, Sizable,
    button::{Button, ButtonVariants},
};

impl Workspace {
    pub(super) fn releases_view(&self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let colors = palette(cx);
        let Some(t) = self.current() else { return div().into_any_element() };
        let Some(repo) = self.repo(&t.repo).cloned() else { return div().into_any_element() };
        let l = self.data(&t.repo).map(|d| &d.releases);
        if let Err(p) = state(colors, l, "Loading releases…") {
            return p.into_any_element();
        }
        let releases = l.and_then(|l| l.data.as_ref()).unwrap();
        if releases.is_empty() {
            let url = format!("https://github.com/{}/releases/new", repo.slug());
            return empty(colors, Glyph::Tag, "No releases yet", "Tag a version on GitHub to publish notes and binaries.")
                .child(
                    Button::new("draft-release")
                        .label("Draft a release on GitHub")
                        .icon(IconName::ExternalLink)
                        .outline()
                        .small()
                        .on_click(move |_, _, cx| cx.open_url(&url)),
                )
                .into_any_element();
        }
        // Only cards in view (plus overdraw) are laid out, Markdown included.
        let state = t.release_list.clone();
        if state.item_count() != releases.len() {
            state.reset(releases.len());
        }
        let list = list(
            state.clone(),
            cx.processor(|s, i: usize, window, cx| s.release_card(i, window, cx)),
        )
        .size_full();
        scrollbars(("releases-scrollbar", t.id), &state, ScrollbarAxis::Vertical, list)
            .bg(rgb(colors.background))
            .into_any_element()
    }

    fn release_card(&self, i: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let colors = palette(cx);
        let mono = settings::mono(cx);
        let Some(t) = self.current() else { return div().into_any_element() };
        let Some(releases) = self.data(&t.repo).and_then(|d| d.releases.data.as_ref()) else {
            return div().into_any_element();
        };
        let Some(r) = releases.get(i) else { return div().into_any_element() };
        let latest = releases
            .iter()
            .find(|r| !r.draft && !r.prerelease)
            .map(|r| r.tag_name.clone());
        {
            let is_latest = latest.as_ref() == Some(&r.tag_name);
            let author = r.author.as_ref().map(|a| a.login.clone()).unwrap_or_default();
            let url = r.html_url.clone();
            let tag = r.tag_name.clone();
            let mut card = card(colors)
                .child(
                    row()
                        .px_4()
                        .py_3()
                        .gap_3()
                        .border_b_1()
                        .border_color(rgb(colors.row_border))
                        .child(
                            col()
                                .flex_1()
                                .min_w_0()
                                .gap(px(3.))
                                .child(
                                    row()
                                        .gap_2()
                                        .child(div().text_size(px(16.)).font_weight(FontWeight::SEMIBOLD).truncate().child(r.title()))
                                        .when(is_latest, |s| s.child(pill("Latest", colors.green, colors.green_tint)))
                                        .when(r.prerelease, |s| s.child(pill("Pre-release", colors.amber, colors.amber_tint)))
                                        .when(r.draft, |s| s.child(pill("Draft", colors.muted, colors.hover))),
                                )
                                .child(
                                    row()
                                        .gap_2()
                                        .text_size(px(12.))
                                        .text_color(rgb(colors.muted))
                                        .child(chip(colors, mono.clone(), r.tag_name.clone()))
                                        .when(!author.is_empty(), |s| {
                                            s.child(self.avatar(Some(&author), &author, 16., cx)).child(author.clone())
                                        })
                                        .child(format!("released {}", self.when(r.time(), cx))),
                                ),
                        )
                        .child(
                            Button::new(("copy-tag", i))
                                .icon(IconName::Copy)
                                .ghost()
                                .xsmall()
                                .tooltip("Copy tag")
                                .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(tag.clone()))),
                        )
                        .child(
                            Button::new(("open-release", i))
                                .icon(IconName::ExternalLink)
                                .ghost()
                                .xsmall()
                                .tooltip("Open on GitHub")
                                .on_click(move |_, _, cx| cx.open_url(&url)),
                        ),
                );
            let body = r.body.clone().unwrap_or_default();
            card = card.child(div().px_5().py_4().child(if body.trim().is_empty() {
                label(colors, "No release notes").into_any_element()
            } else {
                markdown(format!("release-{}-{}", t.repo, r.tag_name), &body, window, cx)
            }));
            if !r.assets.is_empty() {
                let mut assets = col().border_t_1().border_color(rgb(colors.row_border)).child(
                    section(colors, format!("Assets · {}", r.assets.len())).px_4(),
                );
                for (k, a) in r.assets.iter().enumerate() {
                    let link = a.browser_download_url.clone();
                    assets = assets.child(
                        row()
                            .id(SharedString::from(format!("asset-{i}-{k}")))
                            .h(px(32.))
                            .px_4()
                            .gap_3()
                            .cursor_pointer()
                            .border_t_1()
                            .border_color(rgb(colors.row_border))
                            .hover(|s| s.bg(rgb(colors.hover)))
                            .child(icon(Glyph::Package).text_color(rgb(colors.muted)))
                            .child(div().flex_1().min_w_0().truncate().child(a.name.clone()))
                            .child(label(colors, util::bytes(a.size)))
                            .child(
                                row()
                                    .gap(px(4.))
                                    .w(px(80.))
                                    .justify_end()
                                    .text_size(px(12.))
                                    .text_color(rgb(colors.muted))
                                    .child(icon(Glyph::Download).size(px(12.)))
                                    .child(util::compact(a.download_count)),
                            )
                            .on_click(move |_, _, cx| cx.open_url(&link)),
                    );
                }
                card = card.child(assets);
            }
            // Timeline: date, a rail fading between neighbouring release colors, the card.
            let tone = |r: &gh::Release| {
                if latest.as_ref() == Some(&r.tag_name) {
                    colors.green
                } else if r.draft {
                    colors.quiet
                } else if r.prerelease {
                    colors.amber
                } else {
                    colors.blue
                }
            };
            let top = if i == 0 { 16. } else { 0. };
            let show_day = i == 0 || !same_day(releases[i - 1].time(), r.time());
            div().w_full().px_4().child(
                div()
                    .flex()
                    .max_w(px(1000.))
                    .w_full()
                    .mx_auto()
                    .gap_2()
                    .child(date_column(colors, r.time(), show_day, top + 16.))
                    .child(rail(
                        i.checked_sub(1).map(|p| tone(&releases[p])),
                        tone(r),
                        releases.get(i + 1).map(tone),
                        top + 20.,
                        None,
                        colors.background,
                    ))
                    .child(div().flex_1().min_w_0().pt(px(top)).pb_5().child(card)),
            )
            .into_any_element()
        }
    }
}
