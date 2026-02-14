#[cfg(not(target_family = "wasm"))]
extern crate std;
#[cfg(not(target_family = "wasm"))]
use std::time::Duration;
#[cfg(target_family = "wasm")]
use web_time::Duration;

use egui::Color32;
use egui::Pos2;
use egui::Rect;
use egui::Sense;
use egui::Stroke;
#[allow(unused_imports)]
use log::info;
#[cfg(target_family = "wasm")]
use wasm_logger;

use mcaux_indicators::IndicatorController;
use momentary::SwitchOutputController;

pub struct TemplateApp {
    generic_switch_controller: SwitchOutputController,
    indicators: IndicatorController, // duty cycles for all indicators
}

impl Default for TemplateApp {
    fn default() -> TemplateApp {
        TemplateApp {
            generic_switch_controller: Default::default(),
            indicators: IndicatorController::new(255, 255, 255, 255, 255, 255),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        #[cfg(target_family = "wasm")]
        wasm_logger::init(wasm_logger::Config::default());

        let mut app: TemplateApp = Default::default();

        let (sw0, out0) = app.generic_switch_controller.add_switch("usb", 2, 1);
        let (sw1, out1) = app.generic_switch_controller.add_switch("auxlight", 2, 0);
        let (sw2, out2) = app.generic_switch_controller.add_switch("gripheat", 5, 0);
        let (sw3, out3) = app
            .generic_switch_controller
            .add_switch_momentary("highbeam");
        let (sw0, out4) = app
            .generic_switch_controller
            .augment_switch_longpress_add_output(sw0, "nav", 2, 0);
        let sw2 = app
            .generic_switch_controller
            .augment_switch_longpress_max_output(sw2, out2);
        assert!(sw0 == 0 && sw1 == 1 && sw2 == 2 && sw3 == 3);
        assert!(out0 == 0 && out1 == 1 && out2 == 2 && out3 == 3 && out4 == 4);
        app
    }
}

fn screen_color_for_switch_and_duty(switch_idx: usize, duty: u8) -> Color32 {
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

        // Calc outputs and state for next cycle.
        self.generic_switch_controller.remap();

        // leds:None means no changes, but we ignore that so pull
        // LedsSItuation straight from self.indicators.duty.
        // next:None means no animation; we do quicken our pace of
        // refresh if this is set and short.
        let (_leds, next_cycle) = self
            .indicators
            .cycle(Some(self.generic_switch_controller.clone()));

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
                Rect {
                    min: Pos2 { x: 50., y: 80. },
                    max: Pos2 {
                        x: 50. + switch_radii * 2.,
                        y: 80. + switch_radii * 2.,
                    },
                },
                Rect {
                    min: Pos2 { x: 40., y: 145. },
                    max: Pos2 { x: 80., y: 185. },
                },
                Rect {
                    min: Pos2 { x: 120., y: 145. },
                    max: Pos2 { x: 160., y: 185. },
                },
            ];

            // save some typing
            let duty: [u16; 6] = [
                self.indicators.duty.usb,
                self.indicators.duty.auxlight,
                self.indicators.duty.gripheat,
                self.indicators.duty.rgb_r,
                self.indicators.duty.rgb_g,
                self.indicators.duty.rgb_b,
            ];

            let indicator_center = Pos2 { x: 142., y: 100. };
            ui.painter().circle(
                indicator_center,
                10.,
                Color32::from_rgb(
                    duty[3].try_into().unwrap(),
                    duty[4].try_into().unwrap(),
                    duty[5].try_into().unwrap(),
                ),
                Stroke {
                    width: 2.,
                    color: Color32::BLACK,
                },
            );

            for (i, item) in switch_rects.iter().enumerate() {
                let circle_center = Pos2 {
                    x: ((item.max.x - item.min.x) / 2.) + item.min.x,
                    y: ((item.max.y - item.min.y) / 2.) + item.min.y,
                };
                let circle_text = if self.generic_switch_controller.switch[i].isclosed {
                    "closed"
                } else {
                    "open"
                };

                ui.painter().circle(
                    circle_center,
                    20.,
                    Color32::from_rgb(255, 255, 255),
                    Stroke {
                        width: 4.,
                        color: screen_color_for_switch_and_duty(i, duty[i].try_into().unwrap()),
                    },
                );
                ui.put(*item, egui::Label::new(circle_text));
                let id_text = format!("SW{i}_representation");
                if ui
                    .interact(*item, egui::Id::new(id_text), Sense::click())
                    .clicked()
                {
                    self.generic_switch_controller.switch[i].isclosed =
                        !self.generic_switch_controller.switch[i].isclosed;
                }
            }

            // this can condition on needs of animations and whether a switch is closed
            let mut repaint_duration: Duration = Duration::from_millis(99);
            if let Some(to_next_animation_cycle) = next_cycle {
                repaint_duration = to_next_animation_cycle;
            }
            ctx.request_repaint_after(repaint_duration);

            // high beam switch
            ui.heading("High beam switch");

            // Display the checkbox and bind its state to a boolean
            ui.checkbox(
                &mut self.generic_switch_controller.switch[3].isclosed,
                "High if checked",
            );

            // Display a message based on the checkbox's state
            if self.generic_switch_controller.switch[3].isclosed {
                ui.label("High");
            } else {
                ui.label("Low");
            }

            // Debug info
            ui.separator();
            ui.label(format!(
                "switches state: {:?}",
                self.generic_switch_controller.switches_state
            ));
            ui.label(format!(
                "switches: {:?}",
                self.generic_switch_controller
                    .switch
                    .iter()
                    .map(|x| if x.isclosed { 1 } else { 0 })
                    .collect::<Vec<u8>>()
            ));
            ui.label(format!(
                "outputs: {:?}",
                self.generic_switch_controller
                    .output
                    .iter()
                    .map(|x| x.value)
                    .collect::<Vec<u8>>()
            ));
            ui.separator();

            for i in 0..3 {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "sw{}: {}",
                        i, self.generic_switch_controller.switch[i].isclosed
                    ));
                    ui.label(format!(
                        "out{}: {:?}",
                        i, self.generic_switch_controller.output[i].value
                    ));
                });
            }
            ui.horizontal(|ui| {
                ui.label(format!(
                    "     out3: {:?}  out4: {:?}",
                    self.generic_switch_controller.output[3].value,
                    self.generic_switch_controller.output[4].value,
                ));
            });

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
