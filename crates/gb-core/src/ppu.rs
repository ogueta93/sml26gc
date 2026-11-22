//! Picture processing unit: LCD timing, STAT/VBlank interrupts and a scanline renderer.

pub const W: usize = 160;
pub const H: usize = 144;

/// One picture: 160 x 144 shades, 0 (lightest) ..= 3 (darkest), row by row.
pub type Screen = [u8; W * H];

#[derive(Clone)]
pub struct Ppu {
    pub(crate) vram: [u8; 0x2000],
    pub(crate) oam: [u8; 0xA0],
    pub(crate) lcdc: u8,
    stat: u8,
    scy: u8,
    scx: u8,
    pub(crate) ly: u8,
    lyc: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,
    dot: u32,
    mode: u8,
    win_line: u8,
    stat_line: bool,
    /// Shades 0 (lightest) ..= 3 (darkest), one byte per pixel.
    pub(crate) fb: Screen,
    pub(crate) frame_ready: bool,
    /// Nobody looks at the screen: lines are not drawn (`GameBoy::set_unseen`).
    pub(crate) unseen: bool,
}

impl Ppu {
    pub(crate) fn new() -> Ppu {
        Ppu {
            vram: [0; 0x2000],
            oam: [0; 0xA0],
            lcdc: 0x91,
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0xFC,
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0,
            wx: 0,
            dot: 0,
            mode: 2,
            win_line: 0,
            stat_line: false,
            fb: [0; W * H],
            frame_ready: false,
            unseen: false,
        }
    }

    /// Video memory: tile graphics at $8000-$97FF, tile maps at $9800-$9FFF.
    pub fn vram(&self) -> &[u8; 0x2000] {
        &self.vram
    }

    /// Sprite table: 40 entries of (y, x, tile, flags).
    pub fn oam(&self) -> &[u8; 0xA0] {
        &self.oam
    }

    pub fn lcdc(&self) -> u8 {
        self.lcdc
    }

    pub fn bgp(&self) -> u8 {
        self.bgp
    }

    pub fn obp(&self, which: u8) -> u8 {
        if which == 0 {
            self.obp0
        } else {
            self.obp1
        }
    }

    pub fn scx(&self) -> u8 {
        self.scx
    }

    pub fn lcd_on(&self) -> bool {
        self.lcdc & 0x80 != 0
    }

    pub(crate) fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF40 => self.lcdc,
            0xFF41 => {
                let mode = if self.lcd_on() { self.mode } else { 0 };
                0x80 | (self.stat & 0x78) | if self.ly == self.lyc { 4 } else { 0 } | mode
            }
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFF47 => self.bgp,
            0xFF48 => self.obp0,
            0xFF49 => self.obp1,
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            _ => 0xFF,
        }
    }

    pub(crate) fn write(&mut self, addr: u16, v: u8) {
        match addr {
            0xFF40 => {
                let was_on = self.lcd_on();
                self.lcdc = v;
                if was_on && !self.lcd_on() {
                    self.ly = 0;
                    self.dot = 0;
                    self.mode = 0;
                    self.stat_line = false;
                } else if !was_on && self.lcd_on() {
                    self.ly = 0;
                    self.dot = 0;
                    self.mode = 2;
                    self.win_line = 0;
                }
            }
            0xFF41 => self.stat = v & 0x78,
            0xFF42 => self.scy = v,
            0xFF43 => self.scx = v,
            0xFF45 => self.lyc = v,
            0xFF47 => self.bgp = v,
            0xFF48 => self.obp0 = v,
            0xFF49 => self.obp1 = v,
            0xFF4A => self.wy = v,
            0xFF4B => self.wx = v,
            _ => {}
        }
    }

    pub(crate) fn tick(&mut self, cycles: u32, iflag: &mut u8) {
        if !self.lcd_on() {
            return;
        }
        let mut left = cycles;
        while left > 0 {
            let step = left.min(4);
            left -= step;
            self.dot += step;
            if self.dot >= 456 {
                self.dot -= 456;
                self.ly += 1;
                if self.ly == 154 {
                    self.ly = 0;
                    self.win_line = 0;
                }
                if self.ly == 144 {
                    *iflag |= 0x01;
                    self.frame_ready = true;
                }
            }
            let mode = if self.ly >= 144 {
                1
            } else if self.dot < 80 {
                2
            } else if self.dot < 252 {
                3
            } else {
                0
            };
            if mode != self.mode {
                if mode == 0 && !self.unseen {
                    self.render_line();
                }
                self.mode = mode;
            }
            let s = self.stat;
            let line = (self.ly == self.lyc && s & 0x40 != 0)
                || (self.mode == 0 && s & 0x08 != 0)
                || (self.mode == 2 && s & 0x20 != 0)
                || (self.mode == 1 && s & 0x10 != 0);
            if line && !self.stat_line {
                *iflag |= 0x02;
            }
            self.stat_line = line;
        }
    }

    fn tile_pixel(&self, addr: usize, bit: u8) -> u8 {
        let lo = self.vram[addr];
        let hi = self.vram[addr + 1];
        (((hi >> bit) & 1) << 1) | ((lo >> bit) & 1)
    }

    fn render_line(&mut self) {
        let ly = self.ly as usize;
        if ly >= H {
            return;
        }
        let mut line = [0u8; W];
        let mut bgidx = [0u8; W];
        let lcdc = self.lcdc;
        if lcdc & 0x01 != 0 {
            let win_on = lcdc & 0x20 != 0 && self.wy <= self.ly && self.wx <= 166;
            let wx0 = self.wx as i32 - 7;
            let mut win_used = false;
            for x in 0..W {
                let (map, px, py) = if win_on && x as i32 >= wx0 {
                    win_used = true;
                    (
                        if lcdc & 0x40 != 0 { 0x1C00 } else { 0x1800 },
                        (x as i32 - wx0) as u8,
                        self.win_line,
                    )
                } else {
                    (
                        if lcdc & 0x08 != 0 { 0x1C00 } else { 0x1800 },
                        (x as u8).wrapping_add(self.scx),
                        self.ly.wrapping_add(self.scy),
                    )
                };
                let tile = self.vram[map + (py as usize / 8) * 32 + px as usize / 8];
                let base = if lcdc & 0x10 != 0 {
                    tile as usize * 16
                } else {
                    (0x1000 + (tile as i8 as i32) * 16) as usize
                };
                let c = self.tile_pixel(base + (py as usize % 8) * 2, 7 - (px % 8));
                bgidx[x] = c;
                line[x] = (self.bgp >> (c * 2)) & 3;
            }
            if win_used {
                self.win_line = self.win_line.wrapping_add(1);
            }
        }
        if lcdc & 0x02 != 0 {
            let h: i32 = if lcdc & 0x04 != 0 { 16 } else { 8 };
            let mut sel: Vec<usize> = Vec::with_capacity(10);
            for i in 0..40 {
                let y = self.oam[i * 4] as i32 - 16;
                if (ly as i32) >= y && (ly as i32) < y + h {
                    sel.push(i);
                    if sel.len() == 10 {
                        break;
                    }
                }
            }
            sel.sort_by_key(|&i| (self.oam[i * 4 + 1], i));
            for &i in sel.iter().rev() {
                let y = self.oam[i * 4] as i32 - 16;
                let x = self.oam[i * 4 + 1] as i32 - 8;
                let mut tile = self.oam[i * 4 + 2] as usize;
                let attr = self.oam[i * 4 + 3];
                let mut row = ly as i32 - y;
                if attr & 0x40 != 0 {
                    row = h - 1 - row;
                }
                if h == 16 {
                    tile &= 0xFE;
                }
                let addr = tile * 16 + row as usize * 2;
                let pal = if attr & 0x10 != 0 { self.obp1 } else { self.obp0 };
                for px in 0..8i32 {
                    let sx = x + px;
                    if !(0..W as i32).contains(&sx) {
                        continue;
                    }
                    let bit = if attr & 0x20 != 0 { px } else { 7 - px } as u8;
                    let c = self.tile_pixel(addr, bit);
                    if c == 0 {
                        continue;
                    }
                    if attr & 0x80 != 0 && bgidx[sx as usize] != 0 {
                        continue;
                    }
                    line[sx as usize] = (pal >> (c * 2)) & 3;
                }
            }
        }
        self.fb[ly * W..ly * W + W].copy_from_slice(&line);
    }
}
