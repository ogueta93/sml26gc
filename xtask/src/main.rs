//! The release tool, run as `cargo xtask <command>` (the alias is in `.cargo/config.toml`):
//!
//!   targets               the platforms a package can be built for
//!   package TARGET        builds the game and the editor for TARGET and writes
//!                         `dist/TARGET/<id>/` and the target's packages in `dist/`
//!   check-portable        checks the platform-independent crates for Linux
//!
//! A package holds the two executables, the read-me (`packaging/<platform>/`) and the cartridge
//! notice (`packaging/`), the licence (`LICENSE`), the legal notice (`LEGAL.md`) and the
//! changelog, the manuals (`docs/GAME.md`, `docs/EDITOR.md`) and the other licences, and nothing
//! else: it is built into its own folder under `target/`, apart from the developer's
//! `target/release` and the files the programs write there.
mod icons;
mod licences;
mod linux;
mod macos;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// A platform a package can be built for, built on that platform.
struct Target {
    name: &'static str,
    triple: &'static str,
    /// `std::env::consts::OS` of the computer that builds it.
    host_os: &'static str,
    suffix: &'static str,
    imports: Imports,
    /// Its texts end their lines with CRLF.
    crlf: bool,
    readme: &'static str,
    build: Build,
    layout: Layout,
    outputs: &'static [Output],
}

/// Which shared libraries the executables may need.
enum Imports {
    /// Any but these (prefixes; they are not on every computer).
    Deny(&'static [&'static str]),
    /// Only these, and no glibc newer than `glibc` (the read-me names them all).
    Allow { libraries: &'static [&'static str], glibc: (u32, u32) },
    /// Only the system's, every architecture of `Build::Universal`, and no minimum macOS newer
    /// than `macos`, which they are built for.
    MacOs { macos: (u32, u32) },
}

enum Build {
    /// `cargo build` on this computer.
    Here,
    /// `cargo build` in the Linux build container (`packaging/linux/Dockerfile`).
    Container,
    /// `cargo build` on this computer for each of these triples, joined by `lipo`.
    Universal(&'static [&'static str]),
}

/// Where the executables go in the package folder.
enum Layout {
    Flat,
    /// Each in its macOS app bundle.
    AppBundles,
}

enum Output {
    Zip,
    TarGz,
    Deb,
    /// The zip made by `ditto`, which keeps the app bundles' modes and signatures.
    Ditto,
}

const TARGETS: &[Target] = &[
    Target { name: "windows-x64", triple: "x86_64-pc-windows-msvc", host_os: "windows", suffix: ".exe", imports: Imports::Deny(&["vcruntime", "api-ms-win-crt-", "msvcp"]), crlf: true, readme: "windows/README.txt", build: Build::Here, layout: Layout::Flat, outputs: &[Output::Zip] },
    Target {
        name: "linux-x64",
        triple: "x86_64-unknown-linux-gnu",
        host_os: "linux",
        suffix: "",
        imports: Imports::Allow { libraries: &["libc.so.6", "libm.so.6", "libgcc_s.so.1", "ld-linux-x86-64.so.2", "libdl.so.2", "libpthread.so.0", "librt.so.1", "libasound.so.2", "libudev.so.1"], glibc: (2, 31) },
        crlf: false,
        readme: "linux/README.txt",
        build: Build::Container,
        layout: Layout::Flat,
        outputs: &[Output::TarGz, Output::Deb],
    },
    Target {
        name: "macos-universal",
        triple: "aarch64-apple-darwin",
        host_os: "macos",
        suffix: "",
        imports: Imports::MacOs { macos: (11, 0) },
        crlf: false,
        readme: "macos/README.txt",
        build: Build::Universal(&["aarch64-apple-darwin", "x86_64-apple-darwin"]),
        layout: Layout::AppBundles,
        outputs: &[Output::Ditto],
    },
];

/// The programs shipped: their crates (the binaries' names come from the manifests).
const PROGRAMS: [&str; 2] = ["game", "editor"];

/// The user manuals, copied into `manuals/`.
const MANUALS: [&str; 2] = ["GAME.md", "EDITOR.md"];

/// The licences of the fonts whose glyphs are in the executables.
const FONT_LICENCES: [&str; 3] = ["OFL-BIZUDGothic.txt", "OFL-NotoSansCJK.txt", "OFL-FusionPixelFont.txt"];

/// The crates checked against Linux: they know no system.
const PORTABLE: [&str; 5] = ["gb-core", "cart-model", "project-model", "i18n", "fonts"];
const LINUX: &str = "x86_64-unknown-linux-gnu";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("targets") => {
            TARGETS.iter().for_each(|t| println!("{} ({}, built on {})", t.name, t.triple, t.host_os));
            Ok(())
        }
        Some("package") => match TARGETS.iter().find(|t| Some(t.name) == args.get(1).map(String::as_str)) {
            Some(t) => package(t),
            None => Err(format!("package what? The targets are: {}", TARGETS.iter().map(|t| t.name).collect::<Vec<_>>().join(", "))),
        },
        Some("check-portable") => check_portable(),
        _ => Err("usage: cargo xtask targets | package TARGET | check-portable".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn cargo() -> Command {
    let mut c = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    c.current_dir(root());
    c
}

fn run(mut c: Command) -> Result<(), String> {
    let status = c.status().map_err(|e| format!("{}: {e}", c.get_program().to_string_lossy()))?;
    status.success().then_some(()).ok_or_else(|| format!("{c:?} failed"))
}

/// `cargo metadata` for `triple`'s platform.
fn metadata(triple: &str) -> Result<serde_json::Value, String> {
    let out = cargo().args(["metadata", "--format-version", "1", "--filter-platform", triple]).output().map_err(|e| format!("cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(format!("cargo metadata: {}", String::from_utf8_lossy(&out.stderr)));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("cargo metadata: {e}"))
}

/// What the texts' `{...}` are filled with.
struct Fill {
    programs: Vec<String>,
    /// The oldest glibc the Linux executables need, as `2.31`.
    glibc: Option<String>,
    /// The oldest macOS the macOS executables run on, as `11.0`.
    macos: Option<String>,
    version: String,
    crlf: bool,
}

impl Fill {
    fn text(&self, text: &str) -> String {
        let text = text
            .replace("{product}", env!("PRODUCT_NAME"))
            .replace("{id}", env!("PRODUCT_ID"))
            .replace("{game}", &self.programs[0])
            .replace("{editor}", &self.programs[1])
            .replace("{source}", env!("PRODUCT_SOURCE"))
            .replace("{maintainer}", env!("PRODUCT_MAINTAINER"))
            .replace("{glibc}", self.glibc.as_deref().unwrap_or("?"))
            .replace("{macos}", self.macos.as_deref().unwrap_or("?"));
        if self.crlf { text.replace("\r\n", "\n").replace('\n', "\r\n") } else { text }
    }
}

fn package(t: &Target) -> Result<(), String> {
    let id = env!("PRODUCT_ID");
    if std::env::consts::OS != t.host_os {
        return Err(format!("the {} package is built on {} (this computer runs {})", t.name, t.host_os, std::env::consts::OS));
    }
    let meta = metadata(t.triple)?;
    let packages = meta["packages"].as_array().ok_or("cargo metadata: no packages")?;
    let game = packages.iter().find(|p| p["name"] == PROGRAMS[0]).ok_or("no game package")?;
    let version = game["version"].as_str().ok_or("no game version")?.to_string();
    let target_dir = PathBuf::from(meta["target_directory"].as_str().ok_or("cargo metadata: no target directory")?);
    let mut bins = Vec::new();
    for p in PROGRAMS {
        let bin = packages.iter().find(|q| q["name"] == p).and_then(|q| q["targets"].as_array()?.iter().find(|t| t["kind"][0] == "bin")?["name"].as_str()).ok_or(format!("no binary in {p}"))?;
        bins.push(bin.to_string());
    }
    let built = match t.build {
        Build::Here => {
            let mut build = cargo();
            build.args(["build", "--profile", "dist", "--target", t.triple]);
            for p in PROGRAMS {
                build.args(["-p", p]);
            }
            run(build)?;
            target_dir.join(t.triple).join("dist")
        }
        Build::Container => {
            let rust = game["rust_version"].as_str().ok_or("no rust-version in the workspace")?;
            linux::build_in_container(&root(), rust, t.triple, &PROGRAMS)?
        }
        Build::Universal(triples) => {
            let Imports::MacOs { macos } = t.imports else { return Err(format!("{}: a universal build needs its minimum macOS", t.name)) };
            macos::build_universal(&target_dir, triples, &PROGRAMS, &bins, macos)?
        }
    };

    let folder = root().join("dist").join(t.name).join(id);
    if folder.exists() {
        std::fs::remove_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    }
    for sub in ["licenses", "manuals"] {
        std::fs::create_dir_all(folder.join(sub)).map_err(|e| format!("{}: {e}", folder.display()))?;
    }

    let mut programs = Vec::new();
    let (mut glibc, mut macos) = (None, None);
    for bin in &bins {
        let file = format!("{bin}{}", t.suffix);
        let from = built.join(&file);
        match t.imports {
            Imports::Deny(denied) => check_imports(&from, denied)?,
            Imports::Allow { libraries, glibc: limit } => glibc = glibc.max(Some(linux::check_libraries(&from, libraries, limit)?)),
            Imports::MacOs { macos: limit } => {
                let Build::Universal(triples) = t.build else { return Err(format!("{}: the macOS check needs a universal build", t.name)) };
                macos = macos.max(Some(macos::check(&from, triples, limit)?));
            }
        }
        std::fs::copy(&from, folder.join(&file)).map_err(|e| format!("{}: {e}", from.display()))?;
        programs.push(file);
    }
    let fill = Fill { programs, glibc: glibc.map(|(a, b)| format!("{a}.{b}")), macos: macos.map(|(a, b)| format!("{a}.{b}")), version: version.clone(), crlf: t.crlf };
    if let Some(g) = &fill.glibc {
        println!("glibc:   the executables need {g} or newer");
    }
    if let Some(m) = &fill.macos {
        println!("macOS:   the executables need macOS {m} or newer");
    }
    if let (Layout::AppBundles, Imports::MacOs { macos: limit }) = (&t.layout, &t.imports) {
        macos::bundles(&root(), &folder, &fill, *limit)?;
    }

    let packaging = root().join("packaging");
    for (from, to) in [(packaging.join(t.readme), "README.txt"), (packaging.join("NO-CARTRIDGE.txt"), "NO-CARTRIDGE.txt"), (root().join("LICENSE"), "LICENSE.txt"), (root().join("LEGAL.md"), "LEGAL.md"), (root().join("CHANGELOG.md"), "CHANGELOG.md")] {
        let text = std::fs::read_to_string(&from).map_err(|e| format!("{}: {e}", from.display()))?;
        std::fs::write(folder.join(to), fill.text(&text)).map_err(|e| format!("{to}: {e}"))?;
    }
    for f in MANUALS {
        let text = std::fs::read_to_string(root().join("docs").join(f)).map_err(|e| format!("docs/{f}: {e}"))?;
        std::fs::write(folder.join("manuals").join(f), fill.text(&text)).map_err(|e| format!("manuals/{f}: {e}"))?;
    }
    for f in FONT_LICENCES {
        std::fs::copy(root().join("fonts").join(f), folder.join("licenses").join(f)).map_err(|e| format!("fonts/{f}: {e}"))?;
    }
    let crates = licences::third_party(&meta, &PROGRAMS)?;
    std::fs::write(folder.join("licenses").join("THIRD-PARTY.txt"), fill.text(&crates)).map_err(|e| format!("THIRD-PARTY.txt: {e}"))?;
    println!("package: {}", folder.display());

    for output in t.outputs {
        let file = match output {
            Output::Zip => {
                let zip = root().join("dist").join(format!("{id}-{version}-{}.zip", t.name));
                zip_folder(&folder, id, &zip)?;
                zip
            }
            Output::TarGz => {
                let tar = root().join("dist").join(format!("{id}-{version}-{}.tar.gz", t.name));
                linux::tar_folder(&folder, id, &tar, &fill.programs)?;
                tar
            }
            Output::Deb => linux::deb(&root(), &folder, &fill)?,
            Output::Ditto => {
                let zip = root().join("dist").join(format!("{id}-{version}-{}.zip", t.name));
                macos::ditto(&folder, &zip)?;
                zip
            }
        };
        println!("written: {}", file.display());
    }
    Ok(())
}

/// Stops when `exe` imports a library starting with one of `denied`.
fn check_imports(exe: &Path, denied: &[&str]) -> Result<(), String> {
    use object::read::Object;
    let data = std::fs::read(exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    let file = object::File::parse(&*data).map_err(|e| format!("{}: {e}", exe.display()))?;
    let imports = file.imports().map_err(|e| format!("{}: {e}", exe.display()))?;
    let mut bad: Vec<String> = imports.iter().map(|i| String::from_utf8_lossy(i.library()).to_lowercase()).filter(|l| denied.iter().any(|f| l.starts_with(f))).collect();
    bad.sort();
    bad.dedup();
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!("{} needs {} (the C runtime is not linked in: see .cargo/config.toml)", exe.display(), bad.join(", ")))
    }
}

/// `folder`'s files, sorted, as paths relative to it with `/`.
fn files_in(folder: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let mut stack = vec![folder.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?.flatten() {
            if e.path().is_dir() {
                stack.push(e.path());
            } else {
                files.push(e.path());
            }
        }
    }
    files.sort();
    Ok(files.into_iter().map(|f| {
        let rel = f.strip_prefix(folder).unwrap().to_string_lossy().replace('\\', "/");
        (f, rel)
    }).collect())
}

/// `folder`'s files into `zip`, under a top folder `top`.
fn zip_folder(folder: &Path, top: &str, zip: &Path) -> Result<(), String> {
    let file = std::fs::File::create(zip).map_err(|e| format!("{}: {e}", zip.display()))?;
    let mut w = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (f, rel) in files_in(folder)? {
        w.start_file(format!("{top}/{rel}"), options).map_err(|e| e.to_string())?;
        std::io::copy(&mut std::fs::File::open(&f).map_err(|e| format!("{}: {e}", f.display()))?, &mut w).map_err(|e| e.to_string())?;
    }
    w.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn check_portable() -> Result<(), String> {
    let installed = Command::new("rustup").args(["target", "list", "--installed"]).output().map(|o| String::from_utf8_lossy(&o.stdout).contains(LINUX)).unwrap_or(false);
    if !installed {
        return Err(format!("the {LINUX} target is not installed: rustup target add {LINUX}"));
    }
    let mut c = cargo();
    c.args(["check", "--target", LINUX]);
    for p in PORTABLE {
        c.args(["-p", p]);
    }
    run(c)
}
