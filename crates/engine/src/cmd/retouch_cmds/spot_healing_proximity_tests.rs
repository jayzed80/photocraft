//! #12585: a Proximity Match spot heal fills the dab from nearby texture without taking on the
//! colour of the spot it removes. Content-Aware and Create Texture are not changed.

use photocraft_color::{Color, ColorMode, SampleType};
use photocraft_doc::Size;

use super::*;

/// The issue's 800 × 600 RGB/8 document: grey left half, green right half, a black 20 × 20 spot
/// on the grey, and a 160 × 120 checker of 10 px cells on the green with a red 20 × 20 spot in it.
fn document() -> Session {
    let mut s = Session::new();
    s.add_document(Document::with_background("t", Size::new(800, 600), ColorMode::Rgb, SampleType::U8, Color::rgb(0.5, 0.5, 0.5)), None);
    let rgb = |c: [u8; 3]| [f32::from(c[0]) / 255.0, f32::from(c[1]) / 255.0, f32::from(c[2]) / 255.0, 1.0];
    s.edit("paint", |doc, _| {
        let bg = doc.layers[0].surface_mut().unwrap();
        bg.fill_rect(Rect::new(0, 0, 400, 600), &rgb([128, 128, 128]));
        bg.fill_rect(Rect::new(400, 0, 800, 600), &rgb([40, 160, 60]));
        bg.fill_rect(Rect::new(190, 190, 210, 210), &rgb([0, 0, 0]));
        for cy in 0..12 {
            for cx in 0..16 {
                let c = if (cx + cy) % 2 == 0 { [60, 60, 60] } else { [200, 200, 200] };
                let (x, y) = (440 + cx * 10, 420 + cy * 10);
                bg.fill_rect(Rect::new(x, y, x + 10, y + 10), &rgb(c));
            }
        }
        bg.fill_rect(Rect::new(510, 470, 530, 490), &rgb([220, 30, 30]));
        Ok(())
    })
    .unwrap();
    s
}

fn heal(s: &mut Session, x: i32, y: i32, kind: &str) {
    s.execute("paint.spotHealing", json!({"points": [[x, y]], "size": 40, "hardness": 100, "type": kind})).unwrap();
}

/// The Background's pixel at `(x, y)` in 0–255.
fn rgb(s: &Session, x: i32, y: i32) -> [i32; 3] {
    let px = s.active().unwrap().doc.layers[0].surface().unwrap().pixel(x, y);
    [0, 1, 2].map(|c| (px[c] * 255.0).round() as i32)
}

fn layer(s: &Session) -> Vec<f32> {
    let surf = s.active().unwrap().doc.layers[0].surface().unwrap();
    (0..600).flat_map(|y| (0..800).flat_map(move |x| surf.pixel(x, y))).collect()
}

#[test]
fn proximity_match_on_the_checker_heals_in_greys_without_the_spot_colour() {
    let mut s = document();
    heal(&mut s, 520, 480, "proximityMatch");
    let mut tinted = Vec::new();
    for y in 470..=490 {
        for x in 496..=544 {
            let [r, g, b] = rgb(&s, x, y);
            if (r - g).abs() > 10 || (g - b).abs() > 10 || (r - b).abs() > 10 {
                tinted.push((x, y, [r, g, b]));
            }
        }
    }
    assert!(tinted.is_empty(), "{} tinted pixels, first: {:?}", tinted.len(), &tinted[..tinted.len().min(5)]);
}

#[test]
fn proximity_match_on_a_flat_area_still_heals_to_the_flat_grey() {
    let mut s = document();
    heal(&mut s, 200, 200, "proximityMatch");
    for y in [192, 200] {
        for x in 176..=224 {
            assert_eq!(rgb(&s, x, y), [128, 128, 128], "({x}, {y})");
        }
    }
}

#[test]
fn content_aware_and_create_texture_still_heal_the_spot() {
    for kind in ["contentAware", "createTexture"] {
        let mut s = document();
        let before = layer(&s);
        heal(&mut s, 520, 480, kind);
        assert!(layer(&s) != before, "{kind}: healed");
    }
}
