//! Shared semantic colors, after CloudWatch v2 (and Zed's One themes): quiet chrome
//! frames a brighter content surface, and tints carry state so color is never the
//! only signal.
use crate::settings::{self, ThemePref};
use gpui::*;
use gpui_component::{Theme, ThemeMode};

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: u32,
    pub chrome: u32,
    pub panel: u32,
    pub border: u32,
    pub row_border: u32,
    pub text: u32,
    pub muted: u32,
    pub quiet: u32,
    pub blue: u32,
    pub green: u32,
    pub amber: u32,
    pub red: u32,
    pub purple: u32,
    pub selected: u32,
    pub hover: u32,
    pub scrim: u32,
    pub primary_text: u32,
    pub blue_tint: u32,
    pub green_tint: u32,
    pub amber_tint: u32,
    pub red_tint: u32,
    pub purple_tint: u32,
    pub pink: u32,
    pub pink_tint: u32,
    pub popover: u32,
    pub popover_hover: u32,
    pub popover_border: u32,
    /// Branch colors for the commit graph, cycled per lane.
    pub lanes: [u32; 8],
}
impl Global for Palette {}

impl Palette {
    fn new(dark: bool) -> Self {
        if dark {
            Self {
                background: 0x24272d,
                chrome: 0x2b2e35,
                panel: 0x2f333a,
                border: 0x3a3e47,
                row_border: 0x31353c,
                text: 0xdcdfe4,
                muted: 0x959ba6,
                quiet: 0x6c727d,
                blue: 0x74ade8,
                green: 0x98c379,
                amber: 0xe5c07b,
                red: 0xe06c75,
                purple: 0xc678dd,
                selected: 0x2f3d52,
                hover: 0x33373f,
                scrim: 0x0000005c,
                primary_text: 0x1e2127,
                blue_tint: 0x2a3647,
                green_tint: 0x2c3a2e,
                amber_tint: 0x3b3427,
                red_tint: 0x3e2b2f,
                purple_tint: 0x382d42,
                pink: 0xe58fc0,
                pink_tint: 0x3d2a36,
                popover: 0x2e3138,
                popover_hover: 0x3b3f48,
                popover_border: 0x434852,
                lanes: [
                    0x74ade8, 0xc678dd, 0x98c379, 0xe5c07b, 0xe06c75, 0x56b6c2, 0xd19a66, 0xa0a8f0,
                ],
            }
        } else {
            Self {
                background: 0xfcfcfc,
                chrome: 0xf3f3f4,
                panel: 0xf6f6f7,
                border: 0xe0e0e3,
                row_border: 0xededef,
                text: 0x2c2d31,
                muted: 0x76787f,
                quiet: 0xa2a4aa,
                blue: 0x2f6fde,
                green: 0x3b7d3e,
                amber: 0xa86a00,
                red: 0xc2414b,
                purple: 0x8a3fc7,
                selected: 0xe2e8f1,
                hover: 0xebebed,
                scrim: 0x0000001f,
                primary_text: 0xffffff,
                blue_tint: 0xe7effc,
                green_tint: 0xe8f3e9,
                amber_tint: 0xfbf3e3,
                red_tint: 0xfbe9ea,
                purple_tint: 0xf3eafb,
                pink: 0xc23b85,
                pink_tint: 0xfbe8f2,
                popover: 0xf5f5f6,
                popover_hover: 0xe5e5e8,
                popover_border: 0xd8d8dc,
                lanes: [
                    0x2f6fde, 0x8a3fc7, 0x3b8a3e, 0xb07800, 0xc2414b, 0x1f8a99, 0xc0601a, 0x5b62c9,
                ],
            }
        }
    }
    pub fn lane(&self, i: usize) -> u32 {
        self.lanes[i % self.lanes.len()]
    }
}

pub fn palette(cx: &App) -> Palette {
    *cx.global::<Palette>()
}

/// The appearance to render with: the system's, unless a preference pins one.
fn effective(system: WindowAppearance, cx: &App) -> WindowAppearance {
    match settings::get(cx).theme {
        ThemePref::System => system,
        ThemePref::Light => WindowAppearance::Light,
        ThemePref::Dark => WindowAppearance::Dark,
    }
}

/// Read the actual window appearance on startup and whenever macOS changes it.
pub fn sync(window: &mut Window, cx: &mut App) {
    apply(window.appearance(), cx);
    window.refresh();
}

pub fn init(cx: &mut App) {
    cx.set_global(settings::Settings::load());
    apply(cx.window_appearance(), cx);
}

/// Re-apply after a preference change, e.g. a new monospace font or theme.
pub fn refresh(cx: &mut App) {
    apply(cx.window_appearance(), cx);
    cx.refresh_windows();
}

fn apply(system: WindowAppearance, cx: &mut App) {
    let appearance = effective(system, cx);
    Theme::change(appearance, None, cx);
    let p = Palette::new(ThemeMode::from(appearance).is_dark());
    cx.set_global(p);
    let c = |v: u32| -> Hsla { rgb(v).into() };
    let prefs = settings::get(cx).clone();
    let t = Theme::global_mut(cx);
    // GPUI Component sizes controls in rem, and Root sets rem to this value.
    t.font_size = px(15.);
    t.mono_font_family = prefs.mono_font.into();
    t.mono_font_size = px(prefs.mono_size as f32);
    t.radius = px(5.);
    t.radius_lg = px(8.);
    t.shadow = false;
    t.background = c(p.background);
    t.foreground = c(p.text);
    t.border = c(p.border);
    t.input = c(p.border);
    t.muted = c(p.chrome);
    t.muted_foreground = c(p.muted);
    t.primary = c(p.blue);
    t.primary_foreground = c(p.primary_text);
    t.primary_hover = c(p.blue).opacity(0.9);
    t.primary_active = c(p.blue).opacity(0.8);
    t.secondary = c(p.panel);
    t.secondary_foreground = c(p.text);
    t.secondary_hover = c(p.hover);
    t.secondary_active = c(p.border);
    t.accent = c(p.hover);
    t.accent_foreground = c(p.text);
    t.selection = c(p.selected);
    t.caret = c(p.blue);
    t.popover = c(p.popover);
    t.popover_foreground = c(p.text);
    t.ring = c(p.blue);
    t.link = c(p.blue);
    t.title_bar = c(p.chrome);
    t.title_bar_border = c(p.border);
    t.list_active = c(p.selected);
    t.list_active_border = c(p.selected);
    t.list_hover = c(p.hover);
    t.scrollbar = c(p.background).opacity(0.);
    t.scrollbar_thumb = c(p.quiet).opacity(0.45);
    t.scrollbar_thumb_hover = c(p.quiet).opacity(0.7);
    let highlight = std::sync::Arc::make_mut(&mut t.highlight_theme);
    highlight.style.editor_background = Some(c(p.background));
    highlight.style.editor_foreground = Some(c(p.text));
    highlight.style.editor_active_line = Some(c(p.chrome).opacity(0.7));
    highlight.style.editor_line_number = Some(c(p.quiet));
    highlight.style.editor_active_line_number = Some(c(p.muted));
}
