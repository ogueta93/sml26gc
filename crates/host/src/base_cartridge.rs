//! The original cartridge both programs work from, chosen once and checked at every start: the
//! tools patch one dump only (`SUPPORTED_SHA1`), whatever the file is called.
use crate::paths::base_cartridge_path;
use crate::texts::tr;
use i18n::Lang;
use std::path::{Path, PathBuf};

/// Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1): revision 1, byte $014C of the
/// header; revisions 0 and 2 differ and are refused.
pub const SUPPORTED_SHA1: &str = "96e3a314561fb394cdf51101f9178a32713c2313";

/// Why there is no usable base cartridge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaseProblem {
    NotChosen,
    Missing(PathBuf),
    Unreadable(PathBuf),
    /// Another file than the supported cartridge.
    Other(PathBuf),
}

impl BaseProblem {
    /// What to tell the person.
    pub fn text(&self, lang: Lang) -> String {
        match self {
            BaseProblem::NotChosen => tr(lang, "Choose your Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1) cartridge file.").to_string(),
            BaseProblem::Missing(p) => format!("{}\n{}", tr(lang, "The cartridge file is not there any more:"), p.display()),
            BaseProblem::Unreadable(p) => format!("{}\n{}", tr(lang, "The cartridge file could not be read:"), p.display()),
            BaseProblem::Other(p) => format!("{}\n{}", tr(lang, "This file is not Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1), the cartridge the tools work with (other revisions of the game are refused too):"), p.display()),
        }
    }
}

/// Is the file at `path` the supported cartridge?
pub fn check(path: &Path) -> Result<(), BaseProblem> {
    if !crate::storage::is_file(path) {
        return Err(BaseProblem::Missing(path.to_path_buf()));
    }
    let data = crate::storage::read(path).map_err(|_| BaseProblem::Unreadable(path.to_path_buf()))?;
    if sha1_hex(&data) == SUPPORTED_SHA1 {
        Ok(())
    } else {
        Err(BaseProblem::Other(path.to_path_buf()))
    }
}

/// The base cartridge chosen (in either program), checked.
pub fn base_cartridge() -> Result<PathBuf, BaseProblem> {
    let text = crate::storage::read_to_string(base_cartridge_path()).map_err(|_| BaseProblem::NotChosen)?;
    let t = text.trim();
    if t.is_empty() {
        return Err(BaseProblem::NotChosen);
    }
    let p = PathBuf::from(t);
    check(&p).map(|_| p)
}

/// Keep `path` as the base cartridge for both programs, if it is the supported one.
pub fn choose(path: &Path) -> Result<(), BaseProblem> {
    let checked = check(path);
    let path = if cfg!(target_arch = "wasm32") { &keep_arrived(path, checked.is_ok()) } else { path };
    checked?;
    let file = base_cartridge_path();
    crate::storage::write(&file, path.display().to_string()).map_err(|_| BaseProblem::Unreadable(file))
}

/// In the browser the chosen file arrives in a folder of its own: it is moved into the data folder
/// as the one cartridge kept when `accepted`, and dropped either way. Its new place.
fn keep_arrived(path: &Path, accepted: bool) -> PathBuf {
    let dir = crate::paths::data_dir().join("cartridge");
    let kept = dir.join(path.file_name().unwrap_or_default());
    if accepted {
        if let Ok(data) = crate::storage::read(path) {
            let _ = crate::storage::remove_dir_all(&dir);
            let _ = crate::storage::write(&kept, data);
        }
    }
    let _ = crate::storage::remove_file(path);
    kept
}

/// The path chosen before, even if it is no longer usable (to start a file chooser there).
pub fn chosen_before() -> Option<PathBuf> {
    let text = crate::storage::read_to_string(base_cartridge_path()).ok()?;
    let t = text.trim();
    (!t.is_empty()).then(|| PathBuf::from(t))
}

/// SHA-1 of `data` in lower-case hex (FIPS 180-1).
pub fn sha1_hex(data: &[u8]) -> String {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for block in msg.chunks(64) {
        let mut w = [0u32; 80];
        for (i, word) in block.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e]) {
            *x = x.wrapping_add(y);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_of_known_texts() {
        assert_eq!(sha1_hex(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(sha1_hex(&[b'a'; 1000]), "291e9a6c66994949b57ba5e650361e98fc36b1ba");
    }

    #[test]
    fn other_files_are_refused() {
        let dir = std::env::temp_dir().join(format!("nmb-base-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let f = dir.join("other.gb");
        std::fs::write(&f, [0u8; 64]).unwrap();
        assert_eq!(check(&f), Err(BaseProblem::Other(f.clone())));
        assert_eq!(check(&dir.join("none.gb")), Err(BaseProblem::Missing(dir.join("none.gb"))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_supported_cartridge_passes() {
        let Some(path) = std::env::var_os("RESEARCH_ROM") else { return };
        assert_eq!(check(Path::new(&path)), Ok(()));
    }
}
