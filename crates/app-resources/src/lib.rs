//! Build-time only, called from the build scripts of the game and the editor: the program's icon,
//! made from its square PNG in `assets/icons/`, and its version details.
//!
//! - `OUT_DIR/window_icon.rs`: an `Option<miniquad::conf::Icon>` expression (the 16, 32 and 64
//!   pixel images for the window and the taskbar), `None` without the PNG;
//! - on Windows targets, an `.ico` (16 to 256 pixels) and a version resource (product, program,
//!   version, copyright, licence and not-affiliated line) linked into the executable.
//!
//! A missing PNG is a warning, not an error: the program builds with the system's icon.
use std::path::{Path, PathBuf};

/// The copyright line of the executables' version details (and of the macOS bundles, `xtask`).
pub const COPYRIGHT: &str = "Copyright 2026 ogueta93 (https://github.com/ogueta93) and the SML2 6GC authors. GPL-3.0-or-later. No game included; not affiliated with or endorsed by Nintendo.";

/// The sizes the `.ico` holds.
const ICO_SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];

/// An RGBA image, row by row.
struct Image {
    size: u32,
    rgba: Vec<u8>,
}

/// Embed the icon `assets/icons/<program>.png` (from the workspace root) and the version details
/// of the program described as `description`.
pub fn embed(program: &str, description: &str) {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let png = manifest.parent().unwrap().join("assets").join("icons").join(format!("{program}.png"));
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed={}", png.display());
    println!("cargo:rerun-if-env-changed=PRODUCT_NAME");
    let image = match read_square(&png) {
        Ok(i) => Some(i),
        Err(e) => {
            println!("cargo:warning={program}: no icon ({e})");
            None
        }
    };
    write_window_icon(&out, image.as_ref());
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows_resource(&out, program, description, image.as_ref());
    }
}

/// The PNG at `path` as RGBA (palette, grey and RGB expanded), which must be square.
fn read_square(path: &Path) -> Result<Image, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut decoder = png::Decoder::new(file);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| format!("{}: {e}", path.display()))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("{}: {e}", path.display()))?;
    if info.width != info.height {
        return Err(format!("{} is {}x{}, not square", path.display(), info.width, info.height));
    }
    let px = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => px.chunks(2).flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err(format!("{}: palette not expanded", path.display())),
    };
    Ok(Image { size: info.width, rgba })
}

/// `image` scaled to `size` by averaging the pixels each new one covers (colours weighted by
/// their alpha, so transparent pixels do not darken the edges).
fn scaled(image: &Image, size: u32) -> Image {
    let k = image.size as f64 / size as f64;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let (x0, x1, y0, y1) = (x as f64 * k, (x + 1) as f64 * k, y as f64 * k, (y + 1) as f64 * k);
            let mut sum = [0f64; 4];
            let mut area = 0f64;
            for sy in y0.floor() as u32..(y1.ceil() as u32).min(image.size) {
                let wy = (y1.min(sy as f64 + 1.0) - y0.max(sy as f64)).max(0.0);
                for sx in x0.floor() as u32..(x1.ceil() as u32).min(image.size) {
                    let w = wy * (x1.min(sx as f64 + 1.0) - x0.max(sx as f64)).max(0.0);
                    let p = &image.rgba[((sy * image.size + sx) * 4) as usize..][..4];
                    let a = p[3] as f64 / 255.0;
                    for c in 0..3 {
                        sum[c] += p[c] as f64 * a * w;
                    }
                    sum[3] += a * w;
                    area += w;
                }
            }
            for c in 0..3 {
                rgba.push(if sum[3] > 0.0 { (sum[c] / sum[3]).round().clamp(0.0, 255.0) as u8 } else { 0 });
            }
            rgba.push(if area > 0.0 { (sum[3] / area * 255.0).round() as u8 } else { 0 });
        }
    }
    Image { size, rgba }
}

fn png_bytes(image: &Image) -> Vec<u8> {
    let mut out = Vec::new();
    let mut e = png::Encoder::new(&mut out, image.size, image.size);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(&image.rgba).unwrap();
    out
}

fn write_window_icon(out: &Path, image: Option<&Image>) {
    let code = match image {
        Some(i) => {
            for s in [16, 32, 64] {
                std::fs::write(out.join(format!("icon{s}.rgba")), scaled(i, s).rgba).unwrap();
            }
            r#"Some(macroquad::miniquad::conf::Icon { small: *include_bytes!(concat!(env!("OUT_DIR"), "/icon16.rgba")), medium: *include_bytes!(concat!(env!("OUT_DIR"), "/icon32.rgba")), big: *include_bytes!(concat!(env!("OUT_DIR"), "/icon64.rgba")) })"#
        }
        None => "None",
    };
    std::fs::write(out.join("window_icon.rs"), code).unwrap();
}

/// An `.ico` of PNG images (Windows reads them at every size).
fn ico(image: &Image) -> Vec<u8> {
    let images: Vec<(u32, Vec<u8>)> = ICO_SIZES.iter().map(|&s| (s, png_bytes(&scaled(image, s)))).collect();
    let mut out = vec![0, 0, 1, 0];
    out.extend((images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, data) in &images {
        let side = if *size >= 256 { 0 } else { *size as u8 };
        out.extend([side, side, 0, 0]);
        out.extend(1u16.to_le_bytes());
        out.extend(32u16.to_le_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(offset.to_le_bytes());
        offset += data.len() as u32;
    }
    for (_, data) in images {
        out.extend(data);
    }
    out
}

fn windows_resource(out: &Path, program: &str, description: &str, image: Option<&Image>) {
    let product = std::env::var("PRODUCT_NAME").unwrap_or_else(|_| "SML2 6GC".into());
    let mut res = winresource::WindowsResource::new();
    if let Some(i) = image {
        let path = out.join(format!("{program}.ico"));
        std::fs::write(&path, ico(i)).unwrap();
        res.set_icon(path.to_str().unwrap());
    }
    res.set("ProductName", &product);
    res.set("FileDescription", description);
    res.set("OriginalFilename", &format!("{program}.exe"));
    res.set("InternalName", program);
    res.set("LegalCopyright", COPYRIGHT);
    if let Err(e) = res.compile() {
        println!("cargo:warning={program}: no icon or version details ({e})");
    }
}
