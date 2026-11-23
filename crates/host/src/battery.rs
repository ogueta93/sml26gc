//! Loading a cartridge file with its battery save, and writing the save back.
use gb_core::Gb;
use std::path::Path;

/// Power on a Game Boy with this cartridge file; with `save`, its battery RAM comes from that
/// save file (if there is one).
pub fn load_gb(rom_path: &Path, sample_rate: u32, save: Option<&Path>) -> Result<Gb, String> {
    let rom = crate::storage::read(rom_path).map_err(|e| format!("could not read {}: {e}", rom_path.display()))?;
    let mut gb = Gb::new(rom, sample_rate)?;
    if let (Some(save), true) = (save, gb.cart().has_battery()) {
        if let Ok(data) = crate::storage::read(save) {
            let ram = gb.cart_mut().ram_mut();
            let n = data.len().min(ram.len());
            ram[..n].copy_from_slice(&data[..n]);
        }
    }
    Ok(gb)
}

/// Write the battery RAM to the save file `save` if the game changed it since the last write.
/// Returns whether it wrote.
pub fn write_battery_save(gb: &mut Gb, save: &Path) -> std::io::Result<bool> {
    if !gb.cart().has_battery() || !gb.cart().needs_saving() {
        return Ok(false);
    }
    if let Some(d) = save.parent() {
        crate::storage::create_dir_all(d)?;
    }
    crate::storage::write(save, gb.cart().ram())?;
    gb.cart_mut().mark_saved();
    Ok(true)
}
