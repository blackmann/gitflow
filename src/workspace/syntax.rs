//! Tree-sitter syntax highlighting (GPUI Component's grammars, as in Zed), split
//! into per-line styles so virtualized views can colour only what they draw.
//! Light and dark styles are both computed, so switching appearance is free.
use gpui::{HighlightStyle, SharedString};
use gpui_component::{
    Rope,
    highlighter::{HighlightTheme, SyntaxHighlighter},
};
use std::ops::Range;

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
        "tsx" => "tsx",
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
        light: split(&HighlightTheme::default_light()),
        dark: split(&HighlightTheme::default_dark()),
    })
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
    use super::{highlight, language};
    use gpui::SharedString;
    #[test]
    fn picks_languages_by_name() {
        assert_eq!(language("src/main.rs"), Some("rust"));
        assert_eq!(language("App.TSX"), Some("tsx"));
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
}
