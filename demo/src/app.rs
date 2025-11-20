#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(target_arch = "wasm32")]
use web_time::Duration;

use egui::Color32;
use egui::Pos2;
use egui::Rect;
use egui::Sense;
use egui::Stroke;
use log::info;

use momentary::MomentaryController;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,

    #[serde(skip)]
    switch_isclosed: [bool; 16],
    #[serde(skip)]
    output: [u8; 16],
    #[serde(skip)]
    switch_state_name: String,

    // duty cycles are 0-255 to save silly calcs
    #[serde(skip)]
    indicator_duty: [u8; 3],
    #[serde(skip)]
    rgb_duty: [u8; 3],

    #[serde(skip)] // This how you opt-out of serialization of a field
    controller: MomentaryController,

}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello moto world!".to_owned(),
            value: 2.7,
            switch_isclosed: [false; 16],
            switch_state_name: String::new(),
            output: [0; 16],
            indicator_duty: [128,128,128],
            rgb_duty: [90,100,110],
            controller: Default::default(),
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
        let mut app: TemplateApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        let (sw0_idx, _out0_idx) = app.controller.add_switch(2);
        let (sw1_idx, _out1_idx) = app.controller.add_switch(2);
        let (sw2_idx, _out2_idx) = app.controller.add_switch(5);
        let (_sw_l_idx,_out_l_idx) = app.controller.augment_switch_longpress(sw0_idx, 2);
        assert!(sw0_idx == 0 && sw1_idx == 1 && sw2_idx == 2);

        app
    }
}

fn color_for_switch_and_duty(switch_idx: usize, duty: u8) -> Color32 {
    match switch_idx {
        0 => Color32::from_rgb(duty, 0, 0),
        1 => Color32::from_rgb(0, 0, duty),
        2 => Color32::from_rgb(duty, duty, duty),
        _ => Color32::from_rgb(0, 0, 0),
    }
}

impl eframe::App for TemplateApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
//        eframe::set_value(storage, eframe::APP_KEY, self);
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

            // TODO rework to _centers
            let switch_radii: f32 = 20.;
            let switch_rects: [Rect; 3] = [
                Rect { min: Pos2{x: 50., y: 80.}, max: Pos2{x: 50. + switch_radii*2., y: 80. + switch_radii*2.}},
                Rect { min: Pos2{x: 40., y: 145.}, max: Pos2{x: 80., y: 185.}},
                Rect { min: Pos2{x: 120., y: 145.}, max: Pos2{x: 160., y: 185.}},
            ];

            let indicator_center = Pos2{x:142., y:100.};
            ui.painter().circle(
                indicator_center,
                10., 
                Color32::from_rgb(self.rgb_duty[0], self.rgb_duty[1], self.rgb_duty[2]), 
                Stroke { width: 2., color: Color32::BLACK,});
            info!("indicator rgb: {},{},{}", self.rgb_duty[0], self.rgb_duty[1], self.rgb_duty[2]);

            for i in 0..switch_rects.len() {
                let circle_center = Pos2{
                    x: ((switch_rects[i].max.x-switch_rects[i].min.x)/2.)+switch_rects[i].min.x,
                    y: ((switch_rects[i].max.y-switch_rects[i].min.y)/2.)+switch_rects[i].min.y,
                };                
                let circle_text = if self.switch_isclosed[i] {"closed"} else {"open"};
                ui.painter().circle(circle_center, 20., Color32::from_rgb(255, 255, 255), Stroke { width: 4., color: color_for_switch_and_duty(i, self.indicator_duty[i])});
                ui.put(switch_rects[i], egui::Label::new(circle_text));
                let id_text = format!("SW{i}_representation");
                if ui.interact(switch_rects[i], egui::Id::new(id_text), Sense::click()).clicked() {
                    if self.switch_isclosed[i] {
                        self.switch_isclosed[i] = false;
                    } else {
                        self.switch_isclosed[i] = true;
                    }
                }
            }
            (self.output, self.switch_state_name) = self.controller.report(self.switch_isclosed);
            ctx.request_repaint_after(Duration::from_millis(99)); // roughly 10fps

            // Debug info
            ui.separator();
            ui.label(format!("switch controller state: {}", self.switch_state_name));
            ui.label(format!("switches: {:?}",
                self.switch_isclosed[0..=2].iter().map(|x| if *x {1} else {0}).collect::<Vec<u8>>()));
            ui.label(format!("outputs: {:?}", &self.output[0..=3]));
            ui.separator();

            for i in 0..3 {
                ui.horizontal(|ui| {
                    ui.label(format!("sw{}: {}", i, self.switch_isclosed[i]));
                    ui.label(format!("out{}: {}", i, self.output[i]));
                });
            }

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
