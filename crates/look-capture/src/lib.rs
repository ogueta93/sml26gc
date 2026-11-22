//! One capture job, run the same way by the editor's threads, by its window's thread and by the
//! browser's capture workers (`capture-worker`): an object type's picture in a level, or the
//! sprite tiles it shows over its animation. Jobs and results travel as lines of the look
//! cache's format (`editor/src/domain/look_cache.rs`): `p level kind`, `m level` and
//! `t level kind mario` for a job; `level kind -`, `level kind w h dx dy sets pixels`,
//! `m level tiles` or `t level kind tiles` for a result (tiles in hex, no spaces).
use cart_model::art::{baseline, capture_look_with, mario_tiles, object_tiles_with, Baseline, Look};
use cart_model::levels::LEVELS;
use cart_model::GameDriver;
use gb_core::Gb;
use std::sync::{Arc, OnceLock};

/// Frames an object's tiles are watched for.
const TILE_FRAMES: u32 = 600;
/// The capture consoles' sound rate: nobody hears it, and fewer samples are quicker.
const CAPTURE_SOUND_RATE: u32 = 1000;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Job {
    /// The picture of object type `.1` in level `.0`.
    Picture(usize, u8),
    /// The tiles Mario shows in level `.0` with no object (`art::mario_tiles`): a few seconds,
    /// so played once per level and handed to its `Tiles` jobs.
    Mario(usize),
    /// The tiles object type `.1` shows in level `.0`, with the level's `Mario` tiles.
    Tiles(usize, u8, Vec<u8>),
}

#[derive(Clone, PartialEq, Debug)]
pub enum Done {
    /// None: the object draws nothing.
    Picture(Option<Look>),
    Mario(Vec<u8>),
    Tiles(Vec<u8>),
}

/// Each level's start without objects, played once and shared by the captures in that level:
/// for pictures, and longer (watching the sprites) for tiles.
pub type Baselines = Arc<Vec<[OnceLock<Option<Baseline>>; 2]>>;

pub fn new_baselines() -> Baselines {
    Arc::new((0..LEVELS).map(|_| [OnceLock::new(), OnceLock::new()]).collect())
}

/// A console on the world map of `rom`, ready to capture.
pub struct Capturer {
    g: Gb,
    rom: Arc<Vec<u8>>,
    original: Arc<Vec<u8>>,
    baselines: Baselines,
}

impl Capturer {
    /// None when `rom` does not boot to its world map.
    pub fn new(rom: Arc<Vec<u8>>, original: Arc<Vec<u8>>, baselines: Baselines) -> Option<Capturer> {
        Capturer::seen_or_not(rom, original, baselines, true)
    }

    fn seen_or_not(rom: Arc<Vec<u8>>, original: Arc<Vec<u8>>, baselines: Baselines, unseen: bool) -> Option<Capturer> {
        let mut g = Gb::new(rom.to_vec(), CAPTURE_SOUND_RATE).ok()?;
        g.set_unseen(unseen);
        g.boot_to_map().then_some(Capturer { g, rom, original, baselines })
    }

    pub fn run(&self, job: &Job) -> Done {
        let base = |l: usize, frames: u32| self.baselines.get(l)?[usize::from(frames > 0)].get_or_init(|| baseline(&self.g, &self.rom, &self.original, l, frames)).as_ref();
        match job {
            &Job::Picture(l, k) => Done::Picture(base(l, 0).and_then(|b| capture_look_with(&self.g, &self.rom, &self.original, l, k, b))),
            &Job::Mario(l) => Done::Mario(mario_tiles(&self.g, &self.rom, &self.original, l)),
            Job::Tiles(l, k, mario) => Done::Tiles(base(*l, TILE_FRAMES).map(|b| object_tiles_with(&self.g, &self.rom, &self.original, *l, *k, b, mario)).unwrap_or_default()),
        }
    }
}

fn hex(tiles: &[u8]) -> String {
    tiles.iter().map(|b| format!("{b:02x}")).collect()
}

fn parse_hex(hex: &str) -> Option<Vec<u8>> {
    (0..hex.len() / 2).map(|i| u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()).collect()
}

pub fn job_line(job: &Job) -> String {
    match job {
        Job::Picture(l, k) => format!("p {l} {k}"),
        Job::Mario(l) => format!("m {l}"),
        Job::Tiles(l, k, mario) => format!("t {l} {k} {}", hex(mario)),
    }
}

pub fn parse_job(line: &str) -> Option<Job> {
    let f: Vec<&str> = line.split(' ').collect();
    let l = f.get(1)?.parse().ok()?;
    match *f.first()? {
        "p" => Some(Job::Picture(l, f.get(2)?.parse().ok()?)),
        "m" => Some(Job::Mario(l)),
        "t" => Some(Job::Tiles(l, f.get(2)?.parse().ok()?, parse_hex(f.get(3).copied().unwrap_or(""))?)),
        _ => None,
    }
}

/// A picture's line: `level kind -`, or `level kind w h dx dy sets pixels` with one character
/// per pixel (`.` see-through, `0`-`3` a shade).
pub fn picture_line(l: usize, k: u8, look: Option<&Look>) -> String {
    match look {
        None => format!("{l} {k} -"),
        Some(lk) => {
            let px: String = lk.px.iter().map(|p| p.map_or('.', |s| (b'0' + s) as char)).collect();
            format!("{l} {k} {} {} {} {} {} {px}", lk.w, lk.h, lk.dx, lk.dy, lk.sets)
        }
    }
}

/// Tiles' line: `t level kind tiles` (hex, no spaces).
pub fn tiles_line(l: usize, k: u8, tiles: &[u8]) -> String {
    format!("t {l} {k} {}", hex(tiles))
}

pub fn done_line(job: &Job, done: &Done) -> String {
    match (job, done) {
        (&Job::Picture(l, k), Done::Picture(look)) => picture_line(l, k, look.as_ref()),
        (&Job::Picture(l, k), _) => picture_line(l, k, None),
        (&Job::Mario(l), Done::Mario(t)) => format!("m {l} {}", hex(t)),
        (&Job::Mario(l), _) => format!("m {l} "),
        (&Job::Tiles(l, k, _), Done::Tiles(t)) => tiles_line(l, k, t),
        (&Job::Tiles(l, k, _), _) => tiles_line(l, k, &[]),
    }
}

/// A result line's job and result; None when damaged.
pub fn parse_done(line: &str) -> Option<(Job, Done)> {
    if line.starts_with("t ") {
        let (l, k, t) = parse_tiles(line)?;
        return Some((Job::Tiles(l, k, Vec::new()), Done::Tiles(t)));
    }
    if let Some(rest) = line.strip_prefix("m ") {
        let (l, t) = rest.split_once(' ')?;
        return Some((Job::Mario(l.parse().ok()?), Done::Mario(parse_hex(t)?)));
    }
    let (l, k, look) = parse_picture(line)?;
    Some((Job::Picture(l, k), Done::Picture(look)))
}

pub fn parse_tiles(line: &str) -> Option<(usize, u8, Vec<u8>)> {
    let f: Vec<&str> = line.split(' ').collect();
    let (l, k) = (f.get(1)?.parse().ok()?, f.get(2)?.parse().ok()?);
    Some((l, k, parse_hex(f.get(3).copied().unwrap_or(""))?))
}

pub fn parse_picture(line: &str) -> Option<(usize, u8, Option<Look>)> {
    let f: Vec<&str> = line.split(' ').collect();
    let (l, k) = (f.first()?.parse().ok()?, f.get(1)?.parse().ok()?);
    if f.get(2) == Some(&"-") {
        return Some((l, k, None));
    }
    let (w, h): (usize, usize) = (f.get(2)?.parse().ok()?, f.get(3)?.parse().ok()?);
    let (dx, dy) = (f.get(4)?.parse().ok()?, f.get(5)?.parse().ok()?);
    let sets = f.get(6)?.parse().ok()?;
    let px: Vec<Option<u8>> = f.get(7)?.bytes().map(|b| (b != b'.').then(|| b.wrapping_sub(b'0'))).collect();
    (px.len() == w * h && px.iter().all(|p| p.is_none_or(|s| s < 4))).then_some((l, k, Some(Look { w, h, px, dx, dy, sets })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jobs_and_results_go_to_lines_and_back() {
        for job in [Job::Picture(3, 31), Job::Mario(2), Job::Tiles(0, 127, vec![0x10, 0x2F]), Job::Tiles(0, 1, vec![])] {
            assert_eq!(parse_job(&job_line(&job)), Some(job));
        }
        let look = Look { w: 2, h: 2, px: vec![None, Some(1), Some(3), Some(0)], dx: -4, dy: 8, sets: 5 };
        let results = [(Job::Picture(3, 31), Done::Picture(Some(look))), (Job::Picture(3, 32), Done::Picture(None)), (Job::Mario(5), Done::Mario(vec![0x01, 0x20])), (Job::Mario(6), Done::Mario(vec![])), (Job::Tiles(4, 17, vec![]), Done::Tiles(vec![0xB0, 0x0A])), (Job::Tiles(4, 18, vec![]), Done::Tiles(vec![]))];
        for (job, done) in results {
            assert_eq!(parse_done(&done_line(&job, &done)), Some((job, done)));
        }
        assert_eq!(done_line(&Job::Picture(3, 31), &Done::Picture(None)), "3 31 -");
        assert_eq!(parse_done("3 31 2 2 0 0 0 .1"), None);
        assert_eq!(parse_done("t 1 2 zz"), None);
        assert_eq!(parse_job("x 1 2"), None);
    }

    /// Unseen consoles capture what seen ones do: pictures and tiles of every type of a level.
    #[test]
    fn unseen_consoles_capture_the_same() {
        let Some(rom) = std::env::var("SML2_ROM").ok().and_then(|p| std::fs::read(p).ok()) else {
            eprintln!("SML2_ROM not set: test skipped");
            return;
        };
        let rom = Arc::new(rom);
        let unseen = Capturer::seen_or_not(rom.clone(), rom.clone(), new_baselines(), true).unwrap();
        let seen = Capturer::seen_or_not(rom.clone(), rom.clone(), new_baselines(), false).unwrap();
        for k in 0..128u8 {
            assert_eq!(unseen.run(&Job::Picture(4, k)), seen.run(&Job::Picture(4, k)), "picture of type {k}");
        }
        let mario = unseen.run(&Job::Mario(4));
        assert_eq!(mario, seen.run(&Job::Mario(4)));
        let Done::Mario(mario) = mario else { panic!("not Mario's tiles") };
        for k in [0x1F, 0x20, 0x2C] {
            assert_eq!(unseen.run(&Job::Tiles(4, k, mario.clone())), seen.run(&Job::Tiles(4, k, mario.clone())), "tiles of type {k}");
        }
    }
}
