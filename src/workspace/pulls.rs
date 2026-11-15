//! Pull requests: a filterable list, and a detail page with the conversation,
//! commits, checks and files, plus review, merge and comment actions.
use super::ui::*;
use super::*;
use crate::{
    gh::{PrAction, PrFilter},
    icons::Glyph,
    menu::{Entry, Item},
    settings::MergeMethod,
    util,
};
use gpui::prelude::FluentBuilder;
use gpui_component::{
    Disableable,
    IconName, Sizable,
    button::{Button, ButtonVariants},
    input::Input,
    resizable::*,
    tooltip::Tooltip,
};

pub(super) fn segmented<T: Copy + PartialEq + 'static>(
    colors: Palette,
    id: &'static str,
    options: &[(T, String)],
    current: T,
    on: impl Fn(&mut Workspace, T, &mut Context<Workspace>) + 'static + Clone,
    cx: &mut Context<Workspace>,
) -> Div {
    let mut seg = row()
        .p(px(2.))
        .gap(px(2.))
        .rounded(px(6.))
        .bg(rgb(colors.chrome))
        .border_1()
        .border_color(rgb(colors.row_border));
    for (i, (value, title)) in options.iter().enumerate() {
        let active = *value == current;
        let value = *value;
        let on = on.clone();
        seg = seg.child(
            div()
                .id((id, i))
                .px(px(9.))
                .h(px(22.))
                .flex()
                .items_center()
                .rounded(px(4.))
                .text_size(px(12.5))
                .cursor_pointer()
                .map(|s| {
                    if active {
                        s.bg(rgb(colors.background))
                            .text_color(rgb(colors.text))
                            .font_weight(FontWeight::MEDIUM)
                    } else {
                        s.text_color(rgb(colors.muted)).hover(|s| s.text_color(rgb(colors.text)))
                    }
                })
                .child(title.clone())
                .on_click(cx.listener(move |s, _, _, cx| {
                    on(s, value, cx);
                    cx.notify();
                })),
        );
    }
    seg
}

impl Workspace {
    pub(super) fn pulls_view(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        match self.current().and_then(|t| t.pr) {
            Some(n) => self.pull_page(n, window, cx),
            None => self.pull_list(cx),
        }
    }

    fn pull_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = palette(cx);
        let Some(t) = self.current() else { return div().into_any_element() };
        let Some(repo) = self.repo(&t.repo).cloned() else { return div().into_any_element() };
        let l = self.data(&t.repo).and_then(|d| d.pulls.get(&t.pr_filter));
        let q = t.search.read(cx).value().trim().to_lowercase();
        let compare = format!("https://github.com/{}/compare", repo.slug());
        let options = PrFilter::ALL.map(|f| (f, f.title().to_string()));
        let header = row()
            .h(px(BAR + 6.))
            .flex_shrink_0()
            .px_2()
            .gap_2()
            .border_b_1()
            .border_color(rgb(colors.row_border))
            .child(segmented(colors, "pr-filter", &options, t.pr_filter, |s, f, _| {
                if let Some(t) = s.current_mut() {
                    t.pr_filter = f;
                }
            }, cx))
            .child(
                div().w(px(260.)).child(
                    Input::new(&t.search)
                        .small()
                        .prefix(icon(IconName::Search).text_color(rgb(colors.muted)))
                        .cleanable(true),
                ),
            )
            .when(l.is_some_and(|l| l.loading && l.data.is_some()), |s| s.child(spinner(colors)))
            .child(div().flex_1())
            .child(
                Button::new("new-pr")
                    .label("New pull request")
                    .icon(Glyph::Compare)
                    .outline()
                    .small()
                    .tooltip("Compare branches on GitHub")
                    .on_click(move |_, _, cx| cx.open_url(&compare)),
            );
        let body = match state(colors, l, "Loading pull requests…") {
            Err(p) => p.into_any_element(),
            Ok(()) => {
                let pulls = l.and_then(|l| l.data.as_ref()).unwrap();
                let shown = pulls
                    .iter()
                    .filter(|p| {
                        q.is_empty()
                            || p.title.to_lowercase().contains(&q)
                            || p.author.login.to_lowercase().contains(&q)
                            || p.number.to_string() == q.trim_start_matches('#')
                            || p.head_ref_name.to_lowercase().contains(&q)
                    })
                    .collect::<Vec<_>>();
                if shown.is_empty() {
                    empty(
                        colors,
                        Glyph::PullRequest,
                        if q.is_empty() { format!("No {} pull requests", t.pr_filter.title().to_lowercase()) } else { "No matches".into() },
                        if q.is_empty() { "Nothing to review here." } else { "Try another search." },
                    )
                    .into_any_element()
                } else {
                    let mono = settings::mono(cx);
                    let mut list = col();
                    for (i, p) in shown.into_iter().enumerate() {
                        let (glyph, color, _, _) = pr_glyph(colors, &p.state, p.is_draft);
                        let n = p.number;
                        let checks = gh::rollup(&p.status_check_rollup);
                        let review = p.review_decision.as_deref().unwrap_or("");
                        let tip = format!("{} → {}", p.head_ref_name, p.base_ref_name);
                        list = list.child(
                            row()
                                .id(("pr", n))
                                .min_h(px(54.))
                                .px_3()
                                .py_2()
                                .gap_3()
                                .items_start()
                                .cursor_pointer()
                                .when(i > 0, |s| s.border_t_1().border_color(rgb(colors.row_border)))
                                .hover(|s| s.bg(rgb(colors.hover)))
                                .child(div().pt(px(2.)).child(icon(glyph).size(px(16.)).text_color(rgb(color))))
                                .child(
                                    col()
                                        .flex_1()
                                        .min_w_0()
                                        .gap(px(4.))
                                        .child(
                                            row()
                                                .gap(px(6.))
                                                .child(div().min_w_0().truncate().font_weight(FontWeight::MEDIUM).child(p.title.clone()))
                                                .children(p.labels.iter().take(3).map(|l| label_pill(colors, l))),
                                        )
                                        .child(
                                            row()
                                                .gap(px(6.))
                                                .text_size(px(12.))
                                                .text_color(rgb(colors.muted))
                                                .child(format!("#{n}"))
                                                .child(format!("opened {} by {}", self.when(util::parse_time(&p.created_at), cx), p.author.login))
                                                .child(div().text_color(rgb(colors.quiet)).child("·"))
                                                .child(
                                                    div()
                                                        .id(("pr-branch", n))
                                                        .min_w_0()
                                                        .truncate()
                                                        .font_family(mono.clone())
                                                        .text_size(px(11.5))
                                                        .child(p.head_ref_name.clone())
                                                        .tooltip(move |w, cx| Tooltip::new(tip.clone()).build(w, cx)),
                                                ),
                                        ),
                                )
                                .child(
                                    row()
                                        .flex_shrink_0()
                                        .gap_3()
                                        .pt(px(2.))
                                        .when_some(review_pill(colors, review), |s, p| s.child(p))
                                        .when_some(checks, |s, c| s.child(check_icon(colors, c)))
                                        .child(super::diff::counts(colors, p.additions as u32, p.deletions as u32))
                                        .child(self.avatar(Some(&p.author.login), &p.author.login, 20., cx)),
                                )
                                .on_click(cx.listener(move |s, _, _, cx| {
                                    if let Some(t) = s.current_mut() {
                                        t.pr = Some(n);
                                        t.pr_page = PrPage::Conversation;
                                    }
                                    cx.notify();
                                })),
                        );
                    }
                    div()
                        .id(("pr-list", t.id))
                        .size_full()
                        .overflow_y_scroll()
                        .child(list)
                        .into_any_element()
                }
            }
        };
        col()
            .size_full()
            .bg(rgb(colors.background))
            .child(header)
            .child(div().flex_1().min_h_0().child(body))
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn confirm_pr(
        &mut self,
        number: u64,
        title: String,
        detail: String,
        answer: &'static str,
        action: PrAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let prompt = window.prompt(PromptLevel::Warning, &title, Some(&detail), &[answer, "Cancel"], cx);
        cx.spawn(async move |this, cx| {
            if prompt.await == Ok(0) {
                let _ = this.update(cx, |s, cx| s.pr_action(number, action, cx));
            }
        })
        .detach();
    }

    fn pull_page(&self, n: u64, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let colors = palette(cx);
        let mono = settings::mono(cx);
        let Some(t) = self.current() else { return div().into_any_element() };
        let repo = self.repo(&t.repo).cloned();
        let l = self.data(&t.repo).and_then(|d| d.pull.get(&n));
        let crumb = row()
            .h(px(BAR))
            .flex_shrink_0()
            .px_3()
            .gap_1()
            .text_size(px(13.))
            .child(
                row()
                    .id("crumb-pulls")
                    .gap(px(4.))
                    .cursor_pointer()
                    .text_color(rgb(colors.blue))
                    .child(icon(IconName::ChevronLeft).size(px(13.)))
                    .child("Pull requests")
                    .on_click(cx.listener(|s, _, _, cx| {
                        if let Some(t) = s.current_mut() {
                            t.pr = None;
                        }
                        cx.notify();
                    })),
            )
            .child(div().text_color(rgb(colors.quiet)).child("/"))
            .child(div().text_color(rgb(colors.muted)).child(format!("#{n}")))
            .when(l.is_some_and(|l| l.loading), |s| s.child(div().ml_2().child(spinner(colors))));
        let Some(pr) = l.and_then(|l| l.data.as_ref()) else {
            let body = match l.and_then(|l| l.error.clone()) {
                Some(e) => failed(colors, &e),
                None => loading(colors, "Loading pull request…"),
            };
            return col().size_full().child(crumb).child(body).into_any_element();
        };
        let p = &pr.summary;
        let (glyph, fg, bg, state) = pr_glyph(colors, &p.state, p.is_draft);
        let open = p.state == "OPEN";
        let busy = self.busy.is_some();
        let method = settings::get(cx).merge_method;

        // Merge: the default method as the main button, the others in its menu.
        let merge = self.popover_button(
            "merge-menu",
            Button::new("merge-trigger")
                .label(method.short())
                .icon(Glyph::Merge)
                .primary()
                .small()
                .dropdown_caret(true)
                .loading(self.busy_with("Merging"))
                .disabled(busy || !open || p.is_draft),
            None,
            280.,
            move |_, weak, cx| {
                let prefs = settings::get(cx);
                let mut out = vec![Entry::Label(format!("Merge #{n}").into())];
                for m in MergeMethod::ALL {
                    let weak = weak.clone();
                    out.push(Entry::Item(
                        Item::new(m.title())
                            .icon(Glyph::Merge)
                            .checked(m == prefs.merge_method)
                            .on_click(move |w, cx| {
                                let _ = weak.update(cx, |s, cx| {
                                    let delete = settings::get(cx).delete_branch;
                                    s.confirm_pr(
                                        n,
                                        format!("{} #{n}?", m.short()),
                                        if delete {
                                            "The head branch will be deleted afterwards. This can’t be undone from here.".into()
                                        } else {
                                            "This can’t be undone from here.".into()
                                        },
                                        m.short(),
                                        PrAction::Merge { method: m, delete_branch: delete },
                                        w,
                                        cx,
                                    )
                                });
                            }),
                    ));
                }
                let weak = weak.clone();
                out.push(Entry::Separator);
                out.push(Entry::Item(
                    Item::new("Delete branch after merge")
                        .checked(prefs.delete_branch)
                        .keep_open()
                        .on_click(move |_, cx| {
                            let _ = weak.update(cx, |s, cx| s.update_settings(cx, |p| p.delete_branch = !p.delete_branch));
                        }),
                ));
                out
            },
            cx,
        );
        let review = self.popover_button(
            "review-menu",
            Button::new("review-trigger")
                .label("Review")
                .icon(Glyph::ListChecks)
                .outline()
                .small()
                .dropdown_caret(true)
                .disabled(busy || !open),
            None,
            300.,
            move |s, weak, cx| {
                let body = s
                    .current()
                    .map(|t| t.composer.read(cx).value().trim().to_string())
                    .unwrap_or_default();
                let (a, b) = (weak.clone(), weak.clone());
                let body2 = body.clone();
                vec![
                    Entry::Item(Item::new("Approve").icon(IconName::CircleCheck).on_click(move |_, cx| {
                        let _ = a.update(cx, |s, cx| s.pr_action(n, PrAction::Approve, cx));
                    })),
                    Entry::Item(
                        Item::new("Request changes")
                            .icon(IconName::TriangleAlert)
                            .detail(if body.is_empty() { "write a comment first" } else { "uses your comment" })
                            .disabled(body.is_empty())
                            .on_click(move |_, cx| {
                                let body = body2.clone();
                                let _ = b.update(cx, |s, cx| s.pr_action(n, PrAction::RequestChanges(body), cx));
                            }),
                    ),
                ]
            },
            cx,
        );
        let local = repo.as_ref().and_then(|r| r.path.clone());
        let url = pr.url.clone();
        let more = self.popover_button(
            "pr-more",
            icon_button("pr-more-trigger", IconName::Ellipsis, "More actions").disabled(busy),
            None,
            260.,
            move |s, weak, _| {
                let Some(t) = s.current() else { return vec![] };
                let Some(pr) = s.data(&t.repo).and_then(|d| d.pull.get(&n)).and_then(|l| l.data.as_ref()) else {
                    return vec![];
                };
                let p = &pr.summary;
                let mut out = vec![];
                if let Some(path) = local.clone() {
                    let w = weak.clone();
                    out.push(Entry::Item(Item::new("Check out locally").icon(Glyph::Branch).detail("gh pr checkout").on_click(move |_, cx| {
                        let path = path.clone();
                        let _ = w.update(cx, |s, cx| s.pr_action(n, PrAction::Checkout(path), cx));
                    })));
                }
                if p.state == "OPEN" {
                    let w = weak.clone();
                    let (title, action) = if p.is_draft {
                        ("Mark ready for review", PrAction::Ready)
                    } else {
                        ("Convert to draft", PrAction::Draft)
                    };
                    out.push(Entry::Item(Item::new(title).icon(Glyph::PullRequestDraft).on_click(move |_, cx| {
                        let action = action.clone();
                        let _ = w.update(cx, |s, cx| s.pr_action(n, action, cx));
                    })));
                    let w = weak.clone();
                    out.push(Entry::Item(Item::new("Close pull request").icon(Glyph::PullRequestClosed).on_click(move |win, cx| {
                        let _ = w.update(cx, |s, cx| {
                            s.confirm_pr(n, format!("Close #{n}?"), "You can reopen it later.".into(), "Close", PrAction::Close, win, cx)
                        });
                    })));
                } else if p.state == "CLOSED" {
                    let w = weak.clone();
                    out.push(Entry::Item(Item::new("Reopen pull request").icon(Glyph::PullRequest).on_click(move |_, cx| {
                        let _ = w.update(cx, |s, cx| s.pr_action(n, PrAction::Reopen, cx));
                    })));
                }
                out.push(Entry::Separator);
                let (u1, u2) = (pr.url.clone(), pr.url.clone());
                out.push(Entry::Item(Item::new("Copy link").icon(IconName::Copy).on_click(move |_, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(u1.clone()))
                })));
                out.push(Entry::Item(Item::new("Open on GitHub").icon(IconName::ExternalLink).on_click(move |_, cx| cx.open_url(&u2))));
                out
            },
            cx,
        );
        let _ = url;
        let head = col()
            .flex_shrink_0()
            .px_4()
            .pt_1()
            .pb_3()
            .gap_2()
            .child(
                row()
                    .gap_3()
                    .items_start()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(19.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .line_height(relative(1.3))
                            .child(format!("{}  ", p.title))
                            .child(div().text_color(rgb(colors.quiet)).font_weight(FontWeight::NORMAL).child(format!("#{n}"))),
                    )
                    .child(
                        row()
                            .gap_1()
                            .flex_shrink_0()
                            .child(review)
                            .child(merge)
                            .child(more),
                    ),
            )
            .child(
                row()
                    .gap_2()
                    .text_size(px(13.))
                    .text_color(rgb(colors.muted))
                    .child(
                        row()
                            .h(px(24.))
                            .px_2()
                            .gap(px(5.))
                            .rounded(px(12.))
                            .bg(rgb(bg))
                            .text_color(rgb(fg))
                            .font_weight(FontWeight::MEDIUM)
                            .text_size(px(12.))
                            .child(icon(glyph).size(px(13.)))
                            .child(state),
                    )
                    .child(self.avatar(Some(&p.author.login), &p.author.login, 18., cx))
                    .child(div().text_color(rgb(colors.text)).child(p.author.login.clone()))
                    .child(if pr.merged_at.is_some() { "merged" } else { "wants to merge" })
                    .child(format!("{} commit{} into", pr.commits.len(), if pr.commits.len() == 1 { "" } else { "s" }))
                    .child(chip(colors, mono.clone(), p.base_ref_name.clone()))
                    .child("from")
                    .child(chip(colors, mono.clone(), p.head_ref_name.clone())),
            );
        let rollup = gh::rollup(&p.status_check_rollup);
        let tabs = [
            (PrPage::Conversation, format!("Conversation  {}", pr.comments.len() + pr.reviews.iter().filter(|r| !r.body.is_empty()).count())),
            (PrPage::Commits, format!("Commits  {}", pr.commits.len())),
            (PrPage::Checks, format!("Checks  {}", p.status_check_rollup.len())),
            (PrPage::Files, format!("Files changed  {}", pr.changed_files)),
        ];
        let sub = row()
            .h(px(BAR))
            .flex_shrink_0()
            .px_3()
            .gap_2()
            .border_b_1()
            .border_color(rgb(colors.border))
            .child(segmented(colors, "pr-page", &tabs, t.pr_page, |s, page, _| {
                if let Some(t) = s.current_mut() {
                    t.pr_page = page;
                }
            }, cx))
            .when_some(rollup, |s, r| s.child(check_icon(colors, r)))
            .child(div().flex_1())
            .child(super::diff::counts(colors, p.additions as u32, p.deletions as u32));
        let body = match t.pr_page {
            PrPage::Conversation => self.conversation(pr, window, cx),
            PrPage::Commits => self.pr_commits(pr, cx),
            PrPage::Checks => self.pr_checks(pr, cx),
            PrPage::Files => {
                let l = self.data(&t.repo).and_then(|d| d.pull_diff.get(&n));
                match (l.and_then(|l| l.data.clone()), l.and_then(|l| l.error.clone())) {
                    (Some(diff), _) => h_resizable(SharedString::from(format!("pr-files-{}", t.id)))
                        .child(
                            resizable_panel()
                                .size(px(260.))
                                .size_range(px(180.)..px(500.))
                                .child(div().size_full().pt_1().border_r_1().border_color(rgb(colors.row_border)).child(self.diff_files(colors, &diff, t.diff_scroll.clone(), cx))),
                        )
                        .child(resizable_panel().child(self.diff_list(("pr-diff", t.id), diff, t.diff_scroll.clone(), cx)))
                        .into_any_element(),
                    (None, Some(e)) => failed(colors, &e).into_any_element(),
                    _ => loading(colors, "Loading diff…").into_any_element(),
                }
            }
        };
        col()
            .size_full()
            .bg(rgb(colors.background))
            .child(crumb)
            .child(head)
            .child(sub)
            .child(div().flex_1().min_h_0().child(body))
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn comment_card(&self, id: String, login: &str, verb: String, when: i64, body: &str, accent: Option<(u32, u32, &'static str)>, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let colors = palette(cx);
        row()
            .items_start()
            .gap_3()
            .child(self.avatar(Some(login), login, 28., cx))
            .child(
                card(colors)
                    .flex_1()
                    .min_w_0()
                    .child(
                        row()
                            .h(px(34.))
                            .px_3()
                            .gap_2()
                            .bg(rgb(colors.chrome))
                            .border_b_1()
                            .border_color(rgb(colors.row_border))
                            .text_size(px(12.5))
                            .child(div().font_weight(FontWeight::MEDIUM).child(login.to_string()))
                            .child(div().text_color(rgb(colors.muted)).child(format!("{verb} {}", self.when(when, cx))))
                            .when_some(accent, |s, (fg, bg, text)| s.child(pill(text, fg, bg))),
                    )
                    .when(!body.trim().is_empty(), |s| s.child(div().px_4().py_3().child(markdown(id, body, window, cx))))
                    .when(body.trim().is_empty(), |s| s.child(div().px_4().py_3().text_color(rgb(colors.muted)).child("No description provided."))),
            )
    }

    fn conversation(&self, pr: &gh::PullDetail, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let colors = palette(cx);
        let Some(t) = self.current() else { return div().into_any_element() };
        let n = pr.summary.number;
        let mut items: Vec<(i64, Div)> = vec![];
        for (i, c) in pr.comments.iter().enumerate() {
            let at = util::parse_time(&c.created_at);
            items.push((at, self.comment_card(format!("pr-{n}-c{i}"), &c.author.login, "commented".into(), at, &c.body, None, window, cx)));
        }
        for (i, r) in pr.reviews.iter().enumerate() {
            let at = util::parse_time(r.submitted_at.as_deref().unwrap_or(""));
            let accent = match r.state.as_str() {
                "APPROVED" => Some((colors.green, colors.green_tint, "Approved")),
                "CHANGES_REQUESTED" => Some((colors.red, colors.red_tint, "Changes requested")),
                "DISMISSED" => Some((colors.muted, colors.hover, "Dismissed")),
                _ => None,
            };
            if r.body.trim().is_empty() {
                // A bare approval reads as a one-line event.
                let Some((fg, _, text)) = accent else { continue };
                items.push((
                    at,
                    row()
                        .gap_3()
                        .pl(px(6.))
                        .text_size(px(12.5))
                        .text_color(rgb(colors.muted))
                        .child(icon(if r.state == "APPROVED" { IconName::CircleCheck } else { IconName::TriangleAlert }).text_color(rgb(fg)))
                        .child(self.avatar(Some(&r.author.login), &r.author.login, 16., cx))
                        .child(div().text_color(rgb(colors.text)).child(r.author.login.clone()))
                        .child(format!("{} {}", text.to_lowercase(), self.when(at, cx))),
                ));
            } else {
                items.push((at, self.comment_card(format!("pr-{n}-r{i}"), &r.author.login, "reviewed".into(), at, &r.body, accent, window, cx)));
            }
        }
        items.sort_by_key(|(at, _)| *at);
        let p = &pr.summary;
        let mut thread = col().gap_4().child(self.comment_card(
            format!("pr-{n}-body"),
            &p.author.login,
            "opened".into(),
            util::parse_time(&p.created_at),
            &pr.body,
            None,
            window,
            cx,
        ));
        for (_, item) in items {
            thread = thread.child(item);
        }
        // Merge readiness, as GitHub reports it.
        let (glyph, color, title, detail) = match (p.state.as_str(), pr.mergeable.as_str(), pr.merge_state_status.as_str()) {
            ("MERGED", _, _) => (Glyph::Merge.into(), colors.purple, "Merged", "This pull request has been merged.".to_string()),
            ("CLOSED", _, _) => (Glyph::PullRequestClosed.into(), colors.red, "Closed", "Closed without merging.".to_string()),
            (_, "CONFLICTING", _) => (IconName::TriangleAlert.into(), colors.red, "Conflicts", "Resolve conflicts with the base branch before merging.".into()),
            (_, _, "BLOCKED") => (Glyph::Lock.into(), colors.amber, "Blocked", "Required reviews or checks haven’t passed yet.".into()),
            (_, _, "BEHIND") => (Glyph::Fetch.into(), colors.amber, "Behind base", "The head branch is out of date with the base branch.".into()),
            (_, _, "UNSTABLE") => (IconName::TriangleAlert.into(), colors.amber, "Checks failing", "Some non-required checks are failing.".into()),
            (_, _, "DRAFT") => (Glyph::PullRequestDraft.into(), colors.muted, "Draft", "Mark it ready for review to merge.".into()),
            (_, "MERGEABLE", _) => (IconName::CircleCheck.into(), colors.green, "Ready to merge", "No conflicts with the base branch.".into()),
            _ => (Glyph::CircleDashed.into(), colors.muted, "Checking mergeability", "GitHub is still computing whether this can merge.".into()),
        };
        let status: gpui_component::Icon = glyph;
        thread = thread.child(
            row()
                .gap_3()
                .p_3()
                .rounded(px(8.))
                .border_1()
                .border_color(rgb(colors.border))
                .child(status.size(px(18.)).text_color(rgb(color)))
                .child(col().gap(px(2.)).child(div().font_weight(FontWeight::MEDIUM).child(title)).child(label(colors, detail))),
        );
        let open = p.state == "OPEN";
        let composer = col()
            .gap_2()
            .child(Input::new(&t.composer))
            .child(
                row()
                    .gap_2()
                    .child(label(colors, "Markdown · ⌘ Return to send"))
                    .child(div().flex_1())
                    .child(
                        Button::new("send-comment")
                            .label("Comment")
                            .icon(Glyph::Message)
                            .primary()
                            .small()
                            .loading(self.busy_with("Commenting"))
                            .disabled(self.busy.is_some())
                            .on_click(cx.listener(|s, _, w, cx| s.submit(&Submit, w, cx))),
                    ),
            );
        let _ = open;
        div()
            .id(("conversation", t.id))
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&t.page_scroll)
            .child(
                col()
                    .p_4()
                    .gap_4()
                    .max_w(px(900.))
                    .w_full()
                    .mx_auto()
                    .child(thread)
                    .child(row().items_start().gap_3().child(self.avatar(self.user.data.as_ref().map(|u| u.login.as_str()), "you", 28., cx)).child(div().flex_1().child(composer))),
            )
            .into_any_element()
    }

    fn pr_commits(&self, pr: &gh::PullDetail, cx: &mut Context<Self>) -> AnyElement {
        let colors = palette(cx);
        let mono = settings::mono(cx);
        if pr.commits.is_empty() {
            return empty(colors, Glyph::Commit, "No commits", "").into_any_element();
        }
        let mut list = card(colors);
        for (i, c) in pr.commits.iter().enumerate() {
            let author = c.authors.first().cloned().unwrap_or_default();
            let name = author.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| author.login.clone());
            let sha = c.oid.clone();
            list = list.child(
                row()
                    .h(px(40.))
                    .px_3()
                    .gap_3()
                    .when(i > 0, |s| s.border_t_1().border_color(rgb(colors.row_border)))
                    .child(icon(Glyph::Commit).text_color(rgb(colors.muted)))
                    .child(self.avatar(Some(&author.login), &name, 18., cx))
                    .child(div().flex_1().min_w_0().truncate().child(c.message_headline.clone()))
                    .child(label(colors, name))
                    .child(label(colors, self.when(util::parse_time(&c.authored_date), cx)))
                    .child(
                        Button::new(SharedString::from(format!("pr-commit-{i}")))
                            .label(util::short_sha(&c.oid).to_string())
                            .ghost()
                            .xsmall()
                            .font_family(mono.clone())
                            .tooltip("Copy SHA")
                            .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(sha.clone()))),
                    ),
            );
        }
        div()
            .id("pr-commits")
            .size_full()
            .overflow_y_scroll()
            .child(col().p_4().max_w(px(900.)).w_full().mx_auto().child(list))
            .into_any_element()
    }

    fn pr_checks(&self, pr: &gh::PullDetail, cx: &mut Context<Self>) -> AnyElement {
        let colors = palette(cx);
        let checks = &pr.summary.status_check_rollup;
        if checks.is_empty() {
            return empty(colors, Glyph::ListChecks, "No checks", "No status checks were reported for the head commit.").into_any_element();
        }
        let mut sorted = checks.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|c| match c.outcome() {
            gh::CheckState::Fail => 0,
            gh::CheckState::Pending => 1,
            gh::CheckState::Pass => 2,
            gh::CheckState::Skipped => 3,
        });
        let mut list = card(colors);
        for (i, c) in sorted.into_iter().enumerate() {
            let url = c.url();
            list = list.child(
                row()
                    .id(("check", i))
                    .h(px(38.))
                    .px_3()
                    .gap_3()
                    .when(i > 0, |s| s.border_t_1().border_color(rgb(colors.row_border)))
                    .when(url.is_some(), |s| s.cursor_pointer().hover(|s| s.bg(rgb(colors.hover))))
                    .child(check_icon(colors, c.outcome()))
                    .child(div().flex_1().min_w_0().truncate().child(c.title()))
                    .when_some(c.workflow_name.clone().filter(|w| !w.is_empty()), |s, w| s.child(label(colors, w)))
                    .when(url.is_some(), |s| s.child(icon(IconName::ExternalLink).size(px(12.)).text_color(rgb(colors.quiet))))
                    .when_some(url, |s, url| s.on_click(move |_, _, cx| cx.open_url(&url))),
            );
        }
        div()
            .id("pr-checks")
            .size_full()
            .overflow_y_scroll()
            .child(col().p_4().max_w(px(900.)).w_full().mx_auto().child(list))
            .into_any_element()
    }
}

fn label_pill(colors: Palette, l: &gh::Label) -> Div {
    let hex = u32::from_str_radix(l.color.trim_start_matches('#'), 16).unwrap_or(colors.quiet);
    row()
        .flex_shrink_0()
        .h(px(18.))
        .px(px(7.))
        .gap(px(5.))
        .rounded(px(9.))
        .border_1()
        .border_color(rgb(colors.border))
        .text_size(px(11.))
        .text_color(rgb(colors.muted))
        .child(div().size(px(7.)).rounded_full().bg(rgb(hex)))
        .child(l.name.clone())
}
fn review_pill(colors: Palette, decision: &str) -> Option<Div> {
    Some(match decision {
        "APPROVED" => pill("Approved", colors.green, colors.green_tint),
        "CHANGES_REQUESTED" => pill("Changes requested", colors.red, colors.red_tint),
        "REVIEW_REQUIRED" => pill("Review required", colors.amber, colors.amber_tint),
        _ => return None,
    })
}
