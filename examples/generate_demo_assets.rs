#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::f32::consts::PI;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("demo"), PathBuf::from);
    fs::create_dir_all(root.join("images"))?;
    fs::create_dir_all(root.join("audio"))?;

    studio().save(root.join("images/studio.png"))?;
    rooftop().save(root.join("images/rooftop.png"))?;
    mira().save(root.join("images/mira.png"))?;
    write_music(&root.join("audio/evening.wav"))?;
    write_signal(&root.join("audio/signal.wav"))?;
    println!("Generated demo assets in {}", root.display());
    Ok(())
}

fn studio() -> RgbaImage {
    let mut image = RgbaImage::new(960, 540);
    vertical_gradient(&mut image, rgb("#627b83"), rgb("#c58a73"));
    rectangle(&mut image, 0, 346, 960, 194, rgba("#29343a"));
    rectangle(&mut image, 54, 54, 476, 306, rgba("#20292f"));
    rectangle(&mut image, 68, 68, 448, 278, rgba("#7e9fa5"));
    vertical_gradient_region(&mut image, 68, 68, 448, 278, rgb("#718f99"), rgb("#dfa07e"));
    for (x, height) in [
        (78, 74),
        (124, 112),
        (180, 86),
        (230, 140),
        (302, 96),
        (360, 126),
        (424, 78),
    ] {
        rectangle(&mut image, x, 346 - height, 40, height, rgba("#34474e"));
        for y in (346 - height + 16..338).step_by(24) {
            rectangle(&mut image, x + 9, y, 6, 8, rgba("#e7bf63"));
            rectangle(&mut image, x + 25, y, 6, 8, rgba("#d7ebe7"));
        }
    }
    rectangle(&mut image, 60, 68, 8, 292, rgba("#13191d"));
    rectangle(&mut image, 516, 68, 8, 292, rgba("#13191d"));
    rectangle(&mut image, 64, 202, 456, 8, rgba("#13191d"));

    rectangle(&mut image, 602, 76, 270, 26, rgba("#1d262c"));
    rectangle(&mut image, 618, 104, 22, 180, rgba("#3d5458"));
    rectangle(&mut image, 834, 104, 22, 180, rgba("#3d5458"));
    for (x, width, tint) in [
        (632, 28, "#d15f51"),
        (665, 20, "#e7bf63"),
        (690, 34, "#83a99d"),
        (730, 22, "#c8d1ca"),
        (758, 38, "#6f8794"),
        (802, 26, "#d58966"),
    ] {
        rectangle(&mut image, x, 110, width, 126, rgba(tint));
    }
    rectangle(&mut image, 548, 364, 360, 24, rgba("#151c20"));
    polygon(
        &mut image,
        &[(590, 388), (870, 388), (930, 540), (520, 540)],
        rgba("#415257"),
    );
    rectangle(&mut image, 650, 322, 148, 76, rgba("#1c252a"));
    rectangle(&mut image, 666, 336, 116, 50, rgba("#8ab1b0"));
    rectangle(&mut image, 438, 428, 92, 16, rgba("#d6a866"));
    line(&mut image, 454, 442, 436, 540, rgba("#151c20"), 7);
    line(&mut image, 514, 442, 538, 540, rgba("#151c20"), 7);
    image
}

fn rooftop() -> RgbaImage {
    let mut image = RgbaImage::new(960, 540);
    vertical_gradient(&mut image, rgb("#35485a"), rgb("#e18d72"));
    ellipse(&mut image, 742, 142, 56, 56, rgba("#f2cf77"));
    polygon(
        &mut image,
        &[
            (0, 328),
            (114, 244),
            (234, 316),
            (356, 218),
            (494, 320),
            (630, 246),
            (782, 320),
            (960, 226),
            (960, 540),
            (0, 540),
        ],
        rgba("#38464d"),
    );
    polygon(
        &mut image,
        &[
            (0, 376),
            (136, 302),
            (286, 366),
            (410, 278),
            (560, 372),
            (718, 306),
            (850, 350),
            (960, 300),
            (960, 540),
            (0, 540),
        ],
        rgba("#253137"),
    );
    rectangle(&mut image, 0, 416, 960, 124, rgba("#1b2429"));
    rectangle(&mut image, 0, 382, 960, 12, rgba("#b7a184"));
    for x in (34..960).step_by(96) {
        rectangle(&mut image, x, 382, 8, 92, rgba("#766f68"));
    }
    rectangle(&mut image, 0, 466, 960, 10, rgba("#766f68"));
    for x in (84..900).step_by(154) {
        rectangle(&mut image, x, 440, 7, 8, rgba("#e7bf63"));
    }
    image
}

fn mira() -> RgbaImage {
    let mut image = RgbaImage::new(420, 620);
    ellipse(&mut image, 210, 160, 116, 146, rgba("#23272e"));
    polygon(
        &mut image,
        &[
            (92, 188),
            (328, 188),
            (370, 438),
            (336, 612),
            (82, 612),
            (50, 438),
        ],
        rgba("#242a31"),
    );
    ellipse(&mut image, 210, 176, 76, 96, rgba("#e8b99d"));
    polygon(
        &mut image,
        &[
            (130, 136),
            (164, 72),
            (250, 66),
            (301, 130),
            (276, 112),
            (242, 104),
            (198, 118),
            (160, 108),
        ],
        rgba("#30343c"),
    );
    ellipse(&mut image, 180, 174, 7, 5, rgba("#39484e"));
    ellipse(&mut image, 239, 174, 7, 5, rgba("#39484e"));
    line(&mut image, 190, 215, 229, 215, rgba("#a95f58"), 3);
    rectangle(&mut image, 185, 254, 50, 48, rgba("#dfad94"));
    polygon(
        &mut image,
        &[
            (110, 282),
            (176, 258),
            (210, 300),
            (244, 258),
            (310, 282),
            (350, 612),
            (70, 612),
        ],
        rgba("#d56355"),
    );
    polygon(
        &mut image,
        &[
            (176, 258),
            (210, 300),
            (244, 258),
            (272, 282),
            (238, 342),
            (210, 316),
            (182, 342),
            (148, 282),
        ],
        rgba("#f0e4d5"),
    );
    polygon(
        &mut image,
        &[(70, 612), (110, 282), (154, 280), (174, 612)],
        rgba("#b84d46"),
    );
    polygon(
        &mut image,
        &[(350, 612), (310, 282), (266, 280), (246, 612)],
        rgba("#b84d46"),
    );
    line(&mut image, 210, 316, 210, 612, rgba("#893d3b"), 3);
    image
}

fn write_music(path: &Path) -> std::io::Result<()> {
    let sample_rate = 44_100_u32;
    let seconds = 8_u32;
    let count = sample_rate * seconds;
    let mut samples = Vec::with_capacity(count as usize);
    let notes = [220.0_f32, 277.18, 329.63];
    for index in 0..count {
        let time = index as f32 / sample_rate as f32;
        let phrase = ((time / 2.0) as usize).min(notes.len() - 1);
        let fade = (time.min(0.8) / 0.8).min(((seconds as f32 - time).max(0.0) / 0.8).min(1.0));
        let fundamental = (2.0 * PI * notes[phrase] * time).sin();
        let overtone = (2.0 * PI * notes[phrase] * 1.5 * time).sin() * 0.32;
        samples.push(((fundamental + overtone) * fade * 2_800.0) as i16);
    }
    write_wav(path, sample_rate, &samples)
}

fn write_signal(path: &Path) -> std::io::Result<()> {
    let sample_rate = 44_100_u32;
    let count = sample_rate / 3;
    let mut samples = Vec::with_capacity(count as usize);
    for index in 0..count {
        let time = index as f32 / sample_rate as f32;
        let envelope = (1.0 - time * 3.0).max(0.0);
        let wave = (2.0 * PI * (680.0 + time * 420.0) * time).sin();
        samples.push((wave * envelope * 8_000.0) as i16);
    }
    write_wav(path, sample_rate, &samples)
}

fn write_wav(path: &Path, sample_rate: u32, samples: &[i16]) -> std::io::Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    let data_size = u32::try_from(samples.len() * 2).expect("demo audio fits in a WAV file");
    writer.write_all(b"RIFF")?;
    writer.write_all(&(36 + data_size).to_le_bytes())?;
    writer.write_all(b"WAVEfmt ")?;
    writer.write_all(&16_u32.to_le_bytes())?;
    writer.write_all(&1_u16.to_le_bytes())?;
    writer.write_all(&1_u16.to_le_bytes())?;
    writer.write_all(&sample_rate.to_le_bytes())?;
    writer.write_all(&(sample_rate * 2).to_le_bytes())?;
    writer.write_all(&2_u16.to_le_bytes())?;
    writer.write_all(&16_u16.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&data_size.to_le_bytes())?;
    for sample in samples {
        writer.write_all(&sample.to_le_bytes())?;
    }
    writer.flush()
}

fn vertical_gradient(image: &mut RgbaImage, from: [u8; 3], to: [u8; 3]) {
    vertical_gradient_region(image, 0, 0, image.width(), image.height(), from, to);
}

fn vertical_gradient_region(
    image: &mut RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    from: [u8; 3],
    to: [u8; 3],
) {
    for offset_y in 0..height {
        let amount = offset_y as f32 / height.max(1) as f32;
        let pixel = Rgba([
            lerp(from[0], to[0], amount),
            lerp(from[1], to[1], amount),
            lerp(from[2], to[2], amount),
            255,
        ]);
        for offset_x in 0..width {
            put(image, x + offset_x, y + offset_y, pixel);
        }
    }
}

fn rectangle(image: &mut RgbaImage, x: u32, y: u32, width: u32, height: u32, pixel: Rgba<u8>) {
    for offset_y in 0..height {
        for offset_x in 0..width {
            put(image, x + offset_x, y + offset_y, pixel);
        }
    }
}

fn ellipse(
    image: &mut RgbaImage,
    center_x: i32,
    center_y: i32,
    radius_x: i32,
    radius_y: i32,
    pixel: Rgba<u8>,
) {
    for y in center_y - radius_y..=center_y + radius_y {
        for x in center_x - radius_x..=center_x + radius_x {
            let dx = (x - center_x) as f32 / radius_x as f32;
            let dy = (y - center_y) as f32 / radius_y as f32;
            if dx * dx + dy * dy <= 1.0 {
                put_signed(image, x, y, pixel);
            }
        }
    }
}

fn polygon(image: &mut RgbaImage, points: &[(i32, i32)], pixel: Rgba<u8>) {
    let min_x = points.iter().map(|point| point.0).min().unwrap_or(0);
    let max_x = points.iter().map(|point| point.0).max().unwrap_or(0);
    let min_y = points.iter().map(|point| point.1).min().unwrap_or(0);
    let max_y = points.iter().map(|point| point.1).max().unwrap_or(0);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if point_in_polygon(x, y, points) {
                put_signed(image, x, y, pixel);
            }
        }
    }
}

fn point_in_polygon(x: i32, y: i32, points: &[(i32, i32)]) -> bool {
    let mut inside = false;
    let mut previous = points.len() - 1;
    for current in 0..points.len() {
        let (current_x, current_y) = points[current];
        let (previous_x, previous_y) = points[previous];
        if (current_y > y) != (previous_y > y)
            && x < (previous_x - current_x) * (y - current_y) / (previous_y - current_y) + current_x
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn line(image: &mut RgbaImage, x0: i32, y0: i32, x1: i32, y1: i32, pixel: Rgba<u8>, width: i32) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    let (mut x, mut y) = (x0, y0);
    loop {
        for offset_y in -width / 2..=width / 2 {
            for offset_x in -width / 2..=width / 2 {
                put_signed(image, x + offset_x, y + offset_y, pixel);
            }
        }
        if x == x1 && y == y1 {
            break;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx;
        }
        if twice <= dx {
            error += dx;
            y += sy;
        }
    }
}

fn put(image: &mut RgbaImage, x: u32, y: u32, pixel: Rgba<u8>) {
    if x < image.width() && y < image.height() {
        image.put_pixel(x, y, pixel);
    }
}

fn put_signed(image: &mut RgbaImage, x: i32, y: i32, pixel: Rgba<u8>) {
    if let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) {
        put(image, x, y, pixel);
    }
}

fn lerp(from: u8, to: u8, amount: f32) -> u8 {
    (f32::from(from) + (f32::from(to) - f32::from(from)) * amount).round() as u8
}

fn rgb(hex: &str) -> [u8; 3] {
    let pixel = rgba(hex).0;
    [pixel[0], pixel[1], pixel[2]]
}

fn rgba(hex: &str) -> Rgba<u8> {
    let value = hex.strip_prefix('#').expect("colors start with #");
    Rgba([
        u8::from_str_radix(&value[0..2], 16).expect("valid red"),
        u8::from_str_radix(&value[2..4], 16).expect("valid green"),
        u8::from_str_radix(&value[4..6], 16).expect("valid blue"),
        255,
    ])
}
