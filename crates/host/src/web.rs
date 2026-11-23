//! The page's side of the web build: the functions `packaging/web/sml26gc.js` gives the program
//! (its `sml_*` imports). Text and bytes going to the page are a pointer and a length into the
//! program's memory; coming back, the program asks for the size first and hands the page a
//! buffer to fill.
use std::path::PathBuf;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn sml_now_ms() -> f64;
    fn sml_storage_next(sizes: *mut u32) -> i32;
    fn sml_storage_take(path: *mut u8, data: *mut u8) -> f64;
    fn sml_storage_put(path: *const u8, path_len: usize, data: *const u8, data_len: usize, time: f64);
    fn sml_storage_delete(path: *const u8, path_len: usize);
    fn sml_text_len(what: u32) -> i32;
    fn sml_text_take(what: u32, out: *mut u8);
    fn sml_call(what: u32, value: f64) -> f64;
    fn sml_call_text(what: u32, text: *const u8, len: usize) -> f64;
    fn sml_audio_push(samples: *const f32, len: usize, volume: f32);
    fn sml_pad_state(out: *mut f32, len: usize) -> i32;
    fn sml_ui_rect(what: u32, x: f32, y: f32, w: f32, h: f32);
    fn sml_download(name: *const u8, name_len: usize, data: *const u8, data_len: usize);
    fn sml_capture_start(rom: *const u8, rom_len: usize, original: *const u8, original_len: usize) -> f64;
}

/// The page's script checks it matches its own (`miniquad_add_plugin`'s version).
#[unsafe(no_mangle)]
pub extern "C" fn sml26gc_crate_version() -> u32 {
    1
}

/// A size the page gives for a file being deleted.
const DELETED: u32 = u32::MAX;

/// Milliseconds since 1970, the page's clock.
pub fn now_ms() -> f64 {
    unsafe { sml_now_ms() }
}

/// The next file another page (or the first load) changed: its path, its bytes (None: deleted)
/// and when.
pub fn storage_next() -> Option<(String, Option<Vec<u8>>, f64)> {
    let mut sizes = [0u32; 2];
    if unsafe { sml_storage_next(sizes.as_mut_ptr()) } == 0 {
        return None;
    }
    let mut path = vec![0u8; sizes[0] as usize];
    let mut data = vec![0u8; if sizes[1] == DELETED { 0 } else { sizes[1] as usize }];
    let time = unsafe { sml_storage_take(path.as_mut_ptr(), data.as_mut_ptr()) };
    Some((String::from_utf8_lossy(&path).into_owned(), (sizes[1] != DELETED).then_some(data), time))
}

pub fn storage_put(path: &str, data: &[u8], time: f64) {
    unsafe { sml_storage_put(path.as_ptr(), path.len(), data.as_ptr(), data.len(), time) }
}

pub fn storage_delete(path: &str) {
    unsafe { sml_storage_delete(path.as_ptr(), path.len()) }
}

/// Texts the page knows.
#[derive(Clone, Copy)]
#[repr(u32)]
pub enum Text {
    /// The page's address's arguments, one per line (`--level` then `3`).
    Arguments = 0,
    /// The browser's languages, one per line, preferred first.
    Languages = 1,
    /// The name of the next file the person chose or dropped, kept in the storage under `INCOMING`.
    Arrived = 2,
    /// The next result line of the editor's capture workers.
    Captured = 3,
}

pub fn text(what: Text) -> Option<String> {
    let n = unsafe { sml_text_len(what as u32) };
    if n < 0 {
        return None;
    }
    let mut buf = vec![0u8; n as usize];
    unsafe { sml_text_take(what as u32, buf.as_mut_ptr()) };
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// Things the page does or answers.
#[derive(Clone, Copy)]
#[repr(u32)]
pub enum Call {
    /// The sound's sample rate.
    SampleRate = 0,
    /// Seconds of sound waiting to be played.
    QueuedSeconds = 1,
    /// Vibrate for `value` milliseconds, where the device can.
    Vibrate = 2,
    /// 1 when the page can go fullscreen.
    CanFullscreen = 3,
    /// 1 while it is fullscreen.
    IsFullscreen = 4,
    /// Enter (1) or leave (0) fullscreen, from a key or the menu.
    SetFullscreen = 5,
    /// 1 on an iPhone or iPad's Safari not opened from the home screen.
    HomeScreenHint = 6,
    /// 1 when the device's main pointer is a mouse or trackpad (not a finger: phones and
    /// tablets, some of which claim a fine pointer besides).
    FinePointer = 7,
    /// 1 when the device has a touch screen.
    HasTouch = 8,
    /// 1 while a cartridge file is on its way: being chosen, read, or not taken yet.
    CartridgeComing = 9,
    /// The document has unsaved changes (1) or not (0): closing the page asks first then.
    Unsaved = 10,
    /// 1 on a Mac (its shortcuts use Cmd).
    IsMac = 11,
    /// How many capture workers wait for a job; -1 when every one failed to start.
    CaptureIdle = 12,
    /// Stop the capture workers of generation `value` (not newer ones).
    CaptureStop = 13,
}

pub fn call(what: Call, value: f64) -> f64 {
    unsafe { sml_call(what as u32, value) }
}

/// Things the page does with a text.
#[derive(Clone, Copy)]
#[repr(u32)]
pub enum TextCall {
    /// Open the file chooser for files with these extensions (`.gb,.gbc`).
    ChooseFile = 0,
    /// Download the font file of this name, once; it arrives in the storage under `FONTS`.
    FetchFont = 1,
    /// Open a page of the site: `<window name>` and its address, on two lines (a window of
    /// that name already open shows the page instead).
    OpenPage = 2,
    /// The keys (`KeyboardEvent.code`, space separated) whose Ctrl or Cmd combinations the
    /// program uses: the browser does not also act on them.
    OwnKeys = 3,
    /// Give this job line to an idle capture worker.
    CaptureJob = 4,
}

pub fn call_text(what: TextCall, text: &str) -> f64 {
    unsafe { sml_call_text(what as u32, text.as_ptr(), text.len()) }
}

/// Interleaved stereo samples at `volume` (0-1), played after what is waiting.
pub fn audio_push(samples: &[f32], volume: f32) {
    unsafe { sml_audio_push(samples.as_ptr(), samples.len(), volume) }
}

/// The first connected controller's standard-mapping buttons (0-16, 0 or 1) followed by its four
/// axes, or None when none is connected.
pub fn pad_state() -> Option<[f32; 21]> {
    let mut out = [0f32; 21];
    (unsafe { sml_pad_state(out.as_mut_ptr(), out.len()) } != 0).then_some(out)
}

/// Rectangles on the page (in its CSS pixels) where a tap is handled by the page itself.
#[derive(Clone, Copy)]
#[repr(u32)]
pub enum UiRect {
    /// The fullscreen button: a tap there enters or leaves fullscreen.
    Fullscreen = 0,
    /// The Browse button: a tap there opens the cartridge chooser.
    Browse = 1,
}

/// Where `what` is now; a zero size: nowhere.
pub fn ui_rect(what: UiRect, x: f32, y: f32, w: f32, h: f32) {
    unsafe { sml_ui_rect(what as u32, x, y, w, h) }
}

/// The folder the person's chosen and dropped files arrive in.
pub fn incoming_dir() -> PathBuf {
    PathBuf::from("/incoming")
}

/// Save `data` as a download named `name`, in the browser's download folder.
pub fn download(name: &str, data: &[u8]) {
    unsafe { sml_download(name.as_ptr(), name.len(), data.as_ptr(), data.len()) }
}

/// Start the editor's capture workers (`packaging/web/capture-worker.js`) for these cartridges,
/// stopping earlier ones: their generation, or None when the page cannot start workers.
pub fn capture_start(rom: &[u8], original: &[u8]) -> Option<u32> {
    let generation = unsafe { sml_capture_start(rom.as_ptr(), rom.len(), original.as_ptr(), original.len()) };
    (generation > 0.0).then_some(generation as u32)
}

/// Files written here are downloaded, not kept.
pub fn downloads_dir() -> PathBuf {
    PathBuf::from("/downloads")
}

/// Open `address` (a page of the site) in the window named `window` (`_blank`: a new one).
pub fn open_page(window: &str, address: &str) {
    call_text(TextCall::OpenPage, &format!("{window}\n{address}"));
}

/// The program's arguments as a page address's query: `--name value` as `name=value`, a flag
/// as `name` (the page turns them back, `Text::Arguments`).
pub fn query(args: &[String]) -> String {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let name = args[i].trim_start_matches("--");
        match args.get(i + 1).filter(|v| !v.starts_with("--")) {
            Some(v) => {
                out.push(format!("{name}={}", encode(v)));
                i += 2;
            }
            None => {
                out.push(name.to_string());
                i += 1;
            }
        }
    }
    out.join("&")
}

/// `text` safe in an address's query.
fn encode(text: &str) -> String {
    text.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// The folder downloaded fonts are kept in.
pub fn fonts_dir() -> PathBuf {
    PathBuf::from("/fonts")
}
