//! The web package's own parts: the page (`packaging/web/`), the fonts it downloads on demand
//! and the icons its manifest and the page name.
use std::path::Path;

pub const TRIPLE: &str = "wasm32-unknown-unknown";

/// The page's files copied as they are, and the ones whose `{...}` are filled.
const PAGE: [&str; 3] = ["gl.js", "sml26gc.js", "capture-worker.js"];

/// The editor's capture workers' program: its crate and the file it builds.
pub const WORKER: (&str, &str) = ("capture-worker", "sml26gc_capture.wasm");
const FILLED: [&str; 3] = ["index.html", "editor.html", "manifest.webmanifest"];

/// The fonts the page downloads when their language is chosen (`fonts::Face::file`).
const FONTS: [&str; 3] = ["BIZUDGothic-Regular.ttf", "NotoSansSC-Regular.otf", "NotoSansKR-Regular.otf"];

/// The icon `index.html` names for the iPhone's home screen (`apple-touch-icon`), besides the
/// manifest's.
const TOUCH_ICON: (&str, u32) = ("icons/icon-180.png", 180);

/// Stops unless the WebAssembly target is installed.
pub fn check_target() -> std::result::Result<(), String> {
    let installed = std::process::Command::new("rustup").args(["target", "list", "--installed"]).output().map(|o| String::from_utf8_lossy(&o.stdout).contains(TRIPLE)).unwrap_or(false);
    installed.then_some(()).ok_or_else(|| format!("the {TRIPLE} target is not installed: rustup target add {TRIPLE}"))
}

/// The capture workers' program (from `built`), the pages, the fonts and the icons into the
/// package `folder` (which holds the programs already); `fill` fills the texts.
pub fn assemble(root: &Path, built: &Path, folder: &Path, programs: &[String], fill: impl Fn(&str) -> String) -> Result<(), String> {
    let web = root.join("packaging").join("web");
    std::fs::copy(built.join(WORKER.1), folder.join(WORKER.1)).map_err(|e| format!("{}: {e}", WORKER.1))?;
    for f in PAGE {
        std::fs::copy(web.join(f), folder.join(f)).map_err(|e| format!("packaging/web/{f}: {e}"))?;
    }
    let files: Vec<&str> = programs.iter().map(String::as_str).chain([WORKER.1]).chain(PAGE).collect();
    let build = build_id(folder, &files)?;
    for f in FILLED {
        let text = std::fs::read_to_string(web.join(f)).map_err(|e| format!("packaging/web/{f}: {e}"))?;
        std::fs::write(folder.join(f), fill(&text).replace("{build}", &build)).map_err(|e| format!("{f}: {e}"))?;
    }
    std::fs::create_dir_all(folder.join("fonts")).map_err(|e| format!("fonts: {e}"))?;
    for f in FONTS {
        std::fs::copy(root.join("fonts").join(f), folder.join("fonts").join(f)).map_err(|e| format!("fonts/{f}: {e}"))?;
    }
    std::fs::create_dir_all(folder.join("icons")).map_err(|e| format!("icons: {e}"))?;
    let icon = root.join("assets").join("icons").join(format!("{}.png", env!("PRODUCT_ID")));
    for (path, size) in icons(&web.join("manifest.webmanifest"))?.into_iter().chain([(TOUCH_ICON.0.to_string(), TOUCH_ICON.1)]) {
        std::fs::write(folder.join(&path), crate::icons::scaled(&icon, size)?).map_err(|e| format!("{path}: {e}"))?;
    }
    Ok(())
}

/// A tag of the programs' and the scripts' contents, added to their addresses in the pages
/// (`?v=`) so that a browser never runs an older copy it kept.
fn build_id(folder: &Path, files: &[&str]) -> Result<String, String> {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for f in files {
        h.write(&std::fs::read(folder.join(f)).map_err(|e| format!("{f}: {e}"))?);
    }
    Ok(format!("{:016x}", h.finish()))
}

/// The icons the manifest names: their paths and sizes.
fn icons(manifest: &Path) -> Result<Vec<(String, u32)>, String> {
    let text = std::fs::read_to_string(manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", manifest.display()))?;
    json["icons"]
        .as_array()
        .ok_or("the manifest names no icons")?
        .iter()
        .map(|i| {
            let src = i["src"].as_str().ok_or("an icon without src")?;
            let size = i["sizes"].as_str().and_then(|s| s.split('x').next()?.parse().ok()).ok_or(format!("{src}: no size"))?;
            Ok((src.to_string(), size))
        })
        .collect()
}
