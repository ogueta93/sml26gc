//! Object pictures captured from the game in the background, so the window never waits for
//! them: every (level, object type) the cache does not have yet, the shown level's first, and
//! the sprite tiles the game's levels' own objects show over their animation
//! (`art::object_tiles`, about a second each, after their level's Mario tiles, played once),
//! the shown level's first, right after the game's levels' own pictures, before every other
//! type's. Each job (`look_capture`) runs on threads with their own consoles
//! (one per spare processor core), in the browser on its capture workers, or, where these
//! cannot start, one job a frame on the window's thread (`pump`). The results are kept
//! (`look_cache`).
use crate::ports::CapturePool;
use cart_model::art::Look;
use cart_model::levels::LEVELS;
use cart_model::objects;
use look_capture::{job_line, new_baselines, parse_done, Capturer, Done, Job};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// Processor cores left to the window and the rest of the machine.
const SPARE_CORES: usize = 2;
const MOST_WORKERS: usize = 12;

/// Where the jobs run: on threads beside the window (the desktop), on the browser's capture
/// workers, or one a frame on the window's own thread.
#[derive(Clone, Copy)]
#[allow(dead_code)] // each platform builds the one it uses
pub enum Capture {
    Threads,
    Workers(&'static dyn CapturePool),
    Slices,
}

static CAPTURE: Mutex<Capture> = Mutex::new(Capture::Threads);

/// Where every loader started from now on captures (the composition root chooses).
pub fn set_capture(c: Capture) {
    if let Ok(mut now) = CAPTURE.lock() {
        *now = c;
    }
}

pub struct LookLoader {
    s: Shared,
    got: Receiver<(usize, u8, Option<Look>)>,
    tiles: Receiver<(usize, u8, Vec<u8>)>,
    /// How many pictures and tiles there were to capture when it started.
    totals: (usize, usize),
    /// On the window's thread: the one console, run by `pump` and `wait`.
    local: RefCell<Option<Capturer>>,
    /// On capture workers: the pool, this loader's generation of workers and the jobs sent.
    pool: Option<(&'static dyn CapturePool, u32)>,
    sent: Cell<usize>,
}

/// The work queues and where results go, shared by the threads.
#[derive(Clone)]
struct Shared {
    want: Arc<Mutex<VecDeque<(usize, u8)>>>,
    /// Pictures the window asked for: they go before the tiles too.
    urgent: Arc<Mutex<HashSet<(usize, u8)>>>,
    tiles_left: Arc<Mutex<usize>>,
    tile_jobs: Arc<Mutex<VecDeque<(usize, u8)>>>,
    /// Each level's Mario tiles for its tiles jobs: None while being captured.
    mario: Arc<Mutex<HashMap<usize, Option<Vec<u8>>>>>,
    own: Arc<HashSet<(usize, u8)>>,
    tx: Sender<(usize, u8, Option<Look>)>,
    tiles_tx: Sender<(usize, u8, Vec<u8>)>,
}

enum Next {
    Run(Job),
    /// Nothing to start now: other jobs are still on the tiles or on Mario's.
    Wait,
    Finished,
}

impl Shared {
    /// A picture the window asked for or a level's own before the tiles, then the tiles (each
    /// level's after its Mario tiles), then the rest of the pictures.
    fn next(&self) -> Next {
        let tiles_done = self.tiles_left.lock().map(|n| *n == 0).unwrap_or(true);
        let picture = self.want.lock().ok().and_then(|mut q| {
            let key = *q.front()?;
            (self.own.contains(&key) || self.urgent.lock().is_ok_and(|a| a.contains(&key)) || tiles_done).then(|| q.pop_front()).flatten()
        });
        if let Some((l, k)) = picture {
            return Next::Run(Job::Picture(l, k));
        }
        let (Ok(mut jobs), Ok(mut mario)) = (self.tile_jobs.lock(), self.mario.lock()) else { return Next::Finished };
        if jobs.is_empty() {
            return if self.want.lock().map(|q| q.is_empty()).unwrap_or(true) { Next::Finished } else { Next::Wait };
        }
        if let Some((l, k)) = jobs.iter().position(|(l, _)| matches!(mario.get(l), Some(Some(_)))).and_then(|i| jobs.remove(i)) {
            return Next::Run(Job::Tiles(l, k, mario[&l].clone().unwrap_or_default()));
        }
        match jobs.iter().find(|(l, _)| !mario.contains_key(l)) {
            Some(&(l, _)) => {
                mario.insert(l, None);
                Next::Run(Job::Mario(l))
            }
            None => Next::Wait,
        }
    }

    /// Hand a result to the window; false when it is gone.
    fn deliver(&self, job: Job, done: Done) -> bool {
        match (job, done) {
            (Job::Picture(l, k), Done::Picture(look)) => self.tx.send((l, k, look)).is_ok(),
            (Job::Mario(l), Done::Mario(t)) => {
                if let Ok(mut m) = self.mario.lock() {
                    m.insert(l, Some(t));
                }
                true
            }
            (Job::Tiles(l, k, _), Done::Tiles(t)) => {
                if let Ok(mut n) = self.tiles_left.lock() {
                    *n = n.saturating_sub(1);
                }
                self.tiles_tx.send((l, k, t)).is_ok()
            }
            _ => true,
        }
    }

    /// Run the next job on `c`.
    fn step(&self, c: &Capturer) -> Next {
        match self.next() {
            Next::Run(job) => {
                let done = c.run(&job);
                if self.deliver(job.clone(), done) { Next::Run(job) } else { Next::Finished }
            }
            next => next,
        }
    }
}

/// Move the ones of `keys` that `q` has to its front, in the order of `keys`.
fn to_front(q: &mut VecDeque<(usize, u8)>, keys: &[(usize, u8)]) {
    let wanted: HashSet<&(usize, u8)> = keys.iter().collect();
    if !q.iter().any(|k| wanted.contains(k)) {
        return;
    }
    let (found, rest): (Vec<_>, Vec<_>) = q.drain(..).partition(|k| wanted.contains(k));
    let mut found: HashSet<(usize, u8)> = found.into_iter().collect();
    q.extend(keys.iter().filter(|k| found.remove(k)).copied());
    q.extend(rest);
}

impl LookLoader {
    /// Start capturing, in `rom`, what is not in `known`: each level's own object types
    /// first, then the tiles of the game's levels' own types not in `known_tiles`, then every
    /// type in every level (a picture asked for with `prefer` comes first all along).
    pub fn start(rom: Vec<u8>, original: Vec<u8>, known: &HashSet<(usize, u8)>, known_tiles: &HashSet<(usize, u8)>) -> LookLoader {
        let capture = CAPTURE.lock().map(|c| *c).unwrap_or(Capture::Threads);
        LookLoader::start_with(capture, rom, original, known, known_tiles)
    }

    fn start_with(capture: Capture, rom: Vec<u8>, original: Vec<u8>, known: &HashSet<(usize, u8)>, known_tiles: &HashSet<(usize, u8)>) -> LookLoader {
        let mut native: Vec<(usize, u8)> = (0..LEVELS).flat_map(|l| objects::read(&original, l).0.into_iter().map(move |o| (l, o.kind))).collect();
        let mut seen_tiles = known_tiles.clone();
        native.retain(|key| seen_tiles.insert(*key));
        let tiles_left = native.len();
        let mut order: Vec<(usize, u8)> = (0..LEVELS).flat_map(|l| objects::read(&rom, l).0.into_iter().map(move |o| (l, o.kind))).collect();
        order.extend((0..LEVELS).flat_map(|l| (0..128u8).map(move |k| (l, k))));
        let mut seen = known.clone();
        order.retain(|key| seen.insert(*key));
        let totals = (order.len(), tiles_left);
        let (tx, got) = channel();
        let (tiles_tx, tiles) = channel();
        let s = Shared {
            want: Arc::new(Mutex::new(order.into_iter().collect())),
            urgent: Arc::new(Mutex::new(HashSet::new())),
            tiles_left: Arc::new(Mutex::new(tiles_left)),
            tile_jobs: Arc::new(Mutex::new(native.into_iter().collect())),
            mario: Arc::new(Mutex::new(HashMap::new())),
            own: Arc::new((0..LEVELS).flat_map(|l| objects::read(&rom, l).0.into_iter().map(move |o| (l, o.kind))).collect()),
            tx,
            tiles_tx,
        };
        let mut loader = LookLoader { s, got, tiles, totals, local: RefCell::new(None), pool: None, sent: Cell::new(0) };
        let (rom, original) = (Arc::new(rom), Arc::new(original));
        match capture {
            Capture::Threads => {
                let baselines = new_baselines();
                let workers = std::thread::available_parallelism().map(|n| n.get().saturating_sub(SPARE_CORES)).unwrap_or(1).clamp(1, MOST_WORKERS);
                for _ in 0..workers {
                    let (s, rom, original, baselines) = (loader.s.clone(), rom.clone(), original.clone(), baselines.clone());
                    std::thread::spawn(move || {
                        let Some(c) = Capturer::new(rom, original, baselines) else { return };
                        loop {
                            match s.step(&c) {
                                Next::Run(_) => {}
                                Next::Wait => std::thread::sleep(std::time::Duration::from_millis(20)),
                                Next::Finished => return,
                            }
                        }
                    });
                }
            }
            Capture::Workers(pool) => match pool.start(&rom, &original) {
                Some(generation) => loader.pool = Some((pool, generation)),
                None => loader.local = RefCell::new(Capturer::new(rom, original, new_baselines())),
            },
            Capture::Slices => loader.local = RefCell::new(Capturer::new(rom, original, new_baselines())),
        }
        loader
    }

    /// Each frame: on the capture workers, take their results and give the idle ones the next
    /// jobs; on the window's thread, run one job.
    pub fn pump(&self) {
        if let Some((pool, _)) = self.pool {
            self.pump_workers(pool);
            return;
        }
        let mut local = self.local.borrow_mut();
        if let Some(c) = local.as_ref()
            && let Next::Finished = self.s.step(c)
        {
            *local = None;
        }
    }

    fn pump_workers(&self, pool: &dyn CapturePool) {
        while let Some(line) = pool.take() {
            self.sent.set(self.sent.get().saturating_sub(1));
            if let Some((job, done)) = parse_done(&line) {
                self.s.deliver(job, done);
            }
        }
        let Some(mut idle) = pool.idle() else { return };
        while idle > 0 {
            let Next::Run(job) = self.s.next() else { return };
            pool.send(&job_line(&job));
            self.sent.set(self.sent.get() + 1);
            idle -= 1;
        }
    }

    /// How many objects' tiles are still to be captured.
    pub fn tiles_left(&self) -> usize {
        self.s.tiles_left.lock().map(|n| *n).unwrap_or(0)
    }

    /// The object tiles captured since the last call.
    pub fn take_tiles(&self) -> Vec<(usize, u8, Vec<u8>)> {
        self.tiles.try_iter().collect()
    }

    /// Capture these next (the shown level's), in this order.
    pub fn prefer(&self, keys: &[(usize, u8)]) {
        if let Ok(mut u) = self.s.urgent.lock() {
            u.extend(keys.iter().copied());
        }
        if let Ok(mut q) = self.s.want.lock() {
            to_front(&mut q, keys);
        }
    }

    /// Capture these tiles before the other tiles (the shown level's).
    pub fn prefer_tiles(&self, keys: &[(usize, u8)]) {
        if let Ok(mut q) = self.s.tile_jobs.lock() {
            to_front(&mut q, keys);
        }
    }

    /// The pictures captured since the last call.
    pub fn take(&self) -> Vec<(usize, u8, Option<Look>)> {
        self.got.try_iter().collect()
    }

    /// Wait for the next picture (the check pictures need the shown level's at once; not on
    /// capture workers, which answer between frames).
    pub fn wait(&self) -> Option<(usize, u8, Option<Look>)> {
        if self.local.borrow().is_none() {
            return self.got.recv_timeout(std::time::Duration::from_secs(5)).ok();
        }
        loop {
            if let Ok(got) = self.got.try_recv() {
                return Some(got);
            }
            let local = self.local.borrow();
            match local.as_ref().map(|c| self.s.step(c)) {
                Some(Next::Run(_)) => {}
                _ => return None,
            }
        }
    }

    /// Nothing is left to capture.
    pub fn done(&self) -> bool {
        self.s.want.lock().map(|q| q.is_empty()).unwrap_or(true) && self.tiles_left() == 0 && self.sent.get() == 0
    }

    /// Pictures left and in all, tiles left and in all.
    pub fn progress(&self) -> ((usize, usize), (usize, usize)) {
        let pictures = self.s.want.lock().map(|q| q.len()).unwrap_or(0);
        ((pictures, self.totals.0), (self.tiles_left(), self.totals.1))
    }
}

impl Drop for LookLoader {
    fn drop(&mut self) {
        if let Some((pool, generation)) = self.pool {
            pool.stop(generation);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_and_threads_capture_the_same_pictures() {
        let Some(path) = host::test_cartridge() else {
            eprintln!("no cartridge found: test skipped");
            return;
        };
        let rom = std::fs::read(path).unwrap();
        // everything but level 1's own types is known: only those are captured
        let own: HashSet<(usize, u8)> = objects::read(&rom, 1).0.into_iter().map(|o| (1, o.kind)).collect();
        let known: HashSet<(usize, u8)> = (0..LEVELS).flat_map(|l| (0..128u8).map(move |k| (l, k))).filter(|key| !own.contains(key)).collect();
        let all_tiles: HashSet<(usize, u8)> = (0..LEVELS).flat_map(|l| (0..128u8).map(move |k| (l, k))).collect();
        let captured = |capture: Capture| {
            let loader = LookLoader::start_with(capture, rom.clone(), rom.clone(), &known, &all_tiles);
            let mut got = Vec::new();
            while got.len() < own.len() {
                let Some(look) = loader.wait() else { break };
                got.push(look);
            }
            got.sort_by_key(|g| (g.0, g.1));
            got.into_iter().map(|(l, k, look)| (l, k, look.map(|p| (p.w, p.h, p.px)))).collect::<Vec<_>>()
        };
        let slices = captured(Capture::Slices);
        assert_eq!(slices.len(), own.len());
        assert_eq!(slices, captured(Capture::Threads));
    }
}
