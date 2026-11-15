//! A small GitHub-flavoured Markdown renderer for READMEs, PR threads and release
//! notes. It draws inline code in the monospace font (GPUI highlights can't switch
//! fonts, so text is built from runs) and spaces blocks itself.
use super::ui::{col, row};
use crate::theme::Palette;
use gpui::prelude::FluentBuilder;
use gpui::*;
use markdown::mdast::{AlignKind, Node};
use std::{cell::Cell, ops::Range};

/// How source newlines inside a paragraph read: Markdown files treat them as
/// spaces; GitHub comments and release notes treat them as line breaks.
#[derive(Clone, Copy, PartialEq)]
pub enum Breaks {
    Soft,
    Hard,
}

struct Ctx {
    id: SharedString,
    next: Cell<usize>,
    colors: Palette,
    mono: SharedString,
    mono_size: Pixels,
    breaks: Breaks,
}
impl Ctx {
    fn id(&self, what: &str) -> ElementId {
        let n = self.next.get();
        self.next.set(n + 1);
        ElementId::Name(format!("{}/{what}{n}", self.id).into())
    }
}

#[derive(Clone, Default)]
struct Style {
    bold: bool,
    italic: bool,
    strike: bool,
    code: bool,
    link: Option<String>,
}

/// One paragraph's worth of styled text, with link ranges for click handling.
#[derive(Default)]
struct Inline {
    text: String,
    runs: Vec<(usize, Style)>,
    links: Vec<(Range<usize>, String)>,
}
impl Inline {
    fn push(&mut self, s: &str, style: &Style) {
        if s.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(s);
        self.runs.push((s.len(), style.clone()));
        if let Some(url) = &style.link {
            match self.links.last_mut() {
                Some((r, u)) if u == url && r.end == start => r.end = self.text.len(),
                _ => self.links.push((start..self.text.len(), url.clone())),
            }
        }
    }
    fn walk(&mut self, nodes: &[Node], style: &Style, breaks: Breaks) {
        for n in nodes {
            match n {
                Node::Text(t) => match breaks {
                    Breaks::Soft => self.push(&t.value.replace('\n', " "), style),
                    Breaks::Hard => self.push(&t.value, style),
                },
                Node::Break(_) => self.push("\n", style),
                Node::InlineCode(c) => self.push(
                    &c.value,
                    &Style {
                        code: true,
                        ..style.clone()
                    },
                ),
                Node::Strong(s) => self.walk(
                    &s.children,
                    &Style {
                        bold: true,
                        ..style.clone()
                    },
                    breaks,
                ),
                Node::Emphasis(e) => self.walk(
                    &e.children,
                    &Style {
                        italic: true,
                        ..style.clone()
                    },
                    breaks,
                ),
                Node::Delete(d) => self.walk(
                    &d.children,
                    &Style {
                        strike: true,
                        ..style.clone()
                    },
                    breaks,
                ),
                Node::Link(l) => self.walk(
                    &l.children,
                    &Style {
                        link: Some(l.url.clone()),
                        ..style.clone()
                    },
                    breaks,
                ),
                Node::LinkReference(l) => self.walk(&l.children, style, breaks),
                Node::Image(i) => self.push(
                    &format!("[{}]", if i.alt.is_empty() { "image" } else { &i.alt }),
                    &Style {
                        link: Some(i.url.clone()),
                        ..style.clone()
                    },
                ),
                Node::Html(h) => {
                    let lower = h.value.to_ascii_lowercase();
                    if lower.starts_with("<br") {
                        self.push("\n", style);
                    }
                }
                other => {
                    if let Some(children) = other.children() {
                        self.walk(children, style, breaks);
                    }
                }
            }
        }
    }
    fn element(self, cx: &Ctx) -> AnyElement {
        let colors = cx.colors;
        let runs = self
            .runs
            .iter()
            .map(|(len, s)| TextRun {
                len: *len,
                font: Font {
                    family: if s.code { cx.mono.clone() } else { ".SystemUIFont".into() },
                    features: FontFeatures::default(),
                    fallbacks: None,
                    weight: if s.bold { FontWeight::SEMIBOLD } else { FontWeight::NORMAL },
                    style: if s.italic { FontStyle::Italic } else { FontStyle::Normal },
                },
                color: rgb(if s.link.is_some() {
                    colors.blue
                } else {
                    colors.text
                })
                .into(),
                background_color: s.code.then(|| rgb(colors.hover).into()),
                underline: s.link.as_ref().map(|_| UnderlineStyle {
                    thickness: px(1.),
                    color: Some(Hsla::from(rgb(colors.blue)).opacity(0.4)),
                    wavy: false,
                }),
                strikethrough: s.strike.then(|| StrikethroughStyle {
                    thickness: px(1.),
                    color: None,
                }),
            })
            .collect::<Vec<_>>();
        let styled = StyledText::new(self.text).with_runs(runs);
        if self.links.is_empty() {
            return styled.into_any_element();
        }
        let (ranges, urls): (Vec<_>, Vec<_>) = self.links.into_iter().unzip();
        InteractiveText::new(cx.id("text"), styled)
            .on_click(ranges, move |ix, _, cx| {
                if let Some(url) = urls.get(ix)
                    && (url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:"))
                {
                    cx.open_url(url);
                }
            })
            .into_any_element()
    }
}

fn inline(nodes: &[Node], cx: &Ctx) -> AnyElement {
    let mut i = Inline::default();
    i.walk(nodes, &Style::default(), cx.breaks);
    i.element(cx)
}

/// Text inside HTML blocks (e.g. `<details>`), without the tags.
fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for c in html.chars() {
        match c {
            '<' => tag = true,
            '>' => tag = false,
            c if !tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn blocks(nodes: &[Node], cx: &Ctx) -> Vec<AnyElement> {
    nodes.iter().filter_map(|n| block(n, cx)).collect()
}

fn block(node: &Node, cx: &Ctx) -> Option<AnyElement> {
    let colors = cx.colors;
    Some(match node {
        Node::Paragraph(p) => div().w_full().line_height(relative(1.55)).child(inline(&p.children, cx)).into_any_element(),
        Node::Heading(h) => {
            let size = match h.depth {
                1 => 20.,
                2 => 17.,
                3 => 15.,
                _ => 14.,
            };
            div()
                .w_full()
                .pt(px(if h.depth <= 2 { 6. } else { 2. }))
                .text_size(px(size))
                .font_weight(FontWeight::SEMIBOLD)
                .when(h.depth <= 2, |d| d.pb(px(6.)).border_b_1().border_color(rgb(colors.row_border)))
                .child(inline(&h.children, cx))
                .into_any_element()
        }
        Node::List(l) => {
            let start = l.start.unwrap_or(1);
            let mut list = col().w_full().gap(px(if l.spread { 8. } else { 3. }));
            for (i, item) in l.children.iter().enumerate() {
                let Node::ListItem(item) = item else { continue };
                let marker = match item.checked {
                    Some(done) => div()
                        .mt(px(3.))
                        .size(px(13.))
                        .rounded(px(3.))
                        .border_1()
                        .border_color(rgb(if done { colors.green } else { colors.border }))
                        .when(done, |d| d.bg(rgb(colors.green)))
                        .into_any_element(),
                    None if l.ordered => div()
                        .text_color(rgb(colors.muted))
                        .child(format!("{}.", start as usize + i))
                        .into_any_element(),
                    None => div().text_color(rgb(colors.muted)).child("•").into_any_element(),
                };
                list = list.child(
                    div()
                        .flex()
                        .items_start()
                        .gap_2()
                        .child(div().w(px(18.)).flex_shrink_0().flex().justify_end().line_height(relative(1.55)).child(marker))
                        .child(col().flex_1().min_w_0().gap(px(6.)).children(blocks(&item.children, cx))),
                );
            }
            list.into_any_element()
        }
        Node::Code(c) => {
            let code = c.value.clone();
            div()
                .relative()
                .w_full()
                .group("code")
                .child(
                    div()
                        .id(cx.id("code"))
                        .w_full()
                        .overflow_x_scroll()
                        // A sideways swipe scrolls the code; the page keeps vertical ones.
                        .map(|mut d| {
                            d.style().restrict_scroll_to_axis = Some(true);
                            d
                        })
                        .p(px(10.))
                        .rounded(px(6.))
                        .bg(rgb(colors.panel))
                        .border_1()
                        .border_color(rgb(colors.row_border))
                        .font_family(cx.mono.clone())
                        .text_size(cx.mono_size)
                        .line_height(relative(1.5))
                        .children(c.value.lines().map(|l| div().whitespace_nowrap().min_h(px(4.)).child(l.replace('\t', "    ")))),
                )
                .child(
                    div()
                        .id(cx.id("copy"))
                        .absolute()
                        .top(px(6.))
                        .right(px(6.))
                        .px(px(6.))
                        .py(px(2.))
                        .rounded(px(4.))
                        .text_size(px(11.))
                        .text_color(rgb(colors.muted))
                        .bg(rgb(colors.background))
                        .border_1()
                        .border_color(rgb(colors.border))
                        .cursor_pointer()
                        .opacity(0.)
                        .group_hover("code", |d| d.opacity(1.))
                        .child("Copy")
                        .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(code.clone()))),
                )
                .into_any_element()
        }
        Node::Blockquote(b) => col()
            .w_full()
            .gap(px(8.))
            .pl_3()
            .border_l_2()
            .border_color(rgb(colors.border))
            .text_color(rgb(colors.muted))
            .children(blocks(&b.children, cx))
            .into_any_element(),
        Node::ThematicBreak(_) => div().w_full().h(px(1.)).bg(rgb(colors.border)).into_any_element(),
        Node::Table(t) => {
            let mut table = col()
                .w_full()
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(colors.border))
                .overflow_hidden();
            for (r, tr) in t.children.iter().enumerate() {
                let Node::TableRow(tr) = tr else { continue };
                let mut line = row()
                    .w_full()
                    .when(r > 0, |d| d.border_t_1().border_color(rgb(colors.row_border)))
                    .when(r == 0, |d| d.bg(rgb(colors.chrome)).font_weight(FontWeight::SEMIBOLD));
                for (c, cell) in tr.children.iter().enumerate() {
                    let Node::TableCell(cell) = cell else { continue };
                    let align = t.align.get(c).copied().unwrap_or(AlignKind::None);
                    line = line.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .px_2()
                            .py(px(5.))
                            .when(c > 0, |d| d.border_l_1().border_color(rgb(colors.row_border)))
                            .flex()
                            .map(|d| match align {
                                AlignKind::Center => d.justify_center(),
                                AlignKind::Right => d.justify_end(),
                                _ => d,
                            })
                            .child(inline(&cell.children, cx)),
                    );
                }
                table = table.child(line);
            }
            table.into_any_element()
        }
        Node::Html(h) => {
            let text = strip_tags(&h.value);
            if text.is_empty() {
                return None;
            }
            div().w_full().text_color(rgb(colors.muted)).child(text).into_any_element()
        }
        Node::Root(r) => col().w_full().gap(px(10.)).children(blocks(&r.children, cx)).into_any_element(),
        Node::Definition(_) | Node::FootnoteDefinition(_) | Node::Yaml(_) | Node::Toml(_) => return None,
        other => {
            let children = other.children()?;
            div().w_full().child(inline(children, cx)).into_any_element()
        }
    })
}

/// Render Markdown text. Unparsable input falls back to plain text.
pub fn render(id: SharedString, text: &str, breaks: Breaks, colors: Palette, mono: SharedString, mono_size: Pixels) -> AnyElement {
    let cx = Ctx {
        id,
        next: Cell::new(0),
        colors,
        mono,
        mono_size,
        breaks,
    };
    match markdown::to_mdast(text, &markdown::ParseOptions::gfm()) {
        Ok(Node::Root(root)) => col()
            .w_full()
            .gap(px(10.))
            .text_size(px(13.))
            .children(blocks(&root.children, &cx))
            .into_any_element(),
        _ => div().text_size(px(13.)).child(text.to_string()).into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: gpui's own `test` attribute would shadow the built-in one.
    use super::{Breaks, Inline, Node, Style, strip_tags};
    fn flatten(src: &str, breaks: Breaks) -> Inline {
        let Ok(Node::Root(root)) = markdown::to_mdast(src, &markdown::ParseOptions::gfm()) else {
            panic!()
        };
        let Node::Paragraph(p) = &root.children[0] else { panic!() };
        let mut i = Inline::default();
        i.walk(&p.children, &Style::default(), breaks);
        i
    }
    #[test]
    fn styles_inline_code_and_links() {
        let i = flatten("Run `cargo test` or see [the docs](https://x.dev) now", Breaks::Soft);
        assert_eq!(i.text, "Run cargo test or see the docs now");
        assert!(i.runs.iter().any(|(len, s)| s.code && *len == "cargo test".len()));
        assert_eq!(i.links, vec![(22..30, "https://x.dev".to_string())]);
    }
    #[test]
    fn soft_and_hard_breaks() {
        assert_eq!(flatten("one\ntwo", Breaks::Soft).text, "one two");
        assert_eq!(flatten("one\ntwo", Breaks::Hard).text, "one\ntwo");
    }
    #[test]
    fn strips_html_tags() {
        assert_eq!(strip_tags("<details><summary>More</summary> info</details>"), "More info");
    }
}
