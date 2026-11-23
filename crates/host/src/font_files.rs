//! The fonts' files (`fonts::Face`): built into the programs on the desktop; in the browser
//! downloaded from the page's `fonts/` the first time a language needs one, and kept in the
//! storage for the next visits.
use fonts::Face;
use std::borrow::Cow;

/// The font file of `face` (None: the built-in font, or in the browser one not downloaded yet,
/// whose download this starts).
pub fn bytes(face: Face) -> Option<Cow<'static, [u8]>> {
    #[cfg(target_arch = "wasm32")]
    {
        let file = face.file()?;
        let kept = crate::storage::read(crate::web::fonts_dir().join(file)).ok();
        if kept.is_none() {
            crate::web::call_text(crate::web::TextCall::FetchFont, file);
        }
        kept.map(Cow::Owned)
    }
    #[cfg(not(target_arch = "wasm32"))]
    face.bytes().map(Cow::Borrowed)
}

/// Can `face` be drawn with now, without downloading anything?
pub fn ready(face: Face) -> bool {
    #[cfg(target_arch = "wasm32")]
    return face.file().is_none_or(|f| crate::storage::is_file(crate::web::fonts_dir().join(f)));
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = face;
        true
    }
}
