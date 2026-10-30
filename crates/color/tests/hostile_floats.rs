//! Blend and colour-space maths must not panic on NaN, infinite or huge samples.

use photocraft_color::BlendMode;
use photocraft_color::blend::{blend_channel, blend_channel_hdr, blend_rgb, blend_rgb_hdr, blend_rgb_within, composite, composite_atop, lum, set_lum};
use photocraft_color::convert::*;

const NASTY: [f32; 12] = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN, f32::MIN_POSITIVE, -0.0, 0.0, 1.0, -1.0, 1e-30, 65504.0];

fn modes() -> Vec<BlendMode> {
    BlendMode::layer_modes().collect()
}

#[test]
fn blend_channel_survives_nasty_floats() {
    for m in modes() {
        for &a in &NASTY {
            for &b in &NASTY {
                let _ = blend_channel(m, a, b);
                let _ = blend_channel_hdr(m, a, b);
            }
        }
    }
}

#[test]
fn blend_rgb_and_composite_survive_nasty_floats() {
    for m in modes() {
        for &a in &NASTY {
            for &b in &NASTY {
                let cb = [a, b, a];
                let cs = [b, a, b];
                let _ = blend_rgb(m, cb, cs);
                let _ = blend_rgb_hdr(m, cb, cs);
                let _ = blend_rgb_within(m, cb, cs, a);
                let _ = composite(m, [a, b, a, b], [b, a, b, a], a);
                let _ = composite_atop(m, [a, b, a, b], [b, a, b, a], b);
            }
        }
    }
}

#[test]
fn colour_conversions_survive_nasty_floats() {
    for &a in &NASTY {
        for &b in &NASTY {
            let _ = lum([a, b, a]);
            let _ = set_lum([a, b, a], b);
            let _ = srgb_to_linear(a);
            let _ = linear_to_srgb(a);
            let _ = cmyk_to_rgb([a, b, a, b]);
            let _ = rgb_to_cmyk([a, b, a]);
            let _ = cmyk_to_rgb_naive([a, b, a, b]);
            let _ = rgb_to_cmyk_naive([a, b, a]);
            let _ = lab_to_srgb([a, b, a]);
            let _ = srgb_to_lab([a, b, a]);
            let _ = rgb_to_gray([a, b, a]);
            let _ = rgb_to_hsb([a, b, a]);
            let _ = hsb_to_rgb([a, b, a]);
            let _ = rgb_to_hsb_sextant([a, b, a]);
            let _ = rgb_from_hue_sextant(a, b, a);
        }
    }
}
