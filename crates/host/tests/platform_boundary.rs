//! What depends on the operating system stays in adapters: `adapters/` folders, this crate, the
//! build scripts and the build tools. The rest builds for any platform, so a new platform is new
//! adapters, not changes through the code (docs/ARCHITECTURE.md, "Platforms").
use std::path::Path;

const SYSTEM_MARKS: [&str; 9] = ["cfg(windows)", "cfg(not(windows))", "cfg(unix)", "cfg(target_os", "cfg!(windows)", "cfg!(target_os", "cfg(target_arch", "cfg!(target_arch", "windows_sys"];

/// Where system-dependent code may be (paths from the workspace root, with `/`).
fn allowed(path: &str) -> bool {
    path.contains("/adapters/") || path.starts_with("crates/host/") || path.starts_with("crates/app-resources/") || path.starts_with("xtask/") || path.ends_with("/build.rs")
}

fn sources(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            if !["target", "dist", ".git", "openspec", "legacy"].contains(&rel.as_str()) {
                sources(&path, root, out);
            }
        } else if rel.ends_with(".rs") {
            out.push((rel, std::fs::read_to_string(&path).unwrap()));
        }
    }
}

#[test]
fn system_dependent_code_is_only_in_adapters() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let mut files = Vec::new();
    sources(&root, &root, &mut files);
    assert!(files.iter().any(|(p, _)| p == "game/src/main.rs"), "the sources were not found");
    let wrong: Vec<String> = files.iter().filter(|(p, _)| !allowed(p)).flat_map(|(p, src)| SYSTEM_MARKS.iter().filter(|m| src.contains(*m)).map(move |m| format!("{p}: {m}"))).collect();
    assert!(wrong.is_empty(), "system-dependent code outside the adapters:\n{}", wrong.join("\n"));
}
