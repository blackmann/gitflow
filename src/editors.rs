//! Other apps a file can be opened in: the code editors installed on this Mac,
//! each with its icon, found once at startup.
use std::path::{Path, PathBuf};
use std::process::Command;

/// An installed editor.
#[derive(Clone, Debug)]
pub struct Editor {
    pub name: String,
    /// The `.app` bundle.
    pub app: PathBuf,
    /// The app's icon as a PNG, cached; `None` when it couldn't be read.
    pub icon: Option<PathBuf>,
    /// Whether it opens a folder as a project, not just files.
    pub folders: bool,
}

/// Editors looked for, by bundle name, in the order they're listed. The bool
/// says whether each opens folders.
const KNOWN: &[(&str, bool)] = &[
    ("Zed", true),
    ("Zed Preview", true),
    ("Visual Studio Code", true),
    ("Visual Studio Code - Insiders", true),
    ("Cursor", true),
    ("Windsurf", true),
    ("Kiro", true),
    ("VSCodium", true),
    ("Sublime Text", true),
    ("Nova", true),
    ("BBEdit", true),
    ("CotEditor", false),
    ("Fleet", true),
    ("IntelliJ IDEA", true),
    ("IntelliJ IDEA CE", true),
    ("IntelliJ IDEA Ultimate", true),
    ("WebStorm", true),
    ("RustRover", true),
    ("PyCharm", true),
    ("PyCharm CE", true),
    ("PyCharm Community Edition", true),
    ("GoLand", true),
    ("CLion", true),
    ("PhpStorm", true),
    ("Android Studio", true),
    ("MacVim", true),
    ("Neovide", true),
    ("Xcode", true),
    ("TextEdit", false),
];

/// Finder, for revealing a file rather than opening it.
pub const FINDER: &str = "/System/Library/CoreServices/Finder.app";

fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/Applications")];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join("Applications"));
    }
    dirs.push("/System/Applications".into());
    dirs
}

/// The editors installed, with their icons. Slow-ish (it reads bundles and may
/// convert icons), so call it off the main thread.
pub fn installed() -> Vec<Editor> {
    if !cfg!(target_os = "macos") {
        return Vec::new();
    }
    let dirs = search_dirs();
    KNOWN
        .iter()
        .filter_map(|&(name, folders)| {
            let app = dirs.iter().map(|d| d.join(format!("{name}.app"))).find(|p| p.is_dir())?;
            Some(Editor { name: name.to_string(), icon: icon(&app), app, folders })
        })
        .collect()
}

/// An app's icon as a PNG, converted from its `.icns` into the cache once.
pub fn icon(app: &Path) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let cache = PathBuf::from(home).join("Library/Caches/Gitflow/app-icons");
    let name = app.file_stem()?.to_string_lossy().into_owned();
    let out = cache.join(format!("{name}.png"));
    let plist = app.join("Contents/Info.plist");
    let raw = Command::new("plutil").args(["-extract", "CFBundleIconFile", "raw"]).arg(&plist).output().ok()?;
    let file = String::from_utf8_lossy(&raw.stdout).trim().to_string();
    if !raw.status.success() || file.is_empty() {
        return None;
    }
    let icns = app.join("Contents/Resources").join(if file.ends_with(".icns") { file } else { format!("{file}.icns") });
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    // Fresh as long as the app hasn't been updated since.
    if out.is_file() && modified(&out) >= modified(&icns) {
        return Some(out);
    }
    std::fs::create_dir_all(&cache).ok()?;
    let ok = Command::new("sips")
        .args(["-s", "format", "png", "-Z", "64"])
        .arg(&icns)
        .arg("--out")
        .arg(&out)
        .output()
        .is_ok_and(|o| o.status.success());
    (ok && out.is_file()).then_some(out)
}

/// What the Open with menus offer: the editors, and Finder's icon for Reveal.
#[derive(Default)]
pub struct Editors {
    pub list: Vec<Editor>,
    pub finder: Option<PathBuf>,
}

/// Look everything up; call it off the main thread.
pub fn load() -> Editors {
    Editors { list: installed(), finder: icon(Path::new(FINDER)) }
}

/// Open `path` in the app at `app`, waiting on `open` off the caller's thread.
pub fn open(app: &Path, path: &Path) {
    let (app, path) = (app.to_path_buf(), path.to_path_buf());
    std::thread::spawn(move || {
        if let Err(e) = Command::new("open").arg("-a").arg(&app).arg(&path).status() {
            eprintln!("couldn't open {} in {}: {e}", path.display(), app.display());
        }
    });
}
