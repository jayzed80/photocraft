//! Verifies #15533: Help stays visible after Window when the bar has room, and folds into "…" only when narrow.

use craft::host::target::{Backend, Os, Runtime, WindowKind};
use craft::window::desktop::Desktop;
use craft::window::policy::{TitleBarPref, decide};
use egui::vec2;
use egui_kittest::{Harness, kittest::Queryable};
use photocraft_ui_egui::{PhotocraftApp, Services};

fn integrated_app_at(width: f32, height: f32, pixels_per_point: f32) -> Harness<'static, PhotocraftApp> {
    let mut h = Harness::builder().with_size(vec2(width, height)).with_pixels_per_point(pixels_per_point).with_max_steps(64).build_eframe(move |cc| {
        let desktop = Desktop::detect(Backend::X11);
        let decision = decide(Os::Windows, Runtime::Native, Backend::X11, &desktop, WindowKind::Main, TitleBarPref::Integrated, None);
        PhotocraftApp::setup_window(&cc.egui_ctx, Default::default(), decision);
        let mut session = photocraft_engine::Session::new();
        session.edit_prefs(|p| {
            p.general.auto_show_home_screen = false;
            p.interface.language = "en".into();
        });
        PhotocraftApp::with_context(cc.egui_ctx.clone(), session, Services::default())
    });
    h.run_steps(4);
    h
}

#[test]
fn help_menu_title_is_drawn_in_wide_window() {
    for (width, height, ppp) in [(1200.0, 900.0, 1.0), (1600.0, 1000.0, 1.25), (1900.0, 900.0, 1.25)] {
        let h = integrated_app_at(width, height, ppp);
        let help = h
            .query_by_label("Help")
            .unwrap_or_else(|| panic!("Help menu missing at {width}x{height}@{ppp} (overflow '…' present: {})", h.query_by_label("…").is_some()));
        assert!(h.query_by_label("…").is_none(), "unexpected overflow at {width}x{height}@{ppp}");
        assert!(h.query_all_by_label("PhotoCraft").next().is_some(), "title missing at {width}x{height}@{ppp}");
        let window_item = h.query_all_by_label("Window").find(|n| n.rect().height() > 0.0 && n.rect().width() < 120.0).expect("Window menu item");
        assert!(help.rect().min.x >= window_item.rect().max.x - 1.0, "Help should sit after Window");
    }
}

#[test]
fn help_menu_folds_into_overflow_when_narrow() {
    let mut h = integrated_app_at(500.0, 600.0, 1.0);
    assert!(h.query_by_label("Help").is_none(), "Help should fold when the bar is narrow");
    let at = h.get_by_label("…").rect().center();
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(2);
    h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE });
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE });
    h.run_steps(4);
    assert!(
        h.query_all_by_role(egui::accesskit::Role::MenuItem).any(|n| {
            use egui_kittest::kittest::NodeT;
            n.accesskit_node().label().as_deref() == Some("Help")
        }),
        "Help should be reachable through the overflow menu"
    );
}
