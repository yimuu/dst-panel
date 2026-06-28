//! Map image generation and image-file reading helpers.

use std::{
    io,
    path::{Path, PathBuf},
};

use crate::dst;

use super::session::{MapLevel, latest_world_session_file, read_capped_bytes, read_cluster_text};

const TILE_SCALE: usize = 16;
const MAX_MAP_IMAGE_SIDE: usize = 2048;
const MAX_MAP_PIXELS: usize = 16_777_216;
const MAX_MAP_IMAGE_BYTES: u64 = 72 * 1024 * 1024;
const MAX_TILES_BASE64_BYTES: usize = 8 * 1024 * 1024;
const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// Generates the legacy map image file and returns its cluster-relative path.
pub fn generate_map_image(cluster_dir: &Path, level: &MapLevel) -> io::Result<PathBuf> {
    let world_relative_path = latest_world_session_file(cluster_dir, level)?;
    let world_contents = read_cluster_text(cluster_dir, &world_relative_path)?;
    let png = render_session_map_png(&world_contents)?;
    let image_relative_path = image_relative_path(level);

    // Go writes PNG bytes through `png.Encode` while keeping a `.jpg` filename.
    // Preserve that oddity so existing frontend code can request the same path.
    dst::safe_write_cluster_file(cluster_dir, &image_relative_path, &png)?;
    tracing::info!(
        level_name = level.as_str(),
        bytes = png.len(),
        "generated DST map PNG at legacy jpg path"
    );
    Ok(image_relative_path)
}

/// Reads a previously generated map image.
pub fn read_map_image(cluster_dir: &Path, level: &MapLevel) -> io::Result<Option<Vec<u8>>> {
    let relative_path = image_relative_path(level);
    let Some(mut file) = dst::safe_open_cluster_file(cluster_dir, &relative_path)? else {
        return Ok(None);
    };
    let bytes = read_capped_bytes(
        &mut file,
        MAX_MAP_IMAGE_BYTES,
        "map image exceeds safety limit",
    )?;
    Ok(Some(bytes))
}

fn image_relative_path(level: &MapLevel) -> PathBuf {
    PathBuf::from(format!("dst_map_{}.jpg", level.as_str()))
}

fn render_session_map_png(contents: &str) -> io::Result<Vec<u8>> {
    let height = extract_dimension(contents, "height")?;
    let width = extract_dimension(contents, "width")?;
    if height == 0 || width == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "地图尺寸不能为空",
        ));
    }

    // Preserve Go's call-site quirk: ExtractDimensions returns height,width but
    // the caller passes them into GenerateMap(width,height).
    let image_tile_width = height;
    let image_tile_height = width;
    let tile_scale = tile_scale_for_dimensions(image_tile_width, image_tile_height)?;
    let image_width = image_tile_width
        .checked_mul(tile_scale)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "地图宽度过大"))?;
    let image_height = image_tile_height
        .checked_mul(tile_scale)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "地图高度过大"))?;

    let tiles_base64 = extract_tiles_base64(contents)?;
    if tiles_base64.len() > MAX_TILES_BASE64_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "地图数据超过安全限制",
        ));
    }
    let tile_ids = decode_tile_ids(tiles_base64)?;
    let pixels = render_rgba_pixels(&tile_ids, image_tile_width, image_tile_height, tile_scale);
    encode_png_rgba(image_width as u32, image_height as u32, &pixels)
}

fn tile_scale_for_dimensions(width: usize, height: usize) -> io::Result<usize> {
    let tile_count = width
        .checked_mul(height)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "地图尺寸超过安全限制"))?;
    if tile_count == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "地图尺寸不能为空",
        ));
    }

    let max_side = width.max(height);
    let max_scale_by_side = MAX_MAP_IMAGE_SIDE / max_side;
    let max_scale_by_pixels = ((MAX_MAP_PIXELS / tile_count) as f64).sqrt().floor() as usize;
    let max_scale = max_scale_by_side.min(max_scale_by_pixels);
    if max_scale == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "地图尺寸超过安全限制",
        ));
    }

    Ok(TILE_SCALE.min(max_scale))
}

fn extract_dimension(contents: &str, key: &str) -> io::Result<usize> {
    let marker = format!("{key}=");
    let Some(start) = contents.find(&marker).map(|index| index + marker.len()) else {
        return Ok(0);
    };
    let digits: String = contents[start..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return Ok(0);
    }
    digits
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "地图尺寸格式错误"))
}

fn extract_tiles_base64(contents: &str) -> io::Result<&str> {
    let marker = "tiles=\"";
    let start = contents
        .find(marker)
        .map(|index| index + marker.len())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "无法在存档中找到地图数据"))?;
    let end = contents[start..]
        .find('"')
        .map(|index| start + index)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "无法在存档中找到地图数据"))?;
    Ok(&contents[start..end])
}

fn decode_tile_ids(tiles_base64: &str) -> io::Result<Vec<i32>> {
    let mut bytes = decode_base64(tiles_base64)?;
    let mut data_start = 0;
    if bytes.len() > 5 && bytes.starts_with(b"VRSTN") {
        data_start = 5;
        while data_start < bytes.len() && bytes[data_start] == 0 {
            data_start += 1;
        }
    }
    bytes.drain(..data_start);
    if bytes.len() % 2 != 0 {
        bytes.pop();
    }

    let mut tile_ids = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let encoded = (i32::from(pair[1]) << 8) | i32::from(pair[0]);
        tile_ids.push(restore_tile_id(encoded));
    }
    Ok(tile_ids)
}

fn decode_base64(input: &str) -> io::Result<Vec<u8>> {
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let mut buffer: u32 = 0;
    let mut bits = 0_u8;
    let mut saw_padding = false;

    for byte in input.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        if byte == b'=' {
            saw_padding = true;
            continue;
        }
        if saw_padding {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "base64解码失败"));
        }
        let value = base64_value(byte)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "base64解码失败"))?;
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        while bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xff) as u8);
        }
        if bits > 0 {
            buffer &= (1 << bits) - 1;
        } else {
            buffer = 0;
        }
    }

    Ok(output)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn restore_tile_id(original: i32) -> i32 {
    let high = original >> 8;
    if tile_color(high).is_some() {
        return high;
    }
    if high > 0 && tile_color(high - 1).is_some() {
        return high - 1;
    }
    if (0xC8..=0xFF).contains(&high) {
        let candidate = 200 + (high - 0xC8);
        if tile_color(candidate).is_some() {
            return candidate;
        }
    }
    0
}

fn render_rgba_pixels(tile_ids: &[i32], width: usize, height: usize, scale: usize) -> Vec<u8> {
    let image_width = width * scale;
    let image_height = height * scale;
    let mut pixels = vec![0_u8; image_width * image_height * 4];

    for y in 0..height {
        for x in 0..width {
            let Some(tile_id) = tile_ids.get(y * width + x) else {
                continue;
            };
            let (red, green, blue) = tile_color(*tile_id).unwrap_or((0, 0, 0));
            let flipped_x = width - x - 1;
            for dy in 0..scale {
                for dx in 0..scale {
                    let px = flipped_x * scale + dx;
                    let py = y * scale + dy;
                    let offset = (py * image_width + px) * 4;
                    pixels[offset] = red;
                    pixels[offset + 1] = green;
                    pixels[offset + 2] = blue;
                    pixels[offset + 3] = 255;
                }
            }
        }
    }

    pixels
}

fn tile_color(tile_id: i32) -> Option<(u8, u8, u8)> {
    match tile_id {
        0 => Some((42, 42, 42)),
        1 => Some((42, 42, 44)),
        2 => Some((80, 76, 65)),
        3 => Some((90, 105, 104)),
        4 => Some((117, 107, 85)),
        5 => Some((144, 125, 89)),
        6 => Some((48, 67, 39)),
        7 => Some((40, 47, 18)),
        8 => Some((81, 28, 194)),
        9 => Some((0, 0, 7)),
        10 => Some((85, 69, 48)),
        11 => Some((64, 75, 116)),
        12 => Some((115, 133, 201)),
        13 => Some((139, 131, 115)),
        14 => Some((74, 65, 77)),
        15 => Some((67, 70, 32)),
        16 => Some((75, 75, 73)),
        17 => Some((66, 49, 30)),
        18 => Some((115, 113, 107)),
        19 => Some((86, 86, 81)),
        20 => Some((74, 61, 84)),
        21 => Some((66, 50, 76)),
        22 => Some((39, 38, 39)),
        23 => Some((34, 35, 34)),
        24 => Some((70, 44, 43)),
        25 => Some((62, 76, 61)),
        26..=29 => Some((42, 42, 44)),
        30 => Some((91, 62, 14)),
        31 => Some((117, 86, 46)),
        32 => Some((31, 27, 27)),
        33 | 34 => Some((128, 128, 128)),
        35 => Some((47, 44, 47)),
        36 => Some((158, 104, 105)),
        37 => Some((137, 113, 113)),
        38 => Some((81, 97, 100)),
        39 => Some((66, 58, 49)),
        40 => Some((42, 42, 44)),
        41 => Some((119, 113, 97)),
        42 => Some((84, 108, 107)),
        43 => Some((67, 133, 142)),
        44 => Some((154, 146, 186)),
        45 => Some((138, 96, 73)),
        46 => Some((66, 86, 82)),
        47 => Some((61, 57, 46)),
        48 => Some((42, 42, 44)),
        200 => Some((0, 0, 11)),
        201 | 202 => Some((18, 66, 73)),
        203 => Some((7, 46, 61)),
        204 => Some((1, 32, 46)),
        205 | 206 => Some((6, 54, 81)),
        207 => Some((0, 24, 26)),
        208 => Some((189, 193, 198)),
        247 => Some((42, 42, 44)),
        _ => None,
    }
}

fn encode_png_rgba(width: u32, height: u32, pixels: &[u8]) -> io::Result<Vec<u8>> {
    let row_len = width as usize * 4;
    if pixels.len() != row_len * height as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PNG pixel buffer size mismatch",
        ));
    }

    let mut raw = Vec::with_capacity((row_len + 1) * height as usize);
    for y in 0..height as usize {
        raw.push(0);
        let start = y * row_len;
        raw.extend_from_slice(&pixels[start..start + row_len]);
    }

    let compressed = zlib_store_blocks(&raw);
    let mut png = Vec::new();
    png.extend_from_slice(PNG_SIGNATURE);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    write_png_chunk(&mut png, b"IHDR", &ihdr);
    write_png_chunk(&mut png, b"IDAT", &compressed);
    write_png_chunk(&mut png, b"IEND", &[]);
    Ok(png)
}

fn zlib_store_blocks(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    for (index, chunk) in data.chunks(65_535).enumerate() {
        let is_last = (index + 1) * 65_535 >= data.len();
        out.push(if is_last { 0x01 } else { 0x00 });
        let len = chunk.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn write_png_chunk(output: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(chunk_type);
    output.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(chunk_type.len() + data.len());
    crc_input.extend_from_slice(chunk_type);
    crc_input.extend_from_slice(data);
    output.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

fn adler32(data: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65_521;
    let mut a = 1_u32;
    let mut b = 0_u32;
    for byte in data {
        a = (a + u32::from(*byte)) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }
    (b << 16) | a
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}
