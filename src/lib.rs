#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::TemplateApp;

mod momentary;
pub use momentary::MomentaryControllerState;

/*
fn show_centered_text(ui: &mut egui::Ui, position: egui::Pos2, text: &str) {
    // let text_size = ui.fonts().glyph_width(text);
    let centered_position = position - egui::vec2(/*text_size.x / 2.0*/ 30., 0.0);
    ui.painter().text(centered_position, egui::Align2::CENTER_CENTER, text, font_id, egui::Color32::BLACK);
}
*/