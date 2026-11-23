//! The Linux packages: the executables built in an Ubuntu 18.04 container of the computer's own
//! architecture (so they need no glibc newer than its 2.27), their libraries checked, the
//! portable `.tar.gz` and the `.deb`.
use crate::{files_in, run, Fill};
use std::path::{Path, PathBuf};
use std::process::Command;

const IMAGE: &str = "sml26gc-build-linux";
/// The crates downloaded by the container's builds, kept between them.
const CARGO_VOLUME: &str = "sml26gc-cargo";
/// The container's build folder, apart from the computer's own (`target/release`...).
const CONTAINER_TARGET: &str = "target/linux-container";
/// The file beside the installed executables that sends their data to the user's folder
/// (`host::paths::INSTALLED_MARKER`).
const INSTALLED_MARKER: &str = "installed";
/// The `.deb`'s icon size, scaled down from `assets/icons/*.png`.
const ICON: u32 = 256;

/// Builds `programs` for `triple` with the `dist` profile in the image of `rust`'s version and
/// this computer's architecture (made first when missing); their folder.
pub fn build_in_container(root: &Path, rust: &str, triple: &str, programs: &[&str]) -> Result<PathBuf, String> {
    let docker = Command::new("docker").args(["version", "--format", "{{.Server.Version}}"]).output();
    if !docker.as_ref().is_ok_and(|o| o.status.success()) {
        return Err("the Linux packages are built in a Docker container, and Docker does not answer. Install it (Ubuntu: sudo apt install docker.io), let your user use it (sudo usermod -aG docker $USER, then log out and in) and try again (docs/DEVELOPMENT.md, \"Building for Linux\")".into());
    }
    let image = format!("{IMAGE}:{rust}-{}", std::env::consts::ARCH);
    let mut build = Command::new("docker");
    build.args(["build", "--tag", &image, "--build-arg", &format!("RUST_VERSION={rust}")]).arg(root.join("packaging/linux"));
    run(build)?;
    let id = |flag: &str| Command::new("id").arg(flag).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).ok_or(format!("id {flag} failed"));
    let mut cargo = Command::new("docker");
    cargo.args(["run", "--rm", "--user", &format!("{}:{}", id("-u")?, id("-g")?)]);
    cargo.arg("--volume").arg(format!("{}:/work", root.display()));
    cargo.args(["--volume", &format!("{CARGO_VOLUME}:/cargo"), "--env", &format!("CARGO_TARGET_DIR=/work/{CONTAINER_TARGET}"), &image]);
    cargo.args(["cargo", "build", "--profile", "dist", "--target", triple]);
    for p in programs {
        cargo.args(["-p", p]);
    }
    run(cargo)?;
    Ok(root.join(CONTAINER_TARGET).join(triple).join("dist"))
}

/// The newest glibc version `exe` needs; stops when it needs a library outside `allowed` or a
/// glibc newer than `limit`.
pub fn check_libraries(exe: &Path, allowed: &[&str], limit: (u32, u32)) -> Result<(u32, u32), String> {
    use object::read::elf::{Dyn, ElfFile64};
    let data = std::fs::read(exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    let what = |e: object::Error| format!("{}: {e}", exe.display());
    let file = ElfFile64::<object::Endianness>::parse(&*data).map_err(what)?;
    let (endian, sections) = (file.endian(), file.elf_section_table());
    let mut needed = Vec::new();
    if let Some((entries, link)) = sections.dynamic(endian, &*data).map_err(what)? {
        let strings = sections.strings(endian, &*data, link).map_err(what)?;
        for d in entries.iter().filter(|d| d.tag32(endian) == Some(object::elf::DT_NEEDED)) {
            needed.push(String::from_utf8_lossy(d.string(endian, strings).map_err(what)?).to_string());
        }
    }
    let outside: Vec<&String> = needed.iter().filter(|l| !allowed.contains(&l.as_str())).collect();
    if !outside.is_empty() {
        return Err(format!("{} needs {}, which the Linux read-me does not name (packaging/linux/README.txt and the .deb's dependencies)", exe.display(), outside.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
    }
    let mut glibc = (2, 2);
    if let Some((mut needs, link)) = sections.gnu_verneed(endian, &*data).map_err(what)? {
        let strings = sections.strings(endian, &*data, link).map_err(what)?;
        while let Some((_, mut versions)) = needs.next().map_err(what)? {
            while let Some(v) = versions.next().map_err(what)? {
                if let Some(version) = glibc_version(&String::from_utf8_lossy(v.name(endian, strings).map_err(what)?)) {
                    glibc = glibc.max(version);
                }
            }
        }
    }
    if glibc > limit {
        return Err(format!("{} needs glibc {}.{}, newer than the {}.{} the Linux packages promise: it was not built in the container", exe.display(), glibc.0, glibc.1, limit.0, limit.1));
    }
    Ok(glibc)
}

/// `GLIBC_2.31` as (2, 31); None for other version names (`GLIBC_PRIVATE`, `GCC_3.0`).
fn glibc_version(name: &str) -> Option<(u32, u32)> {
    let mut parts = name.strip_prefix("GLIBC_")?.split('.');
    Some((parts.next()?.parse().ok()?, parts.next().map_or(Some(0), |p| p.parse().ok())?))
}

/// `folder`'s files into `tar` (gzip), under a top folder `top`; `programs` executable.
pub fn tar_folder(folder: &Path, top: &str, tar: &Path, programs: &[String]) -> Result<(), String> {
    let file = std::fs::File::create(tar).map_err(|e| format!("{}: {e}", tar.display()))?;
    let mut w = tar::Builder::new(flate2::write::GzEncoder::new(file, flate2::Compression::best()));
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let files = files_in(folder)?;
    let mut dirs = std::collections::BTreeSet::from([top.to_string()]);
    for (_, rel) in &files {
        let mut parent = Path::new(rel).parent();
        while let Some(p) = parent.filter(|p| !p.as_os_str().is_empty()) {
            dirs.insert(format!("{top}/{}", p.to_string_lossy()));
            parent = p.parent();
        }
    }
    let header = |mode: u32, size: u64, kind: tar::EntryType| {
        let mut h = tar::Header::new_gnu();
        h.set_mode(mode);
        h.set_size(size);
        h.set_mtime(now);
        h.set_entry_type(kind);
        h
    };
    for d in &dirs {
        let mut h = header(0o755, 0, tar::EntryType::Directory);
        w.append_data(&mut h, format!("{d}/"), std::io::empty()).map_err(|e| e.to_string())?;
    }
    for (f, rel) in files {
        let data = std::fs::read(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        let mode = if programs.contains(&rel) { 0o755 } else { 0o644 };
        let mut h = header(mode, data.len() as u64, tar::EntryType::Regular);
        w.append_data(&mut h, format!("{top}/{rel}"), &*data).map_err(|e| e.to_string())?;
    }
    w.into_inner().and_then(|gz| gz.finish()).map_err(|e| e.to_string())?;
    Ok(())
}

/// The `.deb` of the package `folder`, built with `dpkg-deb` from a staging tree in `target/deb/`.
pub fn deb(root: &Path, folder: &Path, fill: &Fill, arch: &str) -> Result<PathBuf, String> {
    let id = env!("PRODUCT_ID");
    if Command::new("dpkg-deb").arg("--version").output().is_err() {
        return Err("dpkg-deb is needed to write the .deb (Debian and Ubuntu have it in the dpkg package; Fedora: sudo dnf install dpkg)".into());
    }
    let name = format!("{id}_{}_{arch}", fill.version);
    let stage = root.join("target/deb").join(&name);
    if stage.exists() {
        std::fs::remove_dir_all(&stage).map_err(|e| format!("{}: {e}", stage.display()))?;
    }
    let lib = stage.join("usr/lib").join(id);
    let doc = stage.join("usr/share/doc").join(id);
    let made = |p: &Path| std::fs::create_dir_all(p).map_err(|e| format!("{}: {e}", p.display()));
    for d in [&lib, &doc, &stage.join("DEBIAN"), &stage.join("usr/bin"), &stage.join("usr/share/applications"), &stage.join(format!("usr/share/icons/hicolor/{ICON}x{ICON}/apps"))] {
        made(d)?;
    }
    for p in &fill.programs {
        std::fs::copy(folder.join(p), lib.join(p)).map_err(|e| format!("{p}: {e}"))?;
        symlink(&format!("../lib/{id}/{p}"), &stage.join("usr/bin").join(p))?;
        let desktop = std::fs::read_to_string(root.join("packaging/linux").join(format!("{p}.desktop"))).map_err(|e| format!("packaging/linux/{p}.desktop: {e}"))?;
        std::fs::write(stage.join("usr/share/applications").join(format!("{p}.desktop")), fill.text(&desktop)).map_err(|e| e.to_string())?;
        let icon = stage.join(format!("usr/share/icons/hicolor/{ICON}x{ICON}/apps/{p}.png"));
        std::fs::write(&icon, crate::icons::scaled(&root.join("assets/icons").join(format!("{p}.png")), ICON)?).map_err(|e| format!("{}: {e}", icon.display()))?;
    }
    std::fs::write(lib.join(INSTALLED_MARKER), "The programs installed by the package keep their files in ~/.local/share/sml26gc.\n").map_err(|e| e.to_string())?;
    for (rel_from, rel_to) in [("README.txt", "README.txt"), ("NO-CARTRIDGE.txt", "NO-CARTRIDGE.txt"), ("LEGAL.md", "LEGAL.md"), ("LICENSE.txt", "LICENSE.txt"), ("CHANGELOG.md", "CHANGELOG.md"), ("manuals", "manuals"), ("licenses", "licenses")] {
        copy_tree(&folder.join(rel_from), &doc.join(rel_to))?;
    }
    gzip(debian_changelog(&fill.version)?.as_bytes(), &doc.join("changelog.gz"))?;
    let copyright = std::fs::read_to_string(root.join("packaging/linux/copyright")).map_err(|e| format!("packaging/linux/copyright: {e}"))?;
    std::fs::write(doc.join("copyright"), fill.text(&copyright)).map_err(|e| e.to_string())?;
    let size_kb = files_in(&stage)?.iter().map(|(f, _)| std::fs::metadata(f).map_or(0, |m| m.len())).sum::<u64>().div_ceil(1024);
    let control = control(&fill.version, arch, fill.glibc.as_deref().ok_or("no glibc version for the .deb")?, size_kb);
    std::fs::write(stage.join("DEBIAN/control"), control).map_err(|e| e.to_string())?;
    set_modes(&stage, &fill.programs)?;
    let deb = root.join("dist").join(format!("{name}.deb"));
    let mut c = Command::new("dpkg-deb");
    c.args(["--root-owner-group", "-Zxz", "--build"]).arg(&stage).arg(&deb);
    run(c)?;
    Ok(deb)
}

/// The `.deb`'s `DEBIAN/control`.
fn control(version: &str, arch: &str, glibc: &str, size_kb: u64) -> String {
    format!(
        "Package: {}\nVersion: {version}\nArchitecture: {arch}\nMaintainer: {}\nInstalled-Size: {size_kb}\nDepends: libc6 (>= {glibc}), libasound2t64 | libasound2, libudev1, libgl1, libx11-6, libxi6, libxkbcommon0\nRecommends: xdg-desktop-portal | zenity\nSection: otherosfs\nPriority: optional\nHomepage: {}\nDescription: {}\n {}\n .\n {}\n",
        env!("PRODUCT_ID"),
        env!("PRODUCT_MAINTAINER"),
        env!("PRODUCT_SOURCE"),
        "emulator and level editor for Super Mario Land 2",
        "Plays Super Mario Land 2: 6 Golden Coins from the user's own cartridge file on\n an emulated Game Boy, and edits its levels, objects and world map into a copy\n of it.",
        "No game is included: the programs ask for a cartridge file the user owns.\n Not affiliated with or endorsed by Nintendo."
    )
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    if from.is_file() {
        return std::fs::copy(from, to).map(|_| ()).map_err(|e| format!("{}: {e}", from.display()));
    }
    for (f, rel) in files_in(from)? {
        let dest = to.join(&rel);
        std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::copy(&f, &dest).map_err(|e| format!("{}: {e}", f.display()))?;
    }
    Ok(())
}

/// The changelog Debian expects of a package (its format, one entry): what each version brings
/// is in `CHANGELOG.md` beside it.
fn debian_changelog(version: &str) -> Result<String, String> {
    let out = Command::new("date").args(["-R", "-u"]).output().map_err(|e| format!("date: {e}"))?;
    let date = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(format!("{} ({version}) unstable; urgency=medium\n\n  * Version {version}: see CHANGELOG.md.\n\n -- {}  {date}\n", env!("PRODUCT_ID"), env!("PRODUCT_MAINTAINER")))
}

fn gzip(data: &[u8], to: &Path) -> Result<(), String> {
    use std::io::Write;
    let mut gz = flate2::write::GzEncoder::new(std::fs::File::create(to).map_err(|e| e.to_string())?, flate2::Compression::best());
    gz.write_all(data).and_then(|_| gz.finish().map(|_| ())).map_err(|e| e.to_string())
}

#[cfg(unix)]
fn symlink(target: &str, link: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(target, link).map_err(|e| format!("{}: {e}", link.display()))
}

/// Directories and `programs` 0755, other files 0644 (what Debian expects whatever the umask).
#[cfg(unix)]
fn set_modes(stage: &Path, programs: &[String]) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let set = |p: &Path, mode: u32| std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).map_err(|e| format!("{}: {e}", p.display()));
    let mut dirs = vec![stage.to_path_buf()];
    while let Some(d) = dirs.pop() {
        set(&d, 0o755)?;
        for e in std::fs::read_dir(&d).map_err(|e| format!("{}: {e}", d.display()))?.flatten() {
            let p = e.path();
            if p.is_symlink() {
                continue;
            } else if p.is_dir() {
                dirs.push(p);
            } else {
                let executable = p.parent().is_some_and(|d| d.ends_with(Path::new("usr/lib").join(env!("PRODUCT_ID")))) && programs.iter().any(|g| p.ends_with(g));
                set(&p, if executable { 0o755 } else { 0o644 })?;
            }
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn symlink(_: &str, _: &Path) -> Result<(), String> {
    Err("the .deb is built on Linux".into())
}

#[cfg(not(unix))]
fn set_modes(_: &Path, _: &[String]) -> Result<(), String> {
    Err("the .deb is built on Linux".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glibc_versions() {
        assert_eq!((glibc_version("GLIBC_2.31"), glibc_version("GLIBC_2.2.5"), glibc_version("GLIBC_2.3")), (Some((2, 31)), Some((2, 2)), Some((2, 3))));
        assert_eq!((glibc_version("GLIBC_PRIVATE"), glibc_version("GCC_3.0")), (None, None));
    }

    #[test]
    fn the_control_file_names_the_architecture() {
        let c = control("1.3.0", "arm64", "2.27", 9000);
        assert!(c.contains("\nArchitecture: arm64\n") && c.contains("Depends: libc6 (>= 2.27),") && c.contains("libxkbcommon0\nRecommends") && c.starts_with("Package: sml26gc\nVersion: 1.3.0\n"), "{c}");
    }
}
