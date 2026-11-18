use std::cell::RefCell;

use craft::behavior::{Arrangement, IconId};
use craft::ui::buttons::IconButton;
use craft::ui::contract::{BodyCx, Props as _, Show as _, ShowContainer as _, TooltipSpec};
use craft::ui::display::{Elide, Text};
use craft::ui::layout::{Placed, arrange};
use craft::ui::runtime::windows::WindowHost;
use craft::ui::shell::{Frame, Title, TitleBar, slots};

use super::{PhotocraftApp, brand};

// ----------------------------------------------------------------------------- title bar

/// The app's top bar, in the shell's title slot: brand mark, menus, the document title and the workspace controls.
/// The framework `TitleBar` follows the window's frame: with the app's own frame (Windows and Linux) it is also the
/// window's title bar and the caption buttons take its ends; with the system's frame it shows the content and no title.
pub fn title_bar(app: &mut PhotocraftApp, ui: &mut egui::Ui) {
    let title = title_text(app);
    // The bar runs its body and its end region one after the other, so the two borrows of the app never overlap.
    // We use `TitleBar::show` rather than `show_titled` so the title's gap arrangement does not claim the remaining
    // width during the row's first measuring sweep, which otherwise forces `MenuBar` down to its minimum sum (#15533).
    let app = RefCell::new(app);
    let out = TitleBar::new()
        .label(title.clone())
        .trailing(|ui, _| {
            let mut guard = app.borrow_mut();
            let app = &mut **guard;
            crate::prefs_ui::appearance_button(app, ui);
            let tip = crate::shortcuts::tip_label(app, "Search commands", "edit.search");
            let search = IconButton::new(IconId::new("search")).label(tip.clone()).tooltip(TooltipSpec::new(tip)).selected(app.ui.palette.open).show(ui);
            if search.clicked() {
                app.ui.palette.open = !app.ui.palette.open;
            }
            crate::workspace_ui::switcher(app, ui);
        })
        .show(ui, |ui, cx| {
            let mut guard = app.borrow_mut();
            let app = &mut **guard;
            let brand_rect = brand::mark().show(ui).rect;
            // With the macOS menu bar the menus are at the top of the screen instead.
            let titles = (app.services.native_menu.is_none()).then(|| crate::menus::menu_bar_rect(app, ui));
            if !matches!(WindowHost::of(ui.ctx()).frame(), Frame::Native) && titles.is_none_or(|r| r.width() > 0.0) {
                show_title_in_gap(ui, cx, &title, brand_rect, titles);
            }
        })
        .output;
    // The bar never closes the window: File › Exit asks about unsaved work first.
    if out.close_requested {
        let mut guard = app.borrow_mut();
        let _ = crate::menus::invoke(&mut guard, ui.ctx(), "file.exit", serde_json::json!({}));
    }
}

/// Draws `title` centred on the window's centre line in the free gap after the brand mark and menu titles.
fn show_title_in_gap(ui: &mut egui::Ui, cx: &BodyCx<'_>, title: &str, brand_rect: egui::Rect, titles: Option<egui::Rect>) {
    let body = cx.rect;
    let rtl = cx.env.direction().is_rtl();
    let margin = cx.gap().unwrap_or_default();
    let gap_rect = if rtl {
        let leading_start = titles.filter(|r| r.width() > 0.0).map_or(brand_rect.left(), |r| r.left());
        egui::Rect::from_min_max(body.left_top(), egui::pos2(leading_start, body.bottom()))
    } else {
        let leading_end = titles.filter(|r| r.width() > 0.0).map_or(brand_rect.right(), |r| r.right());
        egui::Rect::from_min_max(egui::pos2(leading_end, body.top()), body.right_bottom())
    };
    if gap_rect.width() <= 2.0 * margin {
        return;
    }
    let middle = cx
        .shell()
        .and_then(|shell| shell.regions().slots.iter().find(|(key, _)| *key == slots::TITLE).map(|(_, rect)| rect.center().x))
        .unwrap_or_else(|| body.center().x);
    let centre = if rtl { body.right() - middle } else { middle - body.left() };
    let gap = TitleGap { body_width: body.width(), centre, margin, rtl };
    ui.scope_builder(egui::UiBuilder::new().max_rect(gap_rect), |ui| {
        arrange(ui, &gap, &mut [Placed::new("title", Text::new(title.to_owned()).elide(Elide::End))]);
    });
}

struct TitleGap {
    body_width: f32,
    centre: f32,
    margin: f32,
    rtl: bool,
}

impl Arrangement for TitleGap {
    fn place(&self, room: egui::Rect, sizes: &[egui::Vec2], out: &mut Vec<egui::Rect>) -> egui::Vec2 {
        let size = sizes.first().copied().unwrap_or(egui::Vec2::ZERO);
        let free = room.width() - 2.0 * self.margin;
        let room_start = self.body_width - room.width();
        let (from, width) = Title::span(self.centre - room_start - self.margin, free, size.x);
        let x = if self.rtl { room.right() - self.margin - from - width } else { room.min.x + self.margin + from };
        out.push(egui::Rect::from_min_size(egui::pos2(x, room.min.y + (room.height() - size.y) / 2.0), egui::vec2(width, size.y)));
        egui::vec2(room.width(), size.y)
    }
}

/// The title the bar shows: the active document's name with a dot when it has unsaved changes, or the app name with
/// no document open. With Preferences › Interface › Show File Name In Title off, the file name drops out and the app
/// name (and the dot) remains, since the tabs already name the file (#5037).
pub(crate) fn title_text(app: &PhotocraftApp) -> String {
    let show_name = app.session.prefs().interface.show_file_name_in_title;
    app.session
        .active()
        .map(|d| format!("{}{}", if show_name { d.doc.name.as_str() } else { "PhotoCraft" }, if d.is_dirty() { "  •" } else { "" }))
        .unwrap_or_else(|| "PhotoCraft".into())
}

/// The OS window title: the active document's name suffixed with the app name (a `*` prefix
/// marks unsaved changes, like the `•` in the app-drawn title), or just the app name with no
/// document open. With Preferences › Interface › Show File Name In Title off, the file name
/// drops out and only the app name (and the `*`) remains (#5037).
pub fn window_title(app: &PhotocraftApp) -> String {
    if !app.session.prefs().interface.show_file_name_in_title {
        return format!("{}PhotoCraft", if app.session.active().is_some_and(|d| d.is_dirty()) { "*" } else { "" });
    }
    app.session.active().map(|d| format!("{}{} \u{2014} PhotoCraft", if d.is_dirty() { "*" } else { "" }, d.doc.name)).unwrap_or_else(|| "PhotoCraft".into())
}

#[cfg(test)]
mod tests;
