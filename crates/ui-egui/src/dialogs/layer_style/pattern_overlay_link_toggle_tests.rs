use std::sync::Arc;

use egui::accesskit::Role;
use egui::vec2;
use egui_kittest::{Harness, kittest::NodeT, kittest::Queryable};
use photocraft_color::PixelFormat;
use photocraft_doc::{Document, Effect, LayerId, Pattern, Rect};
use photocraft_raster::Surface;
use serde_json::{Value, json};

use super::tests::{DialogId, EffectId, EffectKind, HostedLayerStyle, draft, ok};
use super::{describe_params, open, preview_document, take};
use crate::PhotocraftApp;

fn quadrant_pattern() -> Pattern {
    let mut tile = Surface::new(PixelFormat::RGBA8);
    tile.fill_rect(Rect::new(0, 0, 10, 10), &[1.0, 0.0, 0.0, 1.0]);
    tile.fill_rect(Rect::new(10, 0, 20, 10), &[0.0, 1.0, 0.0, 1.0]);
    tile.fill_rect(Rect::new(0, 10, 10, 20), &[0.0, 0.0, 1.0, 1.0]);
    tile.fill_rect(Rect::new(10, 10, 20, 20), &[0.0, 0.0, 0.0, 1.0]);
    tile.fill_rect(Rect::new(0, 0, 1, 1), &[1.0, 1.0, 1.0, 1.0]);
    Pattern::new("TestQuadrants", tile, 20, 20)
}

fn setup_test_doc(link: bool, phase: (f32, f32)) -> (Box<PhotocraftApp>, LayerId, String) {
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
    app.run("file.new", json!({"width": 400, "height": 300, "resolution": 72, "mode": "rgb", "depth": 8, "background": "white"})).unwrap();
    let pat = quadrant_pattern();
    let pat_id = pat.id.clone();
    app.session.patterns.items.push(pat);

    let layer_id = LayerId(app.run("layer.new.layer", json!({"name": "Shape"})).unwrap()["layer"].as_u64().unwrap());
    app.session
        .edit("Fill Shape", |doc, _| {
            let layer = doc.layer_mut(layer_id).unwrap();
            layer.surface_mut().unwrap().fill_rect(Rect::new(100, 75, 300, 225), &[1.0, 128.0 / 255.0, 0.0, 1.0]);
            Ok(())
        })
        .unwrap();

    app.run(
        "layer.layerStyle.patternOverlay",
        json!({
            "pattern": pat_id,
            "blend": "Normal",
            "opacity": 100,
            "angle": 0,
            "scale": 100,
            "link": link,
            "phaseX": phase.0,
            "phaseY": phase.1,
        }),
    )
    .unwrap();
    (app, layer_id, pat_id)
}

fn pattern_overlay(doc: &Document, layer: LayerId) -> (bool, (f32, f32)) {
    let items = &doc.layer(layer).unwrap().effects.items;
    match &items[0] {
        Effect::PatternOverlay { link, phase, .. } => (*link, *phase),
        other => panic!("expected PatternOverlay, got {other:?}"),
    }
}

fn harness(app: Box<PhotocraftApp>) -> Harness<'static, Box<PhotocraftApp>> {
    let mut h = Harness::builder().with_size(vec2(1500.0, 1000.0)).with_context(app.ctx().clone()).build_ui_state(
        |ui, app: &mut Box<PhotocraftApp>| {
            app.framework.dialogs.run(ui);
            take(app, ui.ctx());
        },
        app,
    );
    h.run_steps(4);
    h
}

fn click_link_checkbox(h: &mut Harness<'static, Box<PhotocraftApp>>) {
    h.get_by_role_and_label(Role::CheckBox, "Link with Layer").click();
    h.run_steps(3);
}

fn link_checked(h: &Harness<'static, Box<PhotocraftApp>>) -> bool {
    h.get_by_role_and_label(Role::CheckBox, "Link with Layer").accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
}

fn row_letters(buf: &photocraft_compose::Buffer, y: i32, x0: i32, len: i32) -> String {
    (x0..x0 + len)
        .map(|x| {
            let [r, g, b, _] = buf.get(x, y);
            if r > 0.9 && g > 0.9 && b > 0.9 {
                'W'
            } else if r > 0.9 && g < 0.1 && b < 0.1 {
                'R'
            } else if r < 0.1 && g > 0.9 && b < 0.1 {
                'G'
            } else if r < 0.1 && g < 0.1 && b > 0.9 {
                'B'
            } else if r < 0.1 && g < 0.1 && b < 0.1 {
                'K'
            } else {
                '?'
            }
        })
        .collect()
}

fn assert_same_pixels(a: &photocraft_compose::Buffer, b: &photocraft_compose::Buffer, why: &str) {
    assert_eq!(a.rect, b.rect, "{why}: rect mismatch");
    for y in a.rect.y0..a.rect.y1 {
        for x in a.rect.x0..a.rect.x1 {
            let pa = a.get(x, y);
            let pb = b.get(x, y);
            assert!(pa.iter().zip(pb).all(|(ca, cb)| (ca - cb).abs() <= 1e-4), "{why}: pixel ({x}, {y}) differs: {pa:?} != {pb:?}");
        }
    }
}

fn open_pattern_overlay(app: &mut PhotocraftApp) -> (DialogId, EffectId) {
    let id = open(app, Some(EffectKind::PatternOverlay.key())).unwrap();
    let overlay = draft(app, id).style.instances(EffectKind::PatternOverlay).next().unwrap();
    (id, overlay)
}

#[test]
fn unticking_link_on_moved_layer_keeps_pixels_unchanged() {
    let (mut app, layer, _) = setup_test_doc(true, (0.0, 0.0));
    app.run("layer.translate", json!({"layer": layer.0, "dx": 7, "dy": 4})).unwrap();

    let original_doc = app.session.active().unwrap().doc.clone();
    let before = photocraft_compose::flatten(&original_doc);
    assert_eq!(row_letters(&before, 150, 100, 31), "WWWWWWWBBBBBBBBBBKKKKKKKKKKBBBB", "moved linked overlay row y=150 x=100..131");

    let (id, _) = open_pattern_overlay(&mut app);
    let mut h = harness(app);
    assert!(link_checked(&h), "Link with Layer starts ticked");

    click_link_checkbox(&mut h);
    assert!(!link_checked(&h), "Link with Layer is unticked");

    let (preview_doc, _) = preview_document(h.state()).expect("live preview builds after unticking Link with Layer");
    let preview_buf = photocraft_compose::flatten(&preview_doc);
    assert_same_pixels(&preview_buf, &before, "live preview after unticking Link with Layer");

    h.get_by_role_and_label(Role::Button, "OK").click();
    h.run_steps(3);
    assert!(h.state().framework.dialogs.get_by_id::<HostedLayerStyle>(id).is_none(), "OK closes the dialog");

    let committed_doc = h.state().session.active().unwrap().doc.clone();
    let (link, _phase) = pattern_overlay(&committed_doc, layer);
    assert!(!link, "Link with Layer was committed as false");

    let after = photocraft_compose::flatten(&committed_doc);
    assert_eq!(row_letters(&after, 150, 100, 31), "WWWWWWWBBBBBBBBBBKKKKKKKKKKBBBB", "row y=150 must not jump");
    assert_same_pixels(&after, &before, "committed pixels after unticking Link with Layer");

    h.state_mut().run("edit.undo", json!({})).unwrap();
    assert_eq!(pattern_overlay(&h.state().session.active().unwrap().doc, layer), (true, (0.0, 0.0)), "undo restores linked overlay");
    h.state_mut().run("edit.redo", json!({})).unwrap();
    let redone_doc = h.state().session.active().unwrap().doc.clone();
    assert!(!pattern_overlay(&redone_doc, layer).0, "redo restores unlinked overlay");
    assert_same_pixels(&photocraft_compose::flatten(&redone_doc), &before, "redo preserves pixels");
}

#[test]
fn unticking_link_with_effects_reference_sets_reference_as_phase() {
    let (mut app, layer, _) = setup_test_doc(true, (0.0, 0.0));
    // Matches PSD `fxrp` / #14960: a layer moved +7,+4 whose effects reference point is (7, 4).
    app.session
        .edit("Set fxrp", |doc, _| {
            doc.layer_mut(layer).unwrap().effects.reference = Some((7.0, 4.0));
            Ok(())
        })
        .unwrap();

    let before = photocraft_compose::flatten(&app.session.active().unwrap().doc);
    let _id = open_pattern_overlay(&mut app);
    let mut h = harness(app);

    click_link_checkbox(&mut h);
    h.get_by_role_and_label(Role::Button, "OK").click();
    h.run_steps(3);

    let after_doc = h.state().session.active().unwrap().doc.clone();
    assert_eq!(pattern_overlay(&after_doc, layer), (false, (7.0, 4.0)));
    assert_same_pixels(&photocraft_compose::flatten(&after_doc), &before, "unticking with fxrp keeps every pixel unchanged");
}

#[test]
fn unticking_and_ticking_link_on_unmoved_layer_keeps_phase_and_pixels() {
    // Sub-case A: untick and tick back within one dialog session.
    let (mut app, layer, _) = setup_test_doc(true, (5.0, 3.0));
    let before = photocraft_compose::flatten(&app.session.active().unwrap().doc);
    let (id, _) = open_pattern_overlay(&mut app);
    let mut h = harness(app);
    click_link_checkbox(&mut h);
    assert!(!link_checked(&h));
    let (mid_preview, _) = preview_document(h.state()).expect("preview builds while unticked");
    assert_same_pixels(&photocraft_compose::flatten(&mid_preview), &before, "unticked preview keeps pixels");
    click_link_checkbox(&mut h);
    assert!(link_checked(&h));
    ok(h.state_mut(), id);
    let after_a = h.state().session.active().unwrap().doc.clone();
    assert_eq!(pattern_overlay(&after_a, layer), (true, (5.0, 3.0)));
    assert_same_pixels(&photocraft_compose::flatten(&after_a), &before, "one-session untick+tick keeps pixels");

    // Sub-case B: untick -> OK, then reopen -> tick -> OK.
    let (mut app, layer, _) = setup_test_doc(true, (5.0, 3.0));
    let before = photocraft_compose::flatten(&app.session.active().unwrap().doc);
    let (id, _) = open_pattern_overlay(&mut app);
    let mut h = harness(app);
    click_link_checkbox(&mut h);
    ok(h.state_mut(), id);
    let between_doc = h.state().session.active().unwrap().doc.clone();
    assert!(!pattern_overlay(&between_doc, layer).0);
    assert_same_pixels(&photocraft_compose::flatten(&between_doc), &before, "between-sessions untick keeps pixels");

    let mut app = h.into_state();
    let (id, _) = open_pattern_overlay(&mut app);
    let mut h = harness(app);
    assert!(!link_checked(&h));
    click_link_checkbox(&mut h);
    assert!(link_checked(&h));
    ok(h.state_mut(), id);
    let after_b = h.state().session.active().unwrap().doc.clone();
    assert_eq!(pattern_overlay(&after_b, layer), (true, (5.0, 3.0)));
    assert_same_pixels(&photocraft_compose::flatten(&after_b), &before, "two-session untick+tick keeps pixels");

    // Sub-case C: unmoved layer with effects reference at (0, 0) (#14960).
    let (mut app, layer, _) = setup_test_doc(true, (5.0, 3.0));
    app.session
        .edit("Set origin fxrp", |doc, _| {
            doc.layer_mut(layer).unwrap().effects.reference = Some((0.0, 0.0));
            Ok(())
        })
        .unwrap();
    let before = photocraft_compose::flatten(&app.session.active().unwrap().doc);
    let (id, _) = open_pattern_overlay(&mut app);
    let mut h = harness(app);
    click_link_checkbox(&mut h);
    ok(h.state_mut(), id);
    assert_eq!(pattern_overlay(&h.state().session.active().unwrap().doc, layer), (false, (5.0, 3.0)));
    let mut app = h.into_state();
    let (id, _) = open_pattern_overlay(&mut app);
    let mut h = harness(app);
    click_link_checkbox(&mut h);
    ok(h.state_mut(), id);
    let after_c = h.state().session.active().unwrap().doc.clone();
    assert_eq!(pattern_overlay(&after_c, layer), (true, (5.0, 3.0)));
    assert_same_pixels(&photocraft_compose::flatten(&after_c), &before, "unmoved fxrp=(0,0) keeps phase and pixels");
}

#[test]
fn pattern_overlay_command_link_false_without_phase_keeps_todays_behaviour() {
    let (mut app, layer, pat_id) = setup_test_doc(true, (0.0, 0.0));
    app.run("layer.translate", json!({"layer": layer.0, "dx": 7, "dy": 4})).unwrap();

    app.run("layer.layerStyle.patternOverlay", json!({"pattern": pat_id, "link": false})).unwrap();
    assert_eq!(pattern_overlay(&app.session.active().unwrap().doc, layer), (false, (0.0, 0.0)));
}

#[test]
fn unticking_link_with_angle_and_scale_keeps_pixels_and_hides_internal_tag() {
    let (mut app, layer, pat_id) = setup_test_doc(true, (4.0, -2.0));
    app.run("layer.translate", json!({"layer": layer.0, "dx": 7, "dy": 4})).unwrap();
    app.run(
        "layer.layerStyle.patternOverlay",
        json!({
            "pattern": pat_id,
            "blend": "Normal",
            "opacity": 100,
            "angle": 35,
            "scale": 150,
            "link": true,
            "phaseX": 4.0,
            "phaseY": -2.0,
        }),
    )
    .unwrap();
    let original_doc = app.session.active().unwrap().doc.clone();
    let before = photocraft_compose::flatten(&original_doc);

    let (id, overlay) = open_pattern_overlay(&mut app);
    let mut dialogs: Value = json!([{"id": id.value().to_string(), "fields": {"effects": [{"id": overlay.key()}]}}]);
    describe_params(&app, &mut dialogs);
    let params_obj = dialogs[0]["fields"]["effects"][0]["params"].as_object().unwrap();
    assert!(!params_obj.contains_key("_patternLink"), "internal tracking key must not leak to describe_params");

    // Cancel leaves the document untouched.
    let mut h = harness(app);
    click_link_checkbox(&mut h);
    h.get_by_role_and_label(Role::Button, "Cancel").click();
    h.run_steps(3);
    assert_eq!(Arc::as_ptr(&h.state().session.active().unwrap().doc), Arc::as_ptr(&original_doc), "Cancel preserves document");

    // Reopen, untick, and OK with angle 35 and scale 150 keeps every pixel unchanged.
    let mut app = h.into_state();
    let (id, _) = open_pattern_overlay(&mut app);
    let mut h = harness(app);
    click_link_checkbox(&mut h);
    ok(h.state_mut(), id);
    let after_doc = h.state().session.active().unwrap().doc.clone();
    assert!(!pattern_overlay(&after_doc, layer).0);
    assert_same_pixels(&photocraft_compose::flatten(&after_doc), &before, "angled+scaled untick keeps pixels");
}
