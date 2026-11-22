//! The fonts the editor and the game's start screen draw a language with when the window
//! library's built-in font (ASCII only) cannot: the files in `fonts/` at the repository root,
//! all under the SIL Open Font License (their licences beside them).
//!
//! - BIZ UDGothic: Japanese.
//! - Noto Sans SC: Simplified Chinese.
//! - Noto Sans KR: Korean, and Russian (its Cyrillic is proportional; BIZ UDGothic's is as
//!   wide as a Japanese character).
//!
//! The Latin languages are written without accents and keep the built-in font.
use i18n::Lang;

#[cfg(feature = "bundled")]
pub const BIZ_UD_GOTHIC: &[u8] = include_bytes!("../../../fonts/BIZUDGothic-Regular.ttf");
#[cfg(feature = "bundled")]
pub const NOTO_SANS_SC: &[u8] = include_bytes!("../../../fonts/NotoSansSC-Regular.otf");
#[cfg(feature = "bundled")]
pub const NOTO_SANS_KR: &[u8] = include_bytes!("../../../fonts/NotoSansKR-Regular.otf");

/// Which font a language is drawn with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    /// The window library's own.
    BuiltIn,
    BizUdGothic,
    NotoSansSc,
    NotoSansKr,
}

impl Face {
    pub fn of(lang: Lang) -> Face {
        match lang {
            Lang::Ja => Face::BizUdGothic,
            Lang::Zh => Face::NotoSansSc,
            Lang::Ko | Lang::Ru => Face::NotoSansKr,
            Lang::En | Lang::Es | Lang::Pt | Lang::Fr | Lang::De | Lang::It | Lang::Id => Face::BuiltIn,
        }
    }

    /// The font file's bytes (None: the built-in font).
    #[cfg(feature = "bundled")]
    pub fn bytes(self) -> Option<&'static [u8]> {
        match self {
            Face::BuiltIn => None,
            Face::BizUdGothic => Some(BIZ_UD_GOTHIC),
            Face::NotoSansSc => Some(NOTO_SANS_SC),
            Face::NotoSansKr => Some(NOTO_SANS_KR),
        }
    }

    /// The font file's name in `fonts/` (None: the built-in font).
    pub fn file(self) -> Option<&'static str> {
        match self {
            Face::BuiltIn => None,
            Face::BizUdGothic => Some("BIZUDGothic-Regular.ttf"),
            Face::NotoSansSc => Some("NotoSansSC-Regular.otf"),
            Face::NotoSansKr => Some("NotoSansKR-Regular.otf"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_names_are_in_the_fonts_folder() {
        for l in Lang::ALL {
            if let Some(f) = Face::of(l).file() {
                assert!(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fonts").join(f).is_file(), "{f}");
            }
        }
    }

    #[cfg(feature = "bundled")]
    #[test]
    fn every_language_has_a_face_and_the_files_are_fonts() {
        for l in Lang::ALL {
            if let Some(b) = Face::of(l).bytes() {
                assert!(b.starts_with(&[0, 1, 0, 0]) || b.starts_with(b"OTTO"), "{l:?}: not a font file");
            }
        }
        assert_eq!(Face::of(Lang::Fr), Face::BuiltIn);
    }
}
