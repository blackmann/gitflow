//! A Zed-style popover menu: optional search, section labels, roomy rounded rows,
//! trailing check marks, keyboard navigation and outside-click dismissal.
//!
//! Entries are rebuilt from a closure on every render, so check marks always
//! reflect live workspace state (multi-select menus can stay open while toggling).
use crate::theme::palette;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{
    Icon, IconName, Sizable,
    input::{self, Input, InputState},
    scroll::{Scrollbar, ScrollbarAxis},
};
use std::rc::Rc;

type Handler = Rc<dyn Fn(&mut Window, &mut App)>;

pub struct Item {
    label: SharedString,
    detail: Option<SharedString>,
    icon: Option<Icon>,
    image: Option<std::path::PathBuf>,
    checked: bool,
    disabled: bool,
    keep_open: bool,
    font: Option<SharedString>,
    handler: Option<Handler>,
}
impl Item {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            detail: None,
            icon: None,
            image: None,
            checked: false,
            disabled: false,
            keep_open: false,
            font: None,
            handler: None,
        }
    }
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }
    /// Show a picture, such as an app's icon, where the icon goes.
    pub fn image(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.image = Some(path.into());
        self
    }
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    /// Keep the menu open after activation, for multi-select lists.
    pub fn keep_open(mut self) -> Self {
        self.keep_open = true;
        self
    }
    /// Render the label in a specific font family (font pickers preview themselves).
    pub fn font(mut self, font: impl Into<SharedString>) -> Self {
        self.font = Some(font.into());
        self
    }
    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.handler = Some(Rc::new(f));
        self
    }
}

// Entries are rebuilt per render and short-lived; boxing items would only add noise.
#[allow(clippy::large_enum_variant)]
pub enum Entry {
    Label(SharedString),
    Separator,
    Item(Item),
}

pub type Builder = Rc<dyn Fn(&App) -> Vec<Entry>>;

pub struct Menu {
    focus: FocusHandle,
    search: Option<Entity<InputState>>,
    build: Builder,
    highlighted: Option<usize>,
    width: Pixels,
    scroll: ScrollHandle,
}
impl EventEmitter<DismissEvent> for Menu {}
impl Focusable for Menu {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Menu {
    pub fn new(
        build: Builder,
        search: Option<&str>,
        width: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = search.map(|placeholder| {
            let input =
                cx.new(|cx| InputState::new(window, cx).placeholder(placeholder.to_string()));
            // Subscribe to edits only: the input also notifies on every caret blink,
            // which would otherwise clear the hover highlight.
            cx.subscribe(&input, |m: &mut Menu, _, e: &input::InputEvent, cx| {
                if matches!(e, input::InputEvent::Change) {
                    m.highlighted = None;
                    cx.notify();
                }
            })
            .detach();
            input
        });
        Self {
            focus: cx.focus_handle(),
            search,
            build,
            highlighted: None,
            width,
            scroll: ScrollHandle::new(),
        }
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        match &self.search {
            Some(s) => s.update(cx, |s, cx| s.focus(window, cx)),
            None => self.focus.focus(window),
        }
    }
    /// While searching, show only matching items, without section chrome.
    fn visible(&self, cx: &App) -> Vec<Entry> {
        let entries = (self.build)(cx);
        let query = self
            .search
            .as_ref()
            .map(|s| s.read(cx).value().trim().to_lowercase())
            .unwrap_or_default();
        if query.is_empty() {
            return entries;
        }
        entries
            .into_iter()
            .filter(|e| match e {
                Entry::Item(i) => {
                    i.label.to_lowercase().contains(&query)
                        || i.detail
                            .as_ref()
                            .is_some_and(|d| d.to_lowercase().contains(&query))
                }
                _ => false,
            })
            .collect()
    }
    fn selectable(entries: &[Entry]) -> Vec<usize> {
        entries
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e, Entry::Item(i) if !i.disabled))
            .map(|(i, _)| i)
            .collect()
    }
    fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let entries = self.visible(cx);
        let Some(Entry::Item(item)) = entries.get(index) else {
            return;
        };
        if item.disabled {
            return;
        }
        if let Some(handler) = item.handler.clone() {
            handler(window, cx);
        }
        if item.keep_open {
            cx.notify();
        } else {
            cx.emit(DismissEvent);
        }
    }
    fn step(&mut self, down: bool, cx: &mut Context<Self>) {
        let entries = self.visible(cx);
        let items = Self::selectable(&entries);
        if items.is_empty() {
            return;
        }
        let pos = self
            .highlighted
            .and_then(|h| items.iter().position(|i| *i == h));
        let next = match (pos, down) {
            (None, true) => 0,
            (None, false) => items.len() - 1,
            (Some(p), true) => (p + 1) % items.len(),
            (Some(p), false) => (p + items.len() - 1) % items.len(),
        };
        self.highlighted = Some(items[next]);
        self.scroll.scroll_to_item(items[next]);
        cx.notify();
    }
    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let first = Self::selectable(&self.visible(cx)).first().copied();
        if let Some(i) = self.highlighted.or(first) {
            self.activate(i, window, cx);
        }
    }
    /// Without a search field the menu itself holds focus and sees raw keys.
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "down" => self.step(true, cx),
            "up" => self.step(false, cx),
            "enter" => self.confirm(window, cx),
            "escape" => cx.emit(DismissEvent),
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Render for Menu {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = palette(cx);
        let entries = self.visible(cx);
        let searching = self
            .search
            .as_ref()
            .is_some_and(|s| !s.read(cx).value().trim().is_empty());
        let mut list = div()
            .id("menu-list")
            .track_scroll(&self.scroll)
            .flex()
            .flex_col()
            .max_h(px(360.))
            .overflow_y_scroll()
            .p(px(4.));
        let empty = entries.is_empty();
        for (i, entry) in entries.into_iter().enumerate() {
            list = list.child(match entry {
                Entry::Label(text) => div()
                    .px(px(8.))
                    .pt(px(if i == 0 { 2. } else { 6. }))
                    .pb(px(3.))
                    .text_size(px(12.))
                    .text_color(rgb(colors.muted))
                    .child(text)
                    .into_any_element(),
                Entry::Separator => div()
                    .my(px(4.))
                    .mx(px(-4.))
                    .h(px(1.))
                    .bg(rgb(colors.popover_border))
                    .into_any_element(),
                Entry::Item(item) => {
                    let highlighted = self.highlighted == Some(i);
                    div()
                        .id(("menu-item", i))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .h(px(28.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .text_size(px(13.))
                        .when(item.disabled, |s| s.text_color(rgb(colors.quiet)))
                        .when(!item.disabled, |s| {
                            s.cursor_pointer()
                                .when(highlighted, |s| s.bg(rgb(colors.popover_hover)))
                                .on_mouse_move(cx.listener(move |m, _, _, cx| {
                                    if m.highlighted != Some(i) {
                                        m.highlighted = Some(i);
                                        cx.notify();
                                    }
                                }))
                                .on_click(cx.listener(move |m, _, w, cx| m.activate(i, w, cx)))
                        })
                        .when_some(item.image, |s, path| s.child(img(path).flex_shrink_0().size(px(16.))))
                        .when_some(item.icon, |s, icon| {
                            s.child(icon.size(px(14.)).text_color(rgb(colors.muted)))
                        })
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .when_some(item.font, |s, f| s.font_family(f))
                                .child(item.label),
                        )
                        .when(item.checked, |s| {
                            s.child(
                                Icon::new(IconName::Check)
                                    .size(px(14.))
                                    .text_color(rgb(colors.blue)),
                            )
                        })
                        .child(div().flex_1())
                        .when_some(item.detail, |s, d| {
                            s.child(
                                div()
                                    .flex_shrink_0()
                                    .text_size(px(12.))
                                    .text_color(rgb(colors.muted))
                                    .child(d),
                            )
                        })
                        .into_any_element()
                }
            });
        }
        if empty {
            list = list.child(
                div()
                    .px(px(8.))
                    .py(px(6.))
                    .text_color(rgb(colors.muted))
                    .child(if searching {
                        "No matches"
                    } else {
                        "Nothing here yet"
                    }),
            );
        }
        div()
            .track_focus(&self.focus)
            .key_context("GitflowMenu")
            .capture_key_down(cx.listener(Self::key))
            .capture_action(cx.listener(|m, _: &input::MoveDown, _, cx| {
                cx.stop_propagation();
                m.step(true, cx)
            }))
            .capture_action(cx.listener(|m, _: &input::MoveUp, _, cx| {
                cx.stop_propagation();
                m.step(false, cx)
            }))
            .capture_action(cx.listener(|m, _: &input::Enter, w, cx| {
                cx.stop_propagation();
                m.confirm(w, cx)
            }))
            .capture_action(cx.listener(|_, _: &input::Escape, _, cx| {
                cx.stop_propagation();
                cx.emit(DismissEvent)
            }))
            .on_mouse_down_out(cx.listener(|_, _, _, cx| cx.emit(DismissEvent)))
            .occlude()
            .w(self.width)
            .flex()
            .flex_col()
            .bg(rgb(colors.popover))
            .border_1()
            .border_color(rgb(colors.popover_border))
            .rounded(px(10.))
            .shadow_lg()
            .overflow_hidden()
            .text_color(rgb(colors.text))
            .font_family(".SystemUIFont")
            .when_some(self.search.clone(), |s, search| {
                s.child(
                    div()
                        .h(px(38.))
                        .px(px(6.))
                        .flex()
                        .items_center()
                        .border_b_1()
                        .border_color(rgb(colors.popover_border))
                        .child(Input::new(&search).appearance(false).small()),
                )
            })
            .child(
                div().relative().child(list).child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .child(Scrollbar::new(&self.scroll).id("menu-scrollbar").axis(ScrollbarAxis::Vertical)),
                ),
            )
    }
}
