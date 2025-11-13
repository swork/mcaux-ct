use egui::Color32;
use egui::Pos2;
use egui::Rect;
use egui::Sense;
use egui::Stroke;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,

    sw0_isclosed: bool,
    sw1_isclosed: bool,
    sw2_isclosed: bool,

    // duty cycles are 0-255 to save silly calcs
    i0_duty: u8,
    i1_duty: u8,
    i2_duty: u8,
    i3r_duty: u8,
    i3g_duty: u8,
    i3b_duty: u8,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello moto world!".to_owned(),
            value: 2.7,
            sw0_isclosed: false,
            sw1_isclosed: false,
            sw2_isclosed: false,
            i0_duty: 1,
            i1_duty: 1,
            i2_duty: 1,
            i3r_duty: 1,
            i3g_duty: 1,
            i3b_duty: 1,
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("MCAux CT");

            /*
            ui.horizontal(|ui| {
                ui.label("Prove it: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

            ui.separator();
            */

            let circle_rect = Rect { min: Pos2{x: 20., y: 20.}, max: Pos2{x: 60., y: 60.}};
            let circle_center = Pos2{x:40., y: 40.};
            let circle_text = if self.sw1_isclosed {"closed"} else {"open"};
            ui.painter().circle(circle_center, 20., Color32::from_rgb(255, 255, 255), Stroke { width: 3., color: Color32::from_rgb(self.i1_duty, 0, 0)});
            ui.put(circle_rect, egui::Label::new(circle_text));
            if ui.interact(circle_rect, egui::Id::new("SW1_representation"), Sense::click()).clicked() {
                if self.sw1_isclosed {
                    self.sw1_isclosed = false;
                } else {
                    self.sw1_isclosed = true;
                }
            }

            /*
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));
            */

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });

    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
