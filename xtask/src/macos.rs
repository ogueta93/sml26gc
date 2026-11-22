//! The macOS package: each program built for Apple Silicon and Intel with the oldest macOS it
//! supports as its minimum, joined into a universal executable (`lipo`) and checked, the two app
//! bundles (`Info.plist`, an `.icns` icon, an ad hoc signature) and the zip (`ditto`, which keeps
//! the bundles' modes and signatures).
use crate::{cargo, run, Fill};
use object::read::macho::{FatArch, MachOFatFile32, MachOFile64};
use object::{Architecture, Endianness};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The app bundles' names (`host::paths::GAME_BUNDLE`, `EDITOR_BUNDLE`), in `PROGRAMS` order.
const BUNDLES: [&str; 2] = ["SML2 6GC", "SML2 6GC Editor"];
/// Finder categories, in `PROGRAMS` order.
const CATEGORIES: [&str; 2] = ["public.app-category.games", "public.app-category.developer-tools"];
/// The joined executables, apart from each architecture's build.
const UNIVERSAL: &str = "universal-apple-darwin";
/// The libraries an executable may link: the system's own, on every Mac.
const SYSTEM_LIBRARIES: [&str; 2] = ["/usr/lib/", "/System/Library/Frameworks/"];

/// Builds `programs` (executables `bins`) for each of `triples` with the `dist` profile and
/// `macos` as their minimum, then joins each program's builds; the joined executables' folder.
pub fn build_universal(target_dir: &Path, triples: &[&str], programs: &[&str], bins: &[String], macos: (u32, u32)) -> Result<PathBuf, String> {
    for triple in triples {
        let mut build = cargo();
        // the compiler and the linker refuse, or link weakly, what is newer than the minimum
        build.env("MACOSX_DEPLOYMENT_TARGET", format!("{}.{}", macos.0, macos.1));
        build.args(["build", "--profile", "dist", "--target", triple]);
        for p in programs {
            build.args(["-p", p]);
        }
        run(build)?;
    }
    let out = target_dir.join(UNIVERSAL).join("dist");
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    for bin in bins {
        let mut lipo = Command::new("lipo");
        lipo.arg("-create").arg("-output").arg(out.join(bin));
        for triple in triples {
            lipo.arg(target_dir.join(triple).join("dist").join(bin));
        }
        run(lipo).map_err(|e| format!("{e} (lipo comes with the Xcode Command Line Tools: xcode-select --install)"))?;
    }
    Ok(out)
}

/// What one architecture's executable says about itself.
#[derive(Debug, PartialEq)]
struct Slice {
    arch: Architecture,
    /// The oldest macOS it runs on.
    minos: Option<(u32, u32)>,
    libraries: Vec<String>,
}

/// The newest minimum macOS of `exe`'s architectures; stops when one of `triples`' architectures
/// is missing, when one needs a macOS newer than `limit`, or when it links a library outside the
/// system's.
pub fn check(exe: &Path, triples: &[&str], limit: (u32, u32)) -> Result<(u32, u32), String> {
    let data = std::fs::read(exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    let slices = slices(&data).map_err(|e| format!("{}: {e}", exe.display()))?;
    verdict(&exe.display().to_string(), &slices, triples, limit)
}

fn verdict(exe: &str, slices: &[Slice], triples: &[&str], limit: (u32, u32)) -> Result<(u32, u32), String> {
    let wanted: Vec<Architecture> = triples.iter().map(|t| if t.starts_with("aarch64") { Architecture::Aarch64 } else { Architecture::X86_64 }).collect();
    let missing: Vec<String> = wanted.iter().filter(|a| !slices.iter().any(|s| s.arch == **a)).map(|a| format!("{a:?}")).collect();
    if !missing.is_empty() {
        return Err(format!("{exe} has no {} code: it is not a universal executable", missing.join(", ")));
    }
    let mut newest = (0, 0);
    for s in slices {
        let minos = s.minos.ok_or(format!("{exe} ({:?}) does not say its minimum macOS", s.arch))?;
        println!("macOS:   {exe} ({:?}) needs macOS {}.{} or newer", s.arch, minos.0, minos.1);
        if minos > limit {
            return Err(format!("{exe} ({:?}) needs macOS {}.{}, newer than the {}.{} the macOS package promises: it was not built with MACOSX_DEPLOYMENT_TARGET={}.{}", s.arch, minos.0, minos.1, limit.0, limit.1, limit.0, limit.1));
        }
        let outside: Vec<&str> = s.libraries.iter().map(String::as_str).filter(|l| !SYSTEM_LIBRARIES.iter().any(|p| l.starts_with(p))).collect();
        if !outside.is_empty() {
            return Err(format!("{exe} ({:?}) needs {}, which is not part of macOS (from Homebrew or another installation): the package would not start on other Macs", s.arch, outside.join(", ")));
        }
        newest = newest.max(minos);
    }
    Ok(newest)
}

/// The architectures of a universal executable, or of a single one.
fn slices(data: &[u8]) -> Result<Vec<Slice>, String> {
    match object::FileKind::parse(data).map_err(|e| e.to_string())? {
        object::FileKind::MachOFat32 => {
            let fat = MachOFatFile32::parse(data).map_err(|e| e.to_string())?;
            fat.arches().iter().map(|a| {
                let (offset, size) = (a.offset() as usize, a.size() as usize);
                slice(data.get(offset..offset + size).ok_or("a slice beyond the end of the file")?)
            }).collect()
        }
        object::FileKind::MachO64 => Ok(vec![slice(data)?]),
        kind => Err(format!("not a 64-bit Mach-O executable ({kind:?})")),
    }
}

fn slice(data: &[u8]) -> Result<Slice, String> {
    use object::read::macho::{LoadCommandVariant, MachHeader};
    use object::read::Object;
    let file = MachOFile64::<Endianness>::parse(data).map_err(|e| e.to_string())?;
    let endian = file.endian();
    let mut minos = None;
    let mut libraries = Vec::new();
    let mut commands = file.macho_header().load_commands(endian, data, 0).map_err(|e| e.to_string())?;
    while let Some(command) = commands.next().map_err(|e| e.to_string())? {
        match command.variant().map_err(|e| e.to_string())? {
            LoadCommandVariant::BuildVersion(b) if b.platform.get(endian) == object::macho::PLATFORM_MACOS => minos = Some(version(b.minos.get(endian))),
            LoadCommandVariant::VersionMin(v) if command.cmd() == object::macho::LC_VERSION_MIN_MACOSX => minos = Some(version(v.version.get(endian))),
            LoadCommandVariant::Dylib(d) => libraries.push(String::from_utf8_lossy(command.string(endian, d.dylib.name).map_err(|e| e.to_string())?).to_string()),
            _ => {}
        }
    }
    Ok(Slice { arch: file.architecture(), minos, libraries })
}

/// A Mach-O version, `xxxx.yy.zz` in nibbles, as (major, minor).
fn version(v: u32) -> (u32, u32) {
    (v >> 16, (v >> 8) & 0xFF)
}

/// Moves each executable `fill.programs` of `folder` into its app bundle there, with its
/// `Info.plist` and icon, and signs the bundle ad hoc (Apple Silicon runs no unsigned code).
pub fn bundles(root: &Path, folder: &Path, fill: &Fill, macos: (u32, u32)) -> Result<(), String> {
    for (i, bin) in fill.programs.iter().enumerate() {
        let app = folder.join(format!("{}.app", BUNDLES[i]));
        let contents = app.join("Contents");
        for d in ["MacOS", "Resources"] {
            std::fs::create_dir_all(contents.join(d)).map_err(|e| format!("{}: {e}", contents.display()))?;
        }
        std::fs::rename(folder.join(bin), contents.join("MacOS").join(bin)).map_err(|e| format!("{bin}: {e}"))?;
        let id = if i == 0 { env!("PRODUCT_BUNDLE_ID").to_string() } else { format!("{}.editor", env!("PRODUCT_BUNDLE_ID")) };
        std::fs::write(contents.join("Info.plist"), info_plist(bin, BUNDLES[i], &id, CATEGORIES[i], &fill.version, macos)).map_err(|e| e.to_string())?;
        std::fs::write(contents.join("PkgInfo"), "APPL????").map_err(|e| e.to_string())?;
        std::fs::write(contents.join("Resources").join(format!("{bin}.icns")), icns(&root.join("assets/icons").join(format!("{bin}.png")))?).map_err(|e| e.to_string())?;
        let mut sign = Command::new("codesign");
        sign.args(["--force", "--deep", "--sign", "-"]).arg(&app);
        run(sign)?;
        println!("bundle:  {}", app.display());
    }
    Ok(())
}

fn info_plist(bin: &str, name: &str, id: &str, category: &str, version: &str, macos: (u32, u32)) -> String {
    let escape = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let entries = [
        ("CFBundleDevelopmentRegion", "en".to_string()),
        ("CFBundleDisplayName", name.to_string()),
        ("CFBundleExecutable", bin.to_string()),
        ("CFBundleIconFile", format!("{bin}.icns")),
        ("CFBundleIdentifier", id.to_string()),
        ("CFBundleInfoDictionaryVersion", "6.0".to_string()),
        ("CFBundleName", name.to_string()),
        ("CFBundlePackageType", "APPL".to_string()),
        ("CFBundleShortVersionString", version.to_string()),
        ("CFBundleVersion", version.to_string()),
        ("LSApplicationCategoryType", category.to_string()),
        ("LSMinimumSystemVersion", format!("{}.{}", macos.0, macos.1)),
        ("NSHumanReadableCopyright", app_resources::COPYRIGHT.to_string()),
    ];
    let mut plist = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n");
    for (key, value) in entries {
        plist += &format!("\t<key>{key}</key>\n\t<string>{}</string>\n", escape(&value));
    }
    plist += "\t<key>NSHighResolutionCapable</key>\n\t<true/>\n</dict>\n</plist>\n";
    plist
}

/// The icon types of an `.icns` holding PNG pictures, and their sizes in pixels.
const ICNS: [(&[u8; 4], u32); 10] = [(b"icp4", 16), (b"icp5", 32), (b"ic11", 32), (b"ic12", 64), (b"ic07", 128), (b"ic13", 256), (b"ic08", 256), (b"ic14", 512), (b"ic09", 512), (b"ic10", 1024)];

/// The `.icns` of `png_file`: each entry its type, its length (with this 8-byte head) and a PNG.
fn icns(png_file: &Path) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    for (kind, size) in ICNS {
        let png = crate::icons::scaled(png_file, size)?;
        body.extend_from_slice(kind);
        body.extend_from_slice(&(png.len() as u32 + 8).to_be_bytes());
        body.extend_from_slice(&png);
    }
    let mut file = b"icns".to_vec();
    file.extend_from_slice(&(body.len() as u32 + 8).to_be_bytes());
    file.extend_from_slice(&body);
    Ok(file)
}

/// `folder` zipped into `zip` as the Finder does, under a top folder of its name.
pub fn ditto(folder: &Path, zip: &Path) -> Result<(), String> {
    let _ = std::fs::remove_file(zip);
    let mut c = Command::new("ditto");
    c.args(["-c", "-k", "--sequesterRsrc", "--keepParent"]).arg(folder).arg(zip);
    run(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 64-bit Mach-O header (`cputype`) with an LC_BUILD_VERSION for macOS `minos` and an
    /// LC_LOAD_DYLIB of `library`.
    fn macho(cputype: u32, minos: u32, library: &str) -> Vec<u8> {
        let mut name = library.as_bytes().to_vec();
        name.resize((name.len() + 1).next_multiple_of(8), 0);
        let dylib_size = 24 + name.len() as u32;
        let mut d = Vec::new();
        for v in [0xFEED_FACF, cputype, 0, 2, 2, 24 + dylib_size, 0, 0] {
            d.extend_from_slice(&u32::to_le_bytes(v));
        }
        for v in [0x32, 24, 1, minos, minos, 0] {
            d.extend_from_slice(&u32::to_le_bytes(v));
        }
        for v in [0xC, dylib_size, 24, 2, 0x10000, 0x10000] {
            d.extend_from_slice(&u32::to_le_bytes(v));
        }
        d.extend_from_slice(&name);
        d
    }

    fn fat(slices: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut d = Vec::new();
        for v in [0xCAFE_BABE, slices.len() as u32] {
            d.extend_from_slice(&v.to_be_bytes());
        }
        let mut offset = 4096;
        for (cputype, s) in slices {
            for v in [*cputype, 0, offset, s.len() as u32, 12] {
                d.extend_from_slice(&v.to_be_bytes());
            }
            offset += (s.len() as u32).next_multiple_of(4096);
        }
        for (_, s) in slices {
            d.resize(d.len().next_multiple_of(4096), 0);
            d.extend_from_slice(s);
        }
        d
    }

    const ARM64: u32 = 0x0100_000C;
    const X86_64: u32 = 0x0100_0007;
    const TRIPLES: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];
    const SYSTEM: &str = "/usr/lib/libSystem.B.dylib";

    #[test]
    fn the_slices_of_a_universal_executable() {
        let data = fat(&[(X86_64, macho(X86_64, 0x000B_0000, SYSTEM)), (ARM64, macho(ARM64, 0x000B_0000, "/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit"))]);
        let s = slices(&data).unwrap();
        assert_eq!(s.iter().map(|s| s.arch).collect::<Vec<_>>(), [Architecture::X86_64, Architecture::Aarch64]);
        assert_eq!((s[0].minos, s[0].libraries.as_slice()), (Some((11, 0)), [SYSTEM.to_string()].as_slice()));
        assert_eq!(verdict("game", &s, &TRIPLES, (11, 0)), Ok((11, 0)));
    }

    #[test]
    fn what_the_check_refuses() {
        let one = slices(&macho(ARM64, 0x000B_0000, SYSTEM)).unwrap();
        assert!(verdict("game", &one, &TRIPLES, (11, 0)).unwrap_err().contains("X86_64"));
        let newer = slices(&fat(&[(X86_64, macho(X86_64, 0x000E_0000, SYSTEM)), (ARM64, macho(ARM64, 0x000B_0000, SYSTEM))])).unwrap();
        assert!(verdict("game", &newer, &TRIPLES, (11, 0)).unwrap_err().contains("needs macOS 14.0"));
        let brew = slices(&fat(&[(X86_64, macho(X86_64, 0x000B_0000, SYSTEM)), (ARM64, macho(ARM64, 0x000B_0000, "/opt/homebrew/lib/libSDL2.dylib"))])).unwrap();
        assert!(verdict("game", &brew, &TRIPLES, (11, 0)).unwrap_err().contains("/opt/homebrew/lib/libSDL2.dylib"));
    }

    #[test]
    fn the_icon_file() {
        let icon = icns(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/icons/sml26gc.png")).unwrap();
        assert_eq!(&icon[..4], b"icns");
        assert_eq!(u32::from_be_bytes(icon[4..8].try_into().unwrap()) as usize, icon.len());
        let (mut at, mut kinds) = (8, Vec::new());
        while at < icon.len() {
            kinds.push(icon[at..at + 4].to_vec());
            assert_eq!(&icon[at + 8..at + 12], b"\x89PNG");
            at += u32::from_be_bytes(icon[at + 4..at + 8].try_into().unwrap()) as usize;
        }
        assert_eq!(at, icon.len());
        assert_eq!(kinds, ICNS.map(|(k, _)| k.to_vec()));
    }

    #[test]
    fn the_info_plist() {
        let plist = info_plist("sml26gc", "SML2 6GC", "io.example.game", CATEGORIES[0], "1.2.0", (11, 0));
        for line in ["<key>CFBundleExecutable</key>\n\t<string>sml26gc</string>", "<key>LSMinimumSystemVersion</key>\n\t<string>11.0</string>", "<key>CFBundleShortVersionString</key>\n\t<string>1.2.0</string>", "<key>NSHighResolutionCapable</key>\n\t<true/>"] {
            assert!(plist.contains(line), "{line}");
        }
    }
}
