//! The programs' icons (`assets/icons/*.png`) scaled to the sizes a package needs.
use std::path::Path;

/// `png_file`, any size and colour type, as a `size`-pixel square RGBA PNG (averaging the pixels).
pub fn scaled(png_file: &Path, size: u32) -> Result<Vec<u8>, String> {
    let what = |e: &dyn std::fmt::Display| format!("{}: {e}", png_file.display());
    let mut decoder = png::Decoder::new(std::fs::File::open(png_file).map_err(|e| what(&e))?);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| what(&e))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| what(&e))?;
    let channels = info.color_type.samples();
    let rgba = |x: u32, y: u32| -> [f32; 4] {
        let p = &buf[(y as usize * info.width as usize + x as usize) * channels..][..channels];
        match channels {
            4 => [p[0], p[1], p[2], p[3]].map(f32::from),
            3 => [p[0], p[1], p[2], 255].map(f32::from),
            2 => [p[0], p[0], p[0], p[1]].map(f32::from),
            _ => [p[0], p[0], p[0], 255].map(f32::from),
        }
    };
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let (x0, x1) = (x * info.width / size, ((x + 1) * info.width / size).max(x * info.width / size + 1));
            let (y0, y1) = (y * info.height / size, ((y + 1) * info.height / size).max(y * info.height / size + 1));
            let mut sum = [0.0; 4];
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let p = rgba(sx, sy);
                    // colours weighted by their opacity, so transparent pixels add no colour
                    for c in 0..3 {
                        sum[c] += p[c] * p[3];
                    }
                    sum[3] += p[3];
                }
            }
            let n = ((x1 - x0) * (y1 - y0)) as f32;
            let alpha = sum[3] / n;
            pixels.extend((0..3).map(|c| if sum[3] > 0.0 { (sum[c] / sum[3]).round() as u8 } else { 0 }));
            pixels.push(alpha.round() as u8);
        }
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().and_then(|mut w| w.write_image_data(&pixels)).map_err(|e| what(&e))?;
    Ok(out)
}
