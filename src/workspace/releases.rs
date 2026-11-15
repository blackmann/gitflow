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
    pub(super) fn releases_view(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let colors = palette(cx);
        let mono = settings::mono(cx);
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
        let latest = releases
            .iter()
            .find(|r| !r.draft && !r.prerelease)
            .map(|r| r.tag_name.clone());
        let mut timeline = col().gap_5();
        for (i, r) in releases.iter().enumerate() {
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
                            div()
                                .size(px(30.))
                                .flex_shrink_0()
                                .rounded(px(7.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(rgb(if is_latest { colors.green_tint } else { colors.hover }))
                                .child(icon(Glyph::Tag).text_color(rgb(if is_latest { colors.green } else { colors.muted }))),
                        )
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
            timeline = timeline.child(card);
        }
        div()
            .id(("releases", t.id))
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&t.page_scroll)
            .bg(rgb(colors.background))
            .child(col().p_4().max_w(px(900.)).w_full().mx_auto().child(timeline))
            .into_any_element()
    }
}
