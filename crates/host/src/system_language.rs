//! The language the system shows its own texts in (Windows: the user's display language; other
//! systems: the locale variables, and on macOS the user's preferred languages).
use i18n::Lang;

/// The system's language when it is one of `Lang::ALL`, English otherwise.
pub fn system_language() -> Lang {
    imp::system_language()
}

#[cfg(windows)]
mod imp {
    use i18n::Lang;
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

    /// Primary language ids (the low 10 bits of a LANGID) and their languages; any Chinese
    /// gives Simplified Chinese.
    const LANGS: [(u16, Lang); 10] = [(0x0A, Lang::Es), (0x11, Lang::Ja), (0x16, Lang::Pt), (0x0C, Lang::Fr), (0x07, Lang::De), (0x10, Lang::It), (0x21, Lang::Id), (0x19, Lang::Ru), (0x04, Lang::Zh), (0x12, Lang::Ko)];

    pub fn system_language() -> Lang {
        // SAFETY: no arguments, no pointers; it only reads the user's settings.
        let id = unsafe { GetUserDefaultUILanguage() };
        LANGS.iter().find(|l| l.0 == id & 0x3FF).map_or(Lang::En, |l| l.1)
    }
}

#[cfg(not(windows))]
mod imp {
    use i18n::Lang;

    /// The locale variables in the order POSIX gives them precedence for messages, with the
    /// user's preferred languages (macOS) before `LANG`, which Terminal sets from the region.
    pub fn system_language() -> Lang {
        let var = |name| std::env::var(name).ok();
        choose([var("LC_ALL"), var("LC_MESSAGES")], &preferred_languages(), var("LANG"))
    }

    fn choose(explicit: [Option<String>; 2], preferred: &[String], lang: Option<String>) -> Lang {
        let set = |l: &Option<String>| l.clone().filter(|l| !l.is_empty() && !["C", "POSIX"].contains(&l.split('.').next().unwrap_or("")));
        if let Some(l) = explicit.iter().find_map(set) {
            return Lang::from_tag(&l);
        }
        preferred.iter().find_map(|t| known(t)).or_else(|| set(&lang).map(|l| Lang::from_tag(&l))).unwrap_or(Lang::En)
    }

    /// The language a tag names, when the programs have it.
    fn known(tag: &str) -> Option<Lang> {
        let lang = Lang::from_tag(tag);
        (lang != Lang::En || tag.to_ascii_lowercase().starts_with("en")).then_some(lang)
    }

    /// The user's preferred languages, most preferred first, as tags (`es-ES`, `zh-Hans-CN`).
    #[cfg(target_os = "macos")]
    fn preferred_languages() -> Vec<String> {
        use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
        use core_foundation_sys::base::CFRelease;
        use core_foundation_sys::locale::CFLocaleCopyPreferredLanguages;
        use core_foundation_sys::string::{kCFStringEncodingUTF8, CFStringGetCString, CFStringRef};
        let mut tags = Vec::new();
        // SAFETY: the array comes from a Copy function, so it is ours to release, once; its
        // strings live as long as it does and are copied out before.
        unsafe {
            let array = CFLocaleCopyPreferredLanguages();
            if array.is_null() {
                return tags;
            }
            for i in 0..CFArrayGetCount(array) {
                let tag = CFArrayGetValueAtIndex(array, i) as CFStringRef;
                let mut buf = [0 as std::ffi::c_char; 64];
                if !tag.is_null() && CFStringGetCString(tag, buf.as_mut_ptr(), buf.len() as _, kCFStringEncodingUTF8) != 0 {
                    tags.push(std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned());
                }
            }
            CFRelease(array as _);
        }
        tags
    }

    #[cfg(not(target_os = "macos"))]
    fn preferred_languages() -> Vec<String> {
        Vec::new()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn some(s: &str) -> Option<String> {
            Some(s.to_string())
        }

        fn from_locale([all, messages, lang]: [Option<String>; 3]) -> Lang {
            choose([all, messages], &[], lang)
        }

        #[test]
        fn locale_variables_in_order() {
            assert_eq!(from_locale([None, None, some("es_ES.UTF-8")]), Lang::Es);
            assert_eq!(from_locale([some("fr_FR.UTF-8"), some("de_DE.UTF-8"), some("es_ES.UTF-8")]), Lang::Fr);
            assert_eq!(from_locale([some(""), some("de_DE.UTF-8"), some("es_ES.UTF-8")]), Lang::De);
            assert_eq!(from_locale([some("C.UTF-8"), None, some("ja_JP.UTF-8")]), Lang::Ja);
            assert_eq!(from_locale([some("POSIX"), None, None]), Lang::En);
            assert_eq!(from_locale([None, None, None]), Lang::En);
        }

        #[test]
        fn preferred_languages_between_the_variables() {
            let preferred = |tags: &[&str]| tags.iter().map(|t| t.to_string()).collect::<Vec<_>>();
            assert_eq!(choose([None, None], &preferred(&["ja-JP", "en-US"]), some("en_US.UTF-8")), Lang::Ja);
            assert_eq!(choose([None, None], &preferred(&["nl-NL", "de-DE"]), None), Lang::De);
            assert_eq!(choose([None, None], &preferred(&["en-GB", "es-ES"]), None), Lang::En);
            assert_eq!(choose([None, None], &preferred(&["nl-NL"]), some("es_ES.UTF-8")), Lang::Es);
            assert_eq!(choose([some("fr_FR.UTF-8"), None], &preferred(&["ja-JP"]), None), Lang::Fr);
            assert_eq!(choose([None, None], &preferred(&["zh-Hans-CN"]), None), Lang::Zh);
        }
    }
}
