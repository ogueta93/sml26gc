//! Audio processing unit: two pulse channels (one with sweep), a wave channel and noise.

const CPU_HZ: f64 = 4_194_304.0;
const DUTY: [u8; 4] = [0b0000_0001, 0b1000_0001, 0b1000_0111, 0b0111_1110];
const NOISE_DIV: [i32; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

#[derive(Default, Clone)]
struct Env {
    init: u8,
    up: bool,
    period: u8,
    timer: u8,
    vol: u8,
}

impl Env {
    fn set(&mut self, v: u8) {
        self.init = v >> 4;
        self.up = v & 8 != 0;
        self.period = v & 7;
    }
    fn trigger(&mut self) {
        self.vol = self.init;
        self.timer = self.period;
    }
    fn clock(&mut self) {
        if self.period == 0 {
            return;
        }
        if self.timer > 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.timer = self.period;
            if self.up && self.vol < 15 {
                self.vol += 1;
            } else if !self.up && self.vol > 0 {
                self.vol -= 1;
            }
        }
    }
}

#[derive(Default, Clone)]
struct Pulse {
    on: bool,
    dac: bool,
    length: u16,
    len_en: bool,
    duty: u8,
    pos: u8,
    freq: u16,
    timer: i32,
    env: Env,
    sw_period: u8,
    sw_neg: bool,
    sw_shift: u8,
    sw_timer: u8,
    sw_en: bool,
    shadow: u16,
}

impl Pulse {
    fn sweep_calc(&mut self) -> u16 {
        let d = self.shadow >> self.sw_shift;
        let n = if self.sw_neg { self.shadow.wrapping_sub(d) } else { self.shadow + d };
        if n > 2047 {
            self.on = false;
        }
        n
    }
    fn trigger(&mut self) {
        self.on = self.dac;
        if self.length == 0 {
            self.length = 64;
        }
        self.timer = (2048 - self.freq as i32) * 4;
        self.env.trigger();
        self.shadow = self.freq;
        self.sw_timer = if self.sw_period == 0 { 8 } else { self.sw_period };
        self.sw_en = self.sw_period != 0 || self.sw_shift != 0;
        if self.sw_shift != 0 {
            self.sweep_calc();
        }
    }
    fn clock_sweep(&mut self) {
        if self.sw_timer > 0 {
            self.sw_timer -= 1;
        }
        if self.sw_timer == 0 {
            self.sw_timer = if self.sw_period == 0 { 8 } else { self.sw_period };
            if self.sw_en && self.sw_period != 0 {
                let n = self.sweep_calc();
                if n <= 2047 && self.sw_shift != 0 {
                    self.shadow = n;
                    self.freq = n;
                    self.sweep_calc();
                }
            }
        }
    }
    fn clock_len(&mut self) {
        if self.len_en && self.length > 0 {
            self.length -= 1;
            if self.length == 0 {
                self.on = false;
            }
        }
    }
    fn step(&mut self, c: i32) {
        self.timer -= c;
        while self.timer <= 0 {
            self.timer += (2048 - self.freq as i32) * 4;
            self.pos = (self.pos + 1) & 7;
        }
    }
    fn out(&self) -> u8 {
        if self.on && (DUTY[self.duty as usize] >> self.pos) & 1 != 0 {
            self.env.vol
        } else {
            0
        }
    }
}

#[derive(Default, Clone)]
struct Wave {
    on: bool,
    dac: bool,
    length: u16,
    len_en: bool,
    vol: u8,
    freq: u16,
    timer: i32,
    pos: u8,
    ram: [u8; 16],
}

impl Wave {
    fn trigger(&mut self) {
        self.on = self.dac;
        if self.length == 0 {
            self.length = 256;
        }
        self.timer = (2048 - self.freq as i32) * 2;
        self.pos = 0;
    }
    fn clock_len(&mut self) {
        if self.len_en && self.length > 0 {
            self.length -= 1;
            if self.length == 0 {
                self.on = false;
            }
        }
    }
    fn step(&mut self, c: i32) {
        self.timer -= c;
        while self.timer <= 0 {
            self.timer += (2048 - self.freq as i32) * 2;
            self.pos = (self.pos + 1) & 31;
        }
    }
    fn out(&self) -> u8 {
        if !self.on || self.vol == 0 {
            return 0;
        }
        let b = self.ram[(self.pos / 2) as usize];
        let s = if self.pos & 1 == 0 { b >> 4 } else { b & 0xF };
        s >> (self.vol - 1)
    }
}

#[derive(Default, Clone)]
struct Noise {
    on: bool,
    dac: bool,
    length: u16,
    len_en: bool,
    env: Env,
    shift: u8,
    narrow: bool,
    div: u8,
    timer: i32,
    lfsr: u16,
}

impl Noise {
    fn period(&self) -> i32 {
        NOISE_DIV[self.div as usize] << self.shift
    }
    fn trigger(&mut self) {
        self.on = self.dac;
        if self.length == 0 {
            self.length = 64;
        }
        self.timer = self.period();
        self.env.trigger();
        self.lfsr = 0x7FFF;
    }
    fn clock_len(&mut self) {
        if self.len_en && self.length > 0 {
            self.length -= 1;
            if self.length == 0 {
                self.on = false;
            }
        }
    }
    fn step(&mut self, c: i32) {
        self.timer -= c;
        while self.timer <= 0 {
            self.timer += self.period();
            let x = (self.lfsr & 1) ^ ((self.lfsr >> 1) & 1);
            self.lfsr = (self.lfsr >> 1) | (x << 14);
            if self.narrow {
                self.lfsr = (self.lfsr & !(1 << 6)) | (x << 6);
            }
        }
    }
    fn out(&self) -> u8 {
        if self.on && self.lfsr & 1 == 0 {
            self.env.vol
        } else {
            0
        }
    }
}

#[derive(Clone)]
pub(crate) struct Apu {
    regs: [u8; 0x30],
    power: bool,
    fs_counter: u32,
    fs_step: u8,
    ch1: Pulse,
    ch2: Pulse,
    ch3: Wave,
    ch4: Noise,
    cycles_per_sample: f64,
    sample_acc: f64,
    sum_l: f32,
    sum_r: f32,
    n: u32,
    cap_l: f32,
    cap_r: f32,
    hp: f32,
    /// Interleaved stereo samples waiting to be played.
    pub(crate) out: Vec<f32>,
    /// Nobody listens: the channels run, but nothing is mixed (`GameBoy::set_unseen`).
    pub(crate) unheard: bool,
}

const READ_MASK: [u8; 0x17] = [
    0x80, 0x3F, 0x00, 0xFF, 0xBF, 0xFF, 0x3F, 0x00, 0xFF, 0xBF, 0x7F, 0xFF, 0x9F, 0xFF, 0xBF, 0xFF,
    0xFF, 0x00, 0x00, 0xBF, 0x00, 0x00, 0x70,
];

impl Apu {
    pub fn new(sample_rate: u32) -> Apu {
        let cps = CPU_HZ / sample_rate as f64;
        Apu {
            regs: [0; 0x30],
            power: true,
            fs_counter: 0,
            fs_step: 0,
            ch1: Pulse::default(),
            ch2: Pulse::default(),
            ch3: Wave::default(),
            ch4: Noise { lfsr: 0x7FFF, ..Default::default() },
            cycles_per_sample: cps,
            sample_acc: 0.0,
            sum_l: 0.0,
            sum_r: 0.0,
            n: 0,
            cap_l: 0.0,
            cap_r: 0.0,
            hp: 0.999958f32.powf(cps as f32),
            out: Vec::with_capacity(4096),
            unheard: false,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        (CPU_HZ / self.cycles_per_sample).round() as u32
    }

    pub fn read(&self, addr: u16) -> u8 {
        let i = (addr - 0xFF10) as usize;
        match addr {
            0xFF26 => {
                (if self.power { 0x80 } else { 0 })
                    | 0x70
                    | (self.ch1.on as u8)
                    | (self.ch2.on as u8) << 1
                    | (self.ch3.on as u8) << 2
                    | (self.ch4.on as u8) << 3
            }
            0xFF30..=0xFF3F => self.ch3.ram[i - 0x20],
            0xFF10..=0xFF25 => self.regs[i] | READ_MASK[i],
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, v: u8) {
        if (0xFF30..=0xFF3F).contains(&addr) {
            self.ch3.ram[(addr - 0xFF30) as usize] = v;
            return;
        }
        if addr == 0xFF26 {
            let on = v & 0x80 != 0;
            if self.power && !on {
                for a in 0xFF10..=0xFF25u16 {
                    self.write(a, 0);
                }
                self.ch1.on = false;
                self.ch2.on = false;
                self.ch3.on = false;
                self.ch4.on = false;
            }
            if !self.power && on {
                self.fs_step = 0;
            }
            self.power = on;
            return;
        }
        if !self.power {
            return;
        }
        self.regs[(addr - 0xFF10) as usize] = v;
        match addr {
            0xFF10 => {
                self.ch1.sw_period = (v >> 4) & 7;
                self.ch1.sw_neg = v & 8 != 0;
                self.ch1.sw_shift = v & 7;
            }
            0xFF11 | 0xFF16 => {
                let ch = if addr == 0xFF11 { &mut self.ch1 } else { &mut self.ch2 };
                ch.duty = v >> 6;
                ch.length = 64 - (v & 63) as u16;
            }
            0xFF12 | 0xFF17 => {
                let ch = if addr == 0xFF12 { &mut self.ch1 } else { &mut self.ch2 };
                ch.env.set(v);
                ch.dac = v & 0xF8 != 0;
                if !ch.dac {
                    ch.on = false;
                }
            }
            0xFF13 | 0xFF18 => {
                let ch = if addr == 0xFF13 { &mut self.ch1 } else { &mut self.ch2 };
                ch.freq = (ch.freq & 0x700) | v as u16;
            }
            0xFF14 | 0xFF19 => {
                let ch = if addr == 0xFF14 { &mut self.ch1 } else { &mut self.ch2 };
                ch.freq = (ch.freq & 0xFF) | (((v & 7) as u16) << 8);
                ch.len_en = v & 0x40 != 0;
                if v & 0x80 != 0 {
                    ch.trigger();
                }
            }
            0xFF1A => {
                self.ch3.dac = v & 0x80 != 0;
                if !self.ch3.dac {
                    self.ch3.on = false;
                }
            }
            0xFF1B => self.ch3.length = 256 - v as u16,
            0xFF1C => self.ch3.vol = (v >> 5) & 3,
            0xFF1D => self.ch3.freq = (self.ch3.freq & 0x700) | v as u16,
            0xFF1E => {
                self.ch3.freq = (self.ch3.freq & 0xFF) | (((v & 7) as u16) << 8);
                self.ch3.len_en = v & 0x40 != 0;
                if v & 0x80 != 0 {
                    self.ch3.trigger();
                }
            }
            0xFF20 => self.ch4.length = 64 - (v & 63) as u16,
            0xFF21 => {
                self.ch4.env.set(v);
                self.ch4.dac = v & 0xF8 != 0;
                if !self.ch4.dac {
                    self.ch4.on = false;
                }
            }
            0xFF22 => {
                self.ch4.shift = v >> 4;
                self.ch4.narrow = v & 8 != 0;
                self.ch4.div = v & 7;
            }
            0xFF23 => {
                self.ch4.len_en = v & 0x40 != 0;
                if v & 0x80 != 0 {
                    self.ch4.trigger();
                }
            }
            _ => {}
        }
    }

    pub fn tick(&mut self, cycles: u32) {
        let mut left = cycles as i32;
        while left > 0 {
            let c = left.min(4);
            left -= c;
            if self.power {
                self.fs_counter += c as u32;
                if self.fs_counter >= 8192 {
                    self.fs_counter -= 8192;
                    match self.fs_step {
                        0 | 4 => self.clock_len(),
                        2 | 6 => {
                            self.clock_len();
                            self.ch1.clock_sweep();
                        }
                        7 => {
                            self.ch1.env.clock();
                            self.ch2.env.clock();
                            self.ch4.env.clock();
                        }
                        _ => {}
                    }
                    self.fs_step = (self.fs_step + 1) & 7;
                }
                self.ch1.step(c);
                self.ch2.step(c);
                self.ch3.step(c);
                self.ch4.step(c);
            }
            if self.unheard {
                continue;
            }
            let (l, r) = if self.power {
                let o = [self.ch1.out(), self.ch2.out(), self.ch3.out(), self.ch4.out()];
                let pan = self.regs[0x15];
                let mut l = 0u32;
                let mut r = 0u32;
                for (i, &s) in o.iter().enumerate() {
                    if pan & (0x10 << i) != 0 {
                        l += s as u32;
                    }
                    if pan & (1 << i) != 0 {
                        r += s as u32;
                    }
                }
                let vol = self.regs[0x14];
                let vl = ((vol >> 4) & 7) as f32 + 1.0;
                let vr = (vol & 7) as f32 + 1.0;
                (l as f32 / 60.0 * vl / 8.0, r as f32 / 60.0 * vr / 8.0)
            } else {
                (0.0, 0.0)
            };
            self.sum_l += l;
            self.sum_r += r;
            self.n += 1;
            self.sample_acc += c as f64;
            if self.sample_acc >= self.cycles_per_sample {
                self.sample_acc -= self.cycles_per_sample;
                let n = self.n.max(1) as f32;
                let (il, ir) = (self.sum_l / n, self.sum_r / n);
                self.sum_l = 0.0;
                self.sum_r = 0.0;
                self.n = 0;
                let ol = il - self.cap_l;
                self.cap_l = il - ol * self.hp;
                let or = ir - self.cap_r;
                self.cap_r = ir - or * self.hp;
                if self.out.len() < 1 << 16 {
                    self.out.push(ol * 0.6);
                    self.out.push(or * 0.6);
                }
            }
        }
    }

    fn clock_len(&mut self) {
        self.ch1.clock_len();
        self.ch2.clock_len();
        self.ch3.clock_len();
        self.ch4.clock_len();
    }
}
