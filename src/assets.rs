use image::GenericImageView;
use std::{io, path::Path};
use tray_icon::Icon;

const TILE_WIDTH: u32 = 250;
const TILE_HEIGHT: u32 = 250;
const TRAY_ICON_SIZE: u32 = 32;
const TRAY_ICON_PADDING: u32 = 1;

pub struct Atlas {
    pub width: u32,
    pub height: u32,
    pub delays: Vec<u16>,
    pub pixels: Vec<u8>,
}

pub fn load_png_tiles(
    path: &Path,
    frame_count: u32,
    scale: f64,
    delays: Vec<u16>,
) -> io::Result<Atlas> {
    let image = image::open(path).map_err(io::Error::other)?.to_rgba8();
    let columns = image.width() / TILE_WIDTH;
    let rows = image.height() / TILE_HEIGHT;
    if columns == 0 || rows == 0 || u64::from(columns) * u64::from(rows) < u64::from(frame_count) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PNGタイルのサイズまたはframe_countが不正です",
        ));
    }
    let width = scaled_dimension(TILE_WIDTH, scale)?;
    let height = scaled_dimension(TILE_HEIGHT, scale)?;
    let frame_size = frame_size(width, height)?;
    let capacity = frame_size
        .checked_mul(usize::try_from(frame_count).map_err(io::Error::other)?)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "PNGのサイズが大きすぎます"))?;
    let mut pixels = Vec::with_capacity(capacity);
    for frame in 0..frame_count {
        let x = (frame % columns) * TILE_WIDTH;
        let y = (frame / columns) * TILE_HEIGHT;
        let tile = image.view(x, y, TILE_WIDTH, TILE_HEIGHT).to_image();
        let tile =
            image::imageops::resize(&tile, width, height, image::imageops::FilterType::Triangle);
        pixels.extend_from_slice(tile.as_raw());
    }
    Ok(Atlas {
        width,
        height,
        delays,
        pixels,
    })
}

fn scaled_dimension(tile_size: u32, scale: f64) -> io::Result<u32> {
    let dimension = (f64::from(tile_size) * scale).round();
    if !dimension.is_finite() || dimension < 1.0 || dimension > f64::from(u32::MAX) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "scaleによる画像サイズが不正です",
        ));
    }
    Ok(dimension as u32)
}

fn frame_size(width: u32, height: u32) -> io::Result<usize> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|size| size.checked_mul(4))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "画像サイズが大きすぎます"))
}

impl Atlas {
    pub fn frame(&self, index: usize) -> Option<&[u8]> {
        let size = frame_size(self.width, self.height).ok()?;
        let start = index.checked_mul(size)?;
        self.pixels.get(start..start.checked_add(size)?)
    }
}

pub fn tray_icon(atlas: &Atlas) -> Result<Icon, Box<dyn std::error::Error>> {
    if let Some(frame0) = atlas
        .frame(0)
        .and_then(|frame| image::RgbaImage::from_raw(atlas.width, atlas.height, frame.to_vec()))
    {
        let trimmed = trim_transparent(&frame0);
        let available = TRAY_ICON_SIZE - TRAY_ICON_PADDING * 2;
        let longest_side = trimmed.width().max(trimmed.height());
        let width = (trimmed.width() * available / longest_side).max(1);
        let height = (trimmed.height() * available / longest_side).max(1);
        let resized = image::imageops::resize(
            &trimmed,
            width,
            height,
            image::imageops::FilterType::Triangle,
        );
        let mut canvas = image::RgbaImage::new(TRAY_ICON_SIZE, TRAY_ICON_SIZE);
        let x = ((TRAY_ICON_SIZE - width) / 2) as i64;
        let y = ((TRAY_ICON_SIZE - height) / 2) as i64;
        image::imageops::overlay(&mut canvas, &resized, x, y);
        if let Ok(icon) = Icon::from_rgba(canvas.into_raw(), TRAY_ICON_SIZE, TRAY_ICON_SIZE) {
            return Ok(icon);
        }
    }
    Ok(Icon::from_rgba(
        vec![255u8; (TRAY_ICON_SIZE * TRAY_ICON_SIZE * 4) as usize],
        TRAY_ICON_SIZE,
        TRAY_ICON_SIZE,
    )?)
}

fn trim_transparent(image: &image::RgbaImage) -> image::RgbaImage {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] != 0 {
            bounds = Some(match bounds {
                Some((min_x, min_y, max_x, max_y)) => {
                    (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
                }
                None => (x, y, x, y),
            });
        }
    }
    let Some((min_x, min_y, max_x, max_y)) = bounds else {
        return image.clone();
    };
    image::imageops::crop_imm(image, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1).to_image()
}
