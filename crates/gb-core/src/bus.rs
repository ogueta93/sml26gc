//! Memory map, timer, joypad and interrupt flags.
use crate::apu::Apu;
use crate::cart::Cart;
use crate::input::Buttons;
use crate::observer::{BusObserver, NoObserver};
use crate::ppu::Ppu;

#[derive(Clone)]
pub(crate) struct Bus<O: BusObserver = NoObserver> {
    pub(crate) cart: Cart,
    pub(crate) ppu: Ppu,
    pub(crate) apu: Apu,
    pub(crate) wram: [u8; 0x2000],
    hram: [u8; 0x7F],
    pub(crate) ie: u8,
    pub(crate) iflag: u8,
    // timer
    pub(crate) div: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    // joypad
    buttons: Buttons,
    joy_sel: u8,
    sb: u8,
    sc: u8,
    /// Address of the instruction being executed (only kept up to date for an observer).
    pub(crate) cur_pc: u16,
    pub(crate) observer: O,
}

impl<O: BusObserver> Bus<O> {
    pub(crate) fn new(cart: Cart, sample_rate: u32, observer: O) -> Bus<O> {
        Bus {
            cart,
            ppu: Ppu::new(),
            apu: Apu::new(sample_rate),
            wram: [0; 0x2000],
            hram: [0; 0x7F],
            ie: 0,
            iflag: 0xE1,
            div: 0xABCC,
            tima: 0,
            tma: 0,
            tac: 0xF8,
            buttons: Buttons::NONE,
            joy_sel: 0x30,
            sb: 0,
            sc: 0,
            cur_pc: 0,
            observer,
        }
    }

    pub(crate) fn set_buttons(&mut self, b: Buttons) {
        if !b.newly_pressed(self.buttons).is_empty() {
            self.iflag |= 0x10;
        }
        self.buttons = b;
    }

    pub(crate) fn read(&mut self, addr: u16) -> u8 {
        if O::ENABLED {
            let bank = self.cart.upper_bank();
            self.observer.on_read(self.cur_pc, bank, addr);
        }
        match addr {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cart.read(addr),
            0x8000..=0x9FFF => self.ppu.vram[(addr - 0x8000) as usize],
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize],
            0xFE00..=0xFE9F => self.ppu.oam[(addr - 0xFE00) as usize],
            0xFEA0..=0xFEFF => 0xFF,
            0xFF00 => {
                let pressed = self.buttons.bits();
                let mut r = 0xC0 | (self.joy_sel & 0x30) | 0x0F;
                if self.joy_sel & 0x10 == 0 {
                    r &= !(pressed & 0x0F);
                }
                if self.joy_sel & 0x20 == 0 {
                    r &= !((pressed >> 4) & 0x0F);
                }
                r
            }
            0xFF01 => self.sb,
            0xFF02 => self.sc | 0x7E,
            0xFF04 => (self.div >> 8) as u8,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac | 0xF8,
            0xFF0F => self.iflag | 0xE0,
            0xFF10..=0xFF3F => self.apu.read(addr),
            0xFF40..=0xFF4B => self.ppu.read(addr),
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            0xFFFF => self.ie,
            _ => 0xFF,
        }
    }

    pub(crate) fn write(&mut self, addr: u16, v: u8) {
        if O::ENABLED {
            // the bank mapped at 4000-7FFF when the write happened
            let bank = self.cart.upper_bank();
            self.observer.on_write(self.cur_pc, bank, addr, v);
        }
        match addr {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cart.write(addr, v),
            0x8000..=0x9FFF => self.ppu.vram[(addr - 0x8000) as usize] = v,
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = v,
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize] = v,
            0xFE00..=0xFE9F => self.ppu.oam[(addr - 0xFE00) as usize] = v,
            0xFF00 => self.joy_sel = v & 0x30,
            0xFF01 => self.sb = v,
            0xFF02 => {
                // No link cable: a started transfer finishes at once and reads 0xFF.
                self.sc = v;
                if v & 0x81 == 0x81 {
                    self.sb = 0xFF;
                    self.sc &= 0x7F;
                    self.iflag |= 0x08;
                }
            }
            0xFF04 => self.div = 0,
            0xFF05 => self.tima = v,
            0xFF06 => self.tma = v,
            0xFF07 => self.tac = v & 7,
            0xFF0F => self.iflag = v & 0x1F,
            0xFF10..=0xFF3F => self.apu.write(addr, v),
            0xFF46 => {
                let src = (v as u16) << 8;
                for i in 0..0xA0u16 {
                    let b = self.read(src + i);
                    self.ppu.oam[i as usize] = b;
                }
            }
            0xFF40..=0xFF4B => self.ppu.write(addr, v),
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = v,
            0xFFFF => self.ie = v,
            _ => {}
        }
    }

    pub(crate) fn tick(&mut self, cycles: u32) {
        // Timer: TIMA counts on the falling edge of the selected DIV bit.
        let bit = match self.tac & 3 {
            0 => 9,
            1 => 3,
            2 => 5,
            _ => 7,
        };
        let mut c = cycles;
        while c > 0 {
            let old = self.div;
            self.div = self.div.wrapping_add(4);
            c = c.saturating_sub(4);
            if self.tac & 4 != 0 && (old >> bit) & 1 == 1 && (self.div >> bit) & 1 == 0 {
                let (t, over) = self.tima.overflowing_add(1);
                if over {
                    self.tima = self.tma;
                    self.iflag |= 0x04;
                } else {
                    self.tima = t;
                }
            }
        }
        self.ppu.tick(cycles, &mut self.iflag);
        self.apu.tick(cycles);
    }
}
