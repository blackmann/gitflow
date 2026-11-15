//! Code: a branch switcher, breadcrumbs, the folder listing with its README, and a
//! virtualized file viewer with line numbers.
use super::ui::*;
use super::*;
use crate::{
    icons::Glyph,
    menu::{Entry, Item},
    util,
};
use gpui::prelude::FluentBuilder;
use gpui_component::{Disableable, IconName};

const LINE: f32 = 20.;

impl Workspace {
    /// The branch popover shared by Code and remote-only Commits.
    pub(super) fn branch_button(&self, id: &'static str, current: String, cx: &mut Context<Self>) -> Div {
        let colors = palette(cx);
        self.popover_button(
            id,
            button(SharedString::from(format!("{id}-trigger")), current)
                .icon(icon(Glyph::Branch).text_color(rgb(colors.muted)))
                .dropdown_caret(true)
                .tooltip("Switch branch"),
            Some("Find a branch…"),
            320.,
            |s, weak, _| {
                let Some(t) = s.current() else { return vec![] };
                let repo = t.repo.clone();
                let current = s.branch_of(t);
                let default = s.default_branch(&repo);
                let data = s.data(&repo);
                let Some(branches) = data.and_then(|d| d.branches.data.as_ref()) else {
                    return vec![Entry::Item(Item::new("Loading branches…").disabled(true))];
                };
                let mut sorted = branches.iter().collect::<Vec<_>>();
                // Default first, then alphabetical.
                sorted.sort_by_key(|b| (Some(&b.name) != default.as_ref(), b.name.to_lowercase()));
                let mut out = vec![Entry::Label(format!("Branches · {}", branches.len()).into())];
                for b in sorted {
                    let weak = weak.clone();
                    let name = b.name.clone();
                    let is_default = Some(&b.name) == default.as_ref();
                    out.push(Entry::Item(
                        Item::new(b.name.clone())
                            .icon(if b.protected { Glyph::Lock } else { Glyph::Branch })
                            .checked(Some(&b.name) == current.as_ref())
                            .detail(if is_default { "default" } else { "" })
                            .on_click(move |_, cx| {
                                let name = name.clone();
                                let _ = weak.update(cx, |s, cx| {
                                    if let Some(t) = s.current_mut() {
                                        t.branch = if is_default { None } else { Some(name) };
                                        t.file = None;
                                    }
                                    cx.notify();
                                });
                            }),
                    ));
                }
                out
            },
            cx,
        )
    }

    fn breadcrumbs(&self, colors: Palette, t: &Tab, repo_name: String, cx: &mut Context<Self>) -> Div {
        let mut crumbs = row().min_w_0().gap(px(2.)).text_size(px(13.));
        let target = t.file.clone().unwrap_or_else(|| t.path.clone());
        let parts = target.split('/').filter(|p| !p.is_empty()).collect::<Vec<_>>();
        let crumb = |id: SharedString, text: String, path: Option<String>, last: bool| {
            div()
                .id(id)
                .px_1()
                .rounded(px(4.))
                .truncate()
                .when(last, |s| s.font_weight(FontWeight::MEDIUM))
                .when(!last, |s| {
                    s.text_color(rgb(colors.blue))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(colors.hover)))
                })
                .child(text)
                .when_some(path.filter(|_| !last), |s, path| {
                    s.on_click(cx.listener(move |s, _, _, cx| {
                        if let Some(t) = s.current_mut() {
                            t.path = path.clone();
                            t.file = None;
                        }
                        cx.notify();
                    }))
                })
        };
        crumbs = crumbs.child(crumb("crumb-root".into(), repo_name, Some(String::new()), parts.is_empty()));
        for (i, p) in parts.iter().enumerate() {
            let path = parts[..=i].join("/");
            crumbs = crumbs
                .child(div().text_color(rgb(colors.quiet)).child("/"))
                .child(crumb(
                    SharedString::from(format!("crumb-{i}")),
                    p.to_string(),
                    Some(path),
                    i == parts.len() - 1,
                ));
        }
        crumbs
    }

    pub(super) fn code_view(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let colors = palette(cx);
        let Some(t) = self.current() else {
            return div().into_any_element();
        };
        let Some(repo) = self.repo(&t.repo).cloned() else {
            return div().into_any_element();
        };
        let data = self.data(&t.repo);
        let Some(branch) = self.branch_of(t) else {
            return match data.and_then(|d| d.info.error.clone()) {
                Some(e) => failed(colors, &e).into_any_element(),
                None => loading(colors, "Loading repository…").into_any_element(),
            };
        };
        let header = row()
            .h(px(BAR + 6.))
            .flex_shrink_0()
            .px_2()
            .gap_1()
            .border_b_1()
            .border_color(rgb(colors.row_border))
            .child(self.branch_button("code-branch", branch.clone(), cx))
            .child(divider(colors))
            .child(self.breadcrumbs(colors, t, repo.name.clone(), cx))
            .child(div().flex_1())
            .when_some(t.file.clone(), |s, file| {
                let url = format!("https://github.com/{}/blob/{}/{}", repo.slug(), branch, file);
                let raw = data
                    .and_then(|d| d.blob.get(&(branch.clone(), file.clone())))
                    .and_then(|l| l.data.clone());
                s.child(
                    icon_button("copy-file", IconName::Copy, "Copy file contents")
                        .disabled(!matches!(raw.as_deref(), Some(BlobView::Text(_))))
                        .on_click(move |_, _, cx| {
                            if let Some(BlobView::Text(lines)) = raw.as_deref() {
                                let text = lines.iter().map(|l| l.as_ref()).collect::<Vec<&str>>().join("\n");
                                cx.write_to_clipboard(ClipboardItem::new_string(text));
                            }
                        }),
                )
                .child(
                    icon_button("copy-path", Glyph::Link, "Copy path")
                        .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(file.clone()))),
                )
                .child(icon_button("file-github", IconName::ExternalLink, "Open file on GitHub").on_click(move |_, _, cx| cx.open_url(&url)))
            });
        let body = match &t.file {
            Some(file) => self.file_view(colors, t, &branch, file, cx),
            None => self.folder_view(colors, t, &branch, window, cx),
        };
        col()
            .size_full()
            .bg(rgb(colors.background))
            .child(header)
            .child(div().flex_1().min_h_0().child(body))
            .into_any_element()
    }

    fn folder_view(&self, colors: Palette, t: &Tab, branch: &str, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let data = self.data(&t.repo);
        let key = (branch.to_string(), t.path.clone());
        let l = data.and_then(|d| d.tree.get(&key));
        if let Err(placeholder) = state(colors, l, "Loading files…") {
            return placeholder.into_any_element();
        }
        let entries = l.and_then(|l| l.data.as_ref()).unwrap();
        let mut list = card(colors);
        if !t.path.is_empty() {
            let parent = t.path.rsplit_once('/').map(|(p, _)| p.to_string()).unwrap_or_default();
            list = list.child(
                row()
                    .id("entry-up")
                    .h(px(ROW + 4.))
                    .px_3()
                    .gap_2()
                    .cursor_pointer()
                    .text_color(rgb(colors.muted))
                    .hover(|s| s.bg(rgb(colors.hover)))
                    .child(icon(IconName::ArrowUp).size(px(13.)))
                    .child("..")
                    .on_click(cx.listener(move |s, _, _, cx| {
                        if let Some(t) = s.current_mut() {
                            t.path = parent.clone();
                        }
                        cx.notify();
                    })),
            );
        }
        for (i, e) in entries.iter().enumerate() {
            let dir = e.is_dir();
            let path = e.path.clone();
            let glyph = match e.kind.as_str() {
                "dir" => icon(IconName::FolderClosed).text_color(rgb(colors.blue)),
                "submodule" => icon(Glyph::FolderGit).text_color(rgb(colors.purple)),
                "symlink" => icon(Glyph::Link).text_color(rgb(colors.muted)),
                _ => icon(IconName::File).text_color(rgb(colors.muted)),
            };
            list = list.child(
                row()
                    .id(("entry", i))
                    .h(px(ROW + 4.))
                    .px_3()
                    .gap_2()
                    .cursor_pointer()
                    .when(i > 0 || !t.path.is_empty(), |s| s.border_t_1().border_color(rgb(colors.row_border)))
                    .hover(|s| s.bg(rgb(colors.hover)))
                    .child(glyph.size(px(14.)))
                    .child(div().flex_1().min_w_0().truncate().child(e.name.clone()))
                    .when(!dir && e.size > 0, |s| s.child(label(colors, util::bytes(e.size))))
                    .on_click(cx.listener(move |s, _, _, cx| {
                        if let Some(t) = s.current_mut() {
                            if dir {
                                t.path = path.clone();
                                t.file = None;
                            } else {
                                t.file = Some(path.clone());
                            }
                        }
                        cx.notify();
                    })),
            );
        }
        if entries.is_empty() {
            list = list.child(div().p_4().text_color(rgb(colors.muted)).child("This folder is empty"));
        }
        let readme = entries
            .iter()
            .find(|e| !e.is_dir() && e.name.to_lowercase().starts_with("readme"));
        let mut page = col().p_4().gap_4().max_w(px(980.)).w_full().mx_auto().child(list);
        if let Some(readme) = readme {
            let blob = data
                .and_then(|d| d.blob.get(&(branch.to_string(), readme.path.clone())))
                .and_then(|l| l.data.clone());
            let md = readme.name.to_lowercase().ends_with(".md") || !readme.name.contains('.');
            page = page.child(
                card(colors)
                    .child(
                        row()
                            .h(px(BAR))
                            .px_3()
                            .gap_2()
                            .bg(rgb(colors.chrome))
                            .border_b_1()
                            .border_color(rgb(colors.row_border))
                            .child(icon(IconName::BookOpen).text_color(rgb(colors.muted)))
                            .child(div().font_weight(FontWeight::MEDIUM).child(readme.name.clone())),
                    )
                    .child(div().p_5().child(match blob.as_deref() {
                        Some(BlobView::Text(lines)) if md => markdown(
                            format!("readme-{}-{}", t.repo, readme.path),
                            &lines.iter().map(|l| l.as_ref()).collect::<Vec<&str>>().join("\n"),
                            window,
                            cx,
                        ),
                        Some(BlobView::Text(lines)) => div()
                            .font_family(settings::mono(cx))
                            .text_size(px(settings::get(cx).mono_size as f32))
                            .children(lines.iter().take(400).map(|l| div().min_h(px(LINE)).child(l.clone())))
                            .into_any_element(),
                        Some(_) => label(colors, "README isn’t text").into_any_element(),
                        None => row().gap_2().text_color(rgb(colors.muted)).child(spinner(colors)).child("Loading README…").into_any_element(),
                    })),
            );
        }
        div()
            .id(("folder-scroll", t.id))
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&t.page_scroll)
            .child(page)
            .into_any_element()
    }

    fn file_view(&self, colors: Palette, t: &Tab, branch: &str, file: &str, cx: &mut Context<Self>) -> AnyElement {
        let l = self
            .data(&t.repo)
            .and_then(|d| d.blob.get(&(branch.to_string(), file.to_string())));
        if let Err(placeholder) = state(colors, l, "Loading file…") {
            return placeholder.into_any_element();
        }
        let blob = l.and_then(|l| l.data.clone()).unwrap();
        let lines = match blob.as_ref() {
            BlobView::Text(lines) => lines.len(),
            BlobView::Binary(n) => {
                return empty(colors, IconName::File, "Binary file", format!("{} · open it on GitHub to download", util::bytes(*n)))
                    .into_any_element();
            }
            BlobView::TooLarge(n) => {
                return empty(colors, IconName::File, "File too large to preview", format!("{} · GitHub serves files over 1 MB as raw downloads", util::bytes(*n)))
                    .into_any_element();
            }
        };
        let mono = settings::mono(cx);
        let size = px(settings::get(cx).mono_size as f32);
        let gutter = 16. + 8. * lines.to_string().len() as f32;
        let tab = t.id;
        let longest = match blob.as_ref() {
            BlobView::Text(l) => l.iter().map(|s| s.len()).max().unwrap_or(0),
            _ => 0,
        };
        let width = gutter + 24. + longest.min(2000) as f32 * settings::get(cx).mono_size as f32 * 0.62;
        let list = uniform_list(
            ("file-lines", tab),
            lines,
            cx.processor(move |_, range: std::ops::Range<usize>, _, cx| {
                let colors = palette(cx);
                let BlobView::Text(text) = blob.as_ref() else {
                    return vec![];
                };
                text[range.clone()]
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(k, line)| {
                        let n = range.start + k + 1;
                        row()
                            .h(px(LINE))
                            .w_full()
                            .hover(|s| s.bg(rgb(colors.chrome)))
                            .child(
                                div()
                                    .w(px(gutter))
                                    .flex_shrink_0()
                                    .pr_3()
                                    .text_right()
                                    .text_color(rgb(colors.quiet))
                                    .child(n.to_string()),
                            )
                            .child(div().pl_2().whitespace_nowrap().child(line))
                    })
                    .collect()
            }),
        )
        .track_scroll(t.list_scroll.clone())
        .h_full()
        .min_w(px(width))
        .w_full();
        div()
            .id(("file-x", tab))
            .size_full()
            .overflow_x_scroll()
            .map(|mut d| {
                d.style().restrict_scroll_to_axis = Some(true);
                d
            })
            .font_family(mono)
            .text_size(size)
            .pt_1()
            .child(list)
            .into_any_element()
    }
}
