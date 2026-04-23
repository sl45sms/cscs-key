use std::env;
use std::fs;
use std::path::Path;

use anyhow::Context;
use image::imageops::FilterType;
use image::{Rgba, RgbaImage};

const BASE_SIZE: u32 = 1024;
const OUTPUTS: &[(&str, u32)] = &[
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
];

fn main() -> anyhow::Result<()> {
    let output_dir = env::args()
        .nth(1)
        .context("usage: make_macos_icon <iconset-dir>")?;
    let output_dir = Path::new(&output_dir);
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    let master = render_master_icon(BASE_SIZE);
    for (file_name, size) in OUTPUTS {
        let icon = image::imageops::resize(&master, *size, *size, FilterType::Lanczos3);
        icon.save(output_dir.join(file_name))
            .with_context(|| format!("failed to write {}", file_name))?;
    }

    Ok(())
}

fn render_master_icon(size: u32) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    paint_background(&mut image);
    paint_glow(&mut image);
    paint_key_shadow(&mut image);
    paint_key(&mut image);
    paint_badge_shadow(&mut image);
    paint_badge(&mut image);
    paint_badge_check(&mut image);
    image
}

fn paint_background(image: &mut RgbaImage) {
    let size = image.width() as f32;
    let center = size * 0.5;
    let half = size * 0.41;
    let radius = size * 0.22;
    let feather = 1.6;

    for y in 0..image.height() {
        for x in 0..image.width() {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let sd = sdf_rounded_rect(px, py, center, center, half, half, radius);
            let alpha = coverage(sd, feather);
            if alpha <= 0.0 {
                continue;
            }

            let nx = px / size;
            let ny = py / size;
            let diagonal = ((nx * 0.72) + (ny * 0.95)).clamp(0.0, 1.0);
            let mut color = mix([17.0, 35.0, 55.0], [38.0, 85.0, 106.0], diagonal);

            let warm = radial(px, py, size * 0.24, size * 0.18, size * 0.52);
            let cool = radial(px, py, size * 0.76, size * 0.84, size * 0.70);
            color = mix_color(color, [214.0, 111.0, 69.0], warm * 0.78);
            color = mix_color(color, [101.0, 195.0, 186.0], cool * 0.18);

            let border_band = (18.0 - sd.abs()).clamp(0.0, 18.0) / 18.0;
            color = mix_color(color, [241.0, 225.0, 205.0], border_band * 0.12);

            blend_pixel(image.get_pixel_mut(x, y), color, alpha);
        }
    }
}

fn paint_glow(image: &mut RgbaImage) {
    let size = image.width() as f32;
    let feather = 2.0;
    let center_x = size * 0.48;
    let center_y = size * 0.5;

    for y in 0..image.height() {
        for x in 0..image.width() {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let sd = sdf_circle(px, py, center_x, center_y, size * 0.27);
            let alpha = coverage(sd, feather) * 0.16;
            if alpha > 0.0 {
                blend_pixel(image.get_pixel_mut(x, y), [255.0, 236.0, 213.0], alpha);
            }
        }
    }
}

fn paint_key_shadow(image: &mut RgbaImage) {
    let size = image.width() as f32;
    let feather = 3.0;

    for y in 0..image.height() {
        for x in 0..image.width() {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let sd = key_sdf(px + size * 0.024, py + size * 0.03, size);
            let alpha = coverage(sd, feather) * 0.28;
            if alpha > 0.0 {
                blend_pixel(image.get_pixel_mut(x, y), [12.0, 18.0, 32.0], alpha);
            }
        }
    }
}

fn paint_key(image: &mut RgbaImage) {
    let size = image.width() as f32;
    let feather = 2.0;

    for y in 0..image.height() {
        for x in 0..image.width() {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let sd = key_sdf(px, py, size);
            let alpha = coverage(sd, feather);
            if alpha <= 0.0 {
                continue;
            }

            let highlight = radial(px, py, size * 0.34, size * 0.30, size * 0.42) * 0.35;
            let color = mix_color([248.0, 240.0, 227.0], [255.0, 253.0, 246.0], highlight);
            blend_pixel(image.get_pixel_mut(x, y), color, alpha);

            let stroke = (10.0 - sd.abs()).clamp(0.0, 10.0) / 10.0;
            if stroke > 0.0 {
                blend_pixel(image.get_pixel_mut(x, y), [228.0, 214.0, 190.0], stroke * 0.18);
            }
        }
    }
}

fn paint_badge_shadow(image: &mut RgbaImage) {
    let size = image.width() as f32;
    let feather = 3.0;

    for y in 0..image.height() {
        for x in 0..image.width() {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let sd = sdf_circle(px + size * 0.014, py + size * 0.018, size * 0.76, size * 0.28, size * 0.12);
            let alpha = coverage(sd, feather) * 0.25;
            if alpha > 0.0 {
                blend_pixel(image.get_pixel_mut(x, y), [12.0, 18.0, 32.0], alpha);
            }
        }
    }
}

fn paint_badge(image: &mut RgbaImage) {
    let size = image.width() as f32;
    let feather = 2.0;
    let cx = size * 0.76;
    let cy = size * 0.28;
    let radius = size * 0.12;

    for y in 0..image.height() {
        for x in 0..image.width() {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let sd = sdf_circle(px, py, cx, cy, radius);
            let alpha = coverage(sd, feather);
            if alpha <= 0.0 {
                continue;
            }

            let glow = radial(px, py, cx - size * 0.018, cy - size * 0.02, radius * 1.3);
            let color = mix_color([206.0, 93.0, 60.0], [244.0, 145.0, 87.0], glow * 0.52);
            blend_pixel(image.get_pixel_mut(x, y), color, alpha);

            let ring = (8.0 - (sd + size * 0.01).abs()).clamp(0.0, 8.0) / 8.0;
            if ring > 0.0 {
                blend_pixel(image.get_pixel_mut(x, y), [255.0, 242.0, 225.0], ring * 0.22);
            }
        }
    }
}

fn paint_badge_check(image: &mut RgbaImage) {
    let size = image.width() as f32;
    let feather = 1.8;

    for y in 0..image.height() {
        for x in 0..image.width() {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let sd = badge_check_sdf(px, py, size);
            let alpha = coverage(sd, feather);
            if alpha > 0.0 {
                blend_pixel(image.get_pixel_mut(x, y), [253.0, 247.0, 237.0], alpha);
            }
        }
    }
}

fn key_sdf(px: f32, py: f32, size: f32) -> f32 {
    let scale = size / BASE_SIZE as f32;
    let (x, y) = rotate_point(px, py, size * 0.5, size * 0.5, -18.0_f32.to_radians());

    let ring_outer = sdf_circle(x, y, 356.0 * scale, 400.0 * scale, 164.0 * scale);
    let ring_inner = sdf_circle(x, y, 356.0 * scale, 400.0 * scale, 82.0 * scale);
    let ring = ring_outer.max(-ring_inner);

    let shaft = sdf_rounded_rect(
        x,
        y,
        566.0 * scale,
        516.0 * scale,
        246.0 * scale,
        58.0 * scale,
        58.0 * scale,
    );
    let tooth_one = sdf_rounded_rect(
        x,
        y,
        748.0 * scale,
        588.0 * scale,
        40.0 * scale,
        54.0 * scale,
        16.0 * scale,
    );
    let tooth_two = sdf_rounded_rect(
        x,
        y,
        834.0 * scale,
        634.0 * scale,
        56.0 * scale,
        34.0 * scale,
        16.0 * scale,
    );
    let top_notch = sdf_rounded_rect(
        x,
        y,
        744.0 * scale,
        470.0 * scale,
        38.0 * scale,
        22.0 * scale,
        12.0 * scale,
    );

    let body = ring.min(shaft).min(tooth_one).min(tooth_two);
    body.max(-top_notch)
}

fn badge_check_sdf(px: f32, py: f32, size: f32) -> f32 {
    let scale = size / BASE_SIZE as f32;
    let first = sdf_segment(
        px,
        py,
        722.0 * scale,
        286.0 * scale,
        770.0 * scale,
        338.0 * scale,
        22.0 * scale,
    );
    let second = sdf_segment(
        px,
        py,
        770.0 * scale,
        338.0 * scale,
        850.0 * scale,
        248.0 * scale,
        22.0 * scale,
    );
    first.min(second)
}

fn sdf_circle(px: f32, py: f32, cx: f32, cy: f32, radius: f32) -> f32 {
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt() - radius
}

fn sdf_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32, radius: f32) -> f32 {
    let pax = px - ax;
    let pay = py - ay;
    let bax = bx - ax;
    let bay = by - ay;
    let h = ((pax * bax + pay * bay) / (bax * bax + bay * bay)).clamp(0.0, 1.0);
    ((pax - bax * h).powi(2) + (pay - bay * h).powi(2)).sqrt() - radius
}

fn sdf_rounded_rect(px: f32, py: f32, cx: f32, cy: f32, half_w: f32, half_h: f32, radius: f32) -> f32 {
    let qx = (px - cx).abs() - half_w + radius;
    let qy = (py - cy).abs() - half_h + radius;
    let ox = qx.max(0.0);
    let oy = qy.max(0.0);
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - radius
}

fn rotate_point(px: f32, py: f32, cx: f32, cy: f32, angle: f32) -> (f32, f32) {
    let sin = angle.sin();
    let cos = angle.cos();
    let dx = px - cx;
    let dy = py - cy;
    (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
}

fn radial(px: f32, py: f32, cx: f32, cy: f32, radius: f32) -> f32 {
    let distance = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
    (1.0 - (distance / radius)).clamp(0.0, 1.0).powf(1.8)
}

fn coverage(sd: f32, feather: f32) -> f32 {
    ((feather - sd) / (feather * 2.0)).clamp(0.0, 1.0)
}

fn blend_pixel(pixel: &mut Rgba<u8>, color: [f32; 3], opacity: f32) {
    let src_alpha = opacity.clamp(0.0, 1.0);
    if src_alpha <= 0.0 {
        return;
    }

    let dst_alpha = pixel[3] as f32 / 255.0;
    let out_alpha = src_alpha + dst_alpha * (1.0 - src_alpha);
    if out_alpha <= 0.0 {
        return;
    }

    let mut channels = [0_u8; 4];
    for channel in 0..3 {
        let src = color[channel] / 255.0;
        let dst = pixel[channel] as f32 / 255.0;
        let out = (src * src_alpha + dst * dst_alpha * (1.0 - src_alpha)) / out_alpha;
        channels[channel] = (out.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    channels[3] = (out_alpha * 255.0).round() as u8;
    *pixel = Rgba(channels);
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn mix_color(base: [f32; 3], overlay: [f32; 3], amount: f32) -> [f32; 3] {
    mix(base, overlay, amount.clamp(0.0, 1.0))
}