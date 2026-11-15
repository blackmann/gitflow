//! Tree-sitter syntax highlighting (GPUI Component's grammars, as in Zed), split
//! into per-line styles so virtualized views can colour only what they draw.
//! Light and dark styles are both computed, so switching appearance is free.
use crate::gh::{DiffLine, LineKind};
use gpui::{HighlightStyle, SharedString};
use gpui_component::{
    Rope,
    highlighter::{HighlightTheme, SyntaxHighlighter},
};
use std::{ops::Range, sync::LazyLock};

/// Larger files show as plain text; highlighting them would stall the view.
const MAX_BYTES: usize = 1_500_000;
const MAX_LINES: usize = 40_000;

pub type LineStyles = Vec<(Range<usize>, HighlightStyle)>;
pub struct Highlights {
    pub light: Vec<LineStyles>,
    pub dark: Vec<LineStyles>,
}
impl Highlights {
    pub fn line(&self, dark: bool, n: usize) -> Option<&LineStyles> {
        if dark { self.dark.get(n) } else { self.light.get(n) }
    }
}

/// The grammar for a file name or a fenced code block's tag, if one is bundled.
pub fn language(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    let file = lower.rsplit('/').next().unwrap_or(&lower);
    match file {
        "makefile" | "gnumakefile" => return Some("make"),
        "cmakelists.txt" => return Some("cmake"),
        _ => {}
    }
    let ext = file.rsplit_once('.').map(|(_, e)| e).unwrap_or(file);
    Some(match ext {
        "rs" | "rust" => "rust",
        "ts" | "mts" | "cts" | "typescript" => "typescript",
        // The bundled TSX grammar carries only TypeScript's own highlight query (no
        // strings, keywords or JSX); JavaScript's reads TSX far better.
        "tsx" => "javascript",
        "js" | "mjs" | "cjs" | "jsx" | "javascript" => "javascript",
        "py" | "pyi" | "python" => "python",
        "go" | "golang" => "go",
        "rb" | "ruby" | "rake" | "gemspec" => "ruby",
        "java" => "java",
        "c" | "h" => "c",
        "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "c++" => "cpp",
        "cs" | "csharp" => "csharp",
        "css" | "scss" => "css",
        "html" | "htm" => "html",
        "json" | "jsonc" | "json5" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "md" | "markdown" | "mdx" => "markdown",
        "sh" | "bash" | "zsh" | "shell" | "console" => "bash",
        "sql" => "sql",
        "swift" => "swift",
        "zig" => "zig",
        "ex" | "exs" | "elixir" => "elixir",
        "graphql" | "gql" => "graphql",
        "proto" | "protobuf" => "proto",
        "scala" | "sc" => "scala",
        "diff" | "patch" => "diff",
        "erb" => "erb",
        "ejs" => "ejs",
        "make" | "mk" => "make",
        "cmake" => "cmake",
        _ => return None,
    })
}

/// Highlight `lines` (already tab-expanded) as `lang`, for both themes.
pub fn highlight(lang: &str, lines: &[SharedString]) -> Option<Highlights> {
    if lines.len() > MAX_LINES {
        return None;
    }
    let mut text = String::new();
    let mut starts = Vec::with_capacity(lines.len());
    for l in lines {
        starts.push(text.len());
        text.push_str(l);
        text.push('\n');
    }
    if text.len() > MAX_BYTES {
        return None;
    }
    let mut highlighter = SyntaxHighlighter::new(lang);
    highlighter.update(None, &Rope::from_str(&text));
    let whole = 0..text.len();
    let split = |theme: &HighlightTheme| per_line(&highlighter.styles(&whole, theme), &starts, lines);
    Some(Highlights {
        light: split(&ONE_LIGHT),
        dark: split(&ONE_DARK),
    })
}

/// Highlight a file's diff lines: each side (old = context + deletions, new =
/// context + additions) parses as its own document so both read as real code.
/// Indexed by diff line; hunk headers get no styles.
pub fn highlight_diff(path: &str, lines: &[DiffLine], text: &[SharedString]) -> Option<Highlights> {
    let lang = language(path)?;
    let side = |keep: LineKind| -> (Vec<usize>, Option<Highlights>) {
        let idx: Vec<usize> = (0..lines.len())
            .filter(|&i| lines[i].kind == LineKind::Context || lines[i].kind == keep)
            .collect();
        let doc: Vec<SharedString> = idx.iter().map(|&i| text[i].clone()).collect();
        (idx, highlight(lang, &doc))
    };
    let (new_idx, new) = side(LineKind::Add);
    let (old_idx, old) = side(LineKind::Del);
    let (new, old) = (new?, old?);
    let mut light = vec![LineStyles::new(); lines.len()];
    let mut dark = vec![LineStyles::new(); lines.len()];
    for (k, &i) in new_idx.iter().enumerate() {
        light[i] = new.light[k].clone();
        dark[i] = new.dark[k].clone();
    }
    for (k, &i) in old_idx.iter().enumerate() {
        if lines[i].kind == LineKind::Del {
            light[i] = old.light[k].clone();
            dark[i] = old.dark[k].clone();
        }
    }
    Some(Highlights { light, dark })
}

/// Code colours: Atom's One Dark and One Light, in plain weights.
static ONE_DARK: LazyLock<HighlightTheme> = LazyLock::new(|| {
    one(
        "dark",
        Ui {
            fg: "#abb2bf",
            comment: "#5c6370",
            red: "#e06c75",
            orange: "#d19a66",
            yellow: "#e5c07b",
            green: "#98c379",
            cyan: "#56b6c2",
            blue: "#61afef",
            purple: "#c678dd",
        },
    )
});
static ONE_LIGHT: LazyLock<HighlightTheme> = LazyLock::new(|| {
    one(
        "light",
        Ui {
            fg: "#383a42",
            comment: "#a0a1a7",
            red: "#e45649",
            orange: "#986801",
            yellow: "#c18401",
            green: "#50a14f",
            cyan: "#0184bc",
            blue: "#4078f2",
            purple: "#a626a4",
        },
    )
});
/// The code foreground for each appearance, for unhighlighted text.
pub const ONE_DARK_FG: u32 = 0xabb2bf;
pub const ONE_LIGHT_FG: u32 = 0x383a42;

struct Ui {
    fg: &'static str,
    comment: &'static str,
    red: &'static str,
    orange: &'static str,
    yellow: &'static str,
    green: &'static str,
    cyan: &'static str,
    blue: &'static str,
    purple: &'static str,
}
fn one(appearance: &str, c: Ui) -> HighlightTheme {
    let color = |hex: &str| serde_json::json!({ "color": hex });
    let syntax = serde_json::json!({
        "attribute": color(c.orange),
        "boolean": color(c.orange),
        "comment": color(c.comment),
        "comment.doc": color(c.comment),
        "constant": color(c.orange),
        "constructor": color(c.yellow),
        "embedded": color(c.fg),
        "enum": color(c.yellow),
        "function": color(c.blue),
        "keyword": color(c.purple),
        "label": color(c.blue),
        "link_text": color(c.blue),
        "link_uri": color(c.cyan),
        "number": color(c.orange),
        "operator": color(c.cyan),
        "preproc": color(c.purple),
        "property": color(c.red),
        "punctuation": color(c.fg),
        "punctuation.bracket": color(c.fg),
        "punctuation.delimiter": color(c.fg),
        "punctuation.list_marker": color(c.red),
        "punctuation.special": color(c.purple),
        "string": color(c.green),
        "string.escape": color(c.cyan),
        "string.regex": color(c.cyan),
        "string.special": color(c.cyan),
        "string.special.symbol": color(c.cyan),
        "tag": color(c.red),
        "tag.doctype": color(c.purple),
        "text.literal": color(c.green),
        "title": color(c.red),
        "type": color(c.yellow),
        "variable": color(c.fg),
        "variable.special": color(c.red),
        "variant": color(c.blue),
    });
    serde_json::from_value(serde_json::json!({
        "name": format!("One {appearance}"),
        "appearance": appearance,
        "style": { "editor.foreground": c.fg, "syntax": syntax },
    }))
    .expect("One theme")
}

/// Clip whole-text styles to each line, as line-relative ranges.
fn per_line(styles: &[(Range<usize>, HighlightStyle)], starts: &[usize], lines: &[SharedString]) -> Vec<LineStyles> {
    let mut out = vec![LineStyles::new(); lines.len()];
    for (range, style) in styles {
        if *style == HighlightStyle::default() || range.is_empty() {
            continue;
        }
        let mut line = starts.partition_point(|s| *s <= range.start).saturating_sub(1);
        while line < lines.len() {
            let (start, len) = (starts[line], lines[line].len());
            if start >= range.end {
                break;
            }
            let from = range.start.max(start) - start;
            let to = range.end.min(start + len) - start;
            if from < to {
                out[line].push((from..to, *style));
            }
            line += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{highlight, highlight_diff, language};
    use crate::gh::{DiffLine, LineKind};
    use gpui::SharedString;
    #[test]
    fn picks_languages_by_name() {
        assert_eq!(language("src/main.rs"), Some("rust"));
        assert_eq!(language("App.TSX"), Some("javascript"));
        assert_eq!(language("Makefile"), Some("make"));
        assert_eq!(language("py"), Some("python"));
        assert_eq!(language("notes.txt"), None);
    }
    #[test]
    fn styles_stay_within_their_lines() {
        let lines = ["fn main() {", "    let x = \"hi\";", "}"].map(SharedString::from);
        let h = highlight("rust", &lines).expect("rust grammar");
        assert_eq!(h.dark.len(), 3);
        assert!(h.dark[0].iter().any(|(r, _)| &lines[0][r.clone()] == "fn"));
        for (n, styles) in h.light.iter().enumerate() {
            assert!(styles.iter().all(|(r, _)| r.end <= lines[n].len()));
        }
    }
    #[test]
    fn diff_sides_highlight_separately() {
        let mk = |kind, text: &str| DiffLine { kind, old: None, new: None, text: text.into() };
        let lines = vec![
            mk(LineKind::Hunk, "@@ -1,2 +1,2 @@"),
            mk(LineKind::Context, "fn main() {"),
            mk(LineKind::Del, "    let a = 1;"),
            mk(LineKind::Add, "    let b = 2;"),
            mk(LineKind::Context, "}"),
        ];
        let text: Vec<SharedString> = lines.iter().map(|l| l.text.clone().into()).collect();
        let h = highlight_diff("x.rs", &lines, &text).expect("rust");
        assert!(h.dark[0].is_empty());
        assert!(h.dark[2].iter().any(|(r, _)| &text[2][r.clone()] == "let"));
        assert!(h.dark[3].iter().any(|(r, _)| &text[3][r.clone()] == "let"));
        assert!(h.dark.iter().flatten().all(|(_, st)| st.font_weight.is_none()));
    }
}
