#![no_std]
#[cfg(not(any(target_os = "none", target_family = "wasm")))]
extern crate std;
#[cfg(target_os = "none")]
use embassy_time::Duration;
#[cfg(not(any(target_os = "none", target_family = "wasm")))]
use std::time::Duration;
#[cfg(target_family = "wasm")]
use web_time::Duration;

use momentary::{SwitchOutputController, SwitchesState};

/// A representation of indicator output specification: how indicators
/// should present at this instant
#[allow(unused)]
#[derive(Copy, Clone, Default)]
pub struct LedsSituation {
    pub usb: u16,
    pub auxlight: u16,
    pub gripheat: u16,
    pub rgb_r: u16,
    pub rgb_g: u16,
    pub rgb_b: u16,
}

#[allow(unused)]
#[derive(Copy, Clone, Default)]
pub struct IndicatorController {
    duty_max: LedsSituation,
    pub duty: LedsSituation,
    pub sm: SwitchesState,
}

impl IndicatorController {
    pub fn new(
        usb_max_duty: u16,
        auxlight_max_duty: u16,
        gripheat_max_duty: u16,
        rgb_r_max_duty: u16,
        rgb_g_max_duty: u16,
        rgb_b_max_duty: u16,
    ) -> IndicatorController {
        IndicatorController {
            duty_max: LedsSituation {
                usb: usb_max_duty,
                auxlight: auxlight_max_duty,
                gripheat: gripheat_max_duty,
                rgb_r: rgb_r_max_duty,
                rgb_g: rgb_g_max_duty,
                rgb_b: rgb_b_max_duty,
            },
            ..Default::default()
        }
    }

    /// Compute the state of indicators at the present instant, and
    /// repeatedly for a sequence of periods into the future. We
    /// should be called back with parameter None after Duration if
    /// there have been no changes to the ApplicationSituation, or
    /// earlier with a new situation. (non-None is fine too, just does
    /// more calculation.)
    #[allow(unused)]
    pub fn cycle(
        &mut self,
        model: Option<SwitchOutputController>,
    ) -> (Option<LedsSituation>, Option<Duration>) {
        let model = model.unwrap();

        self.duty.usb = if model.output[model.oidx("usb")].value != 0 {
            self.duty_max.usb
        } else {
            0
        };
        self.duty.auxlight = if model.output[model.oidx("auxlight")].value != 0 {
            self.duty_max.auxlight
        } else {
            0
        };
        self.duty.gripheat = if model.output[model.oidx("gripheat")].value != 0 {
            self.duty_max.gripheat
        } else {
            0
        };

        // Fix these up from 8-bit RGB numbers
        let rgb = color_for_heat_level(model.output[model.oidx("gripheat")].value);
        let numerator: u16 = rgb[0] as u16 * self.duty_max.rgb_r / 256;
        self.duty.rgb_r = numerator.try_into().unwrap();
        let numerator: u16 = rgb[1] as u16 * self.duty_max.rgb_g / 256;
        self.duty.rgb_g = numerator.try_into().unwrap();
        let numerator: u16 = rgb[2] as u16 * self.duty_max.rgb_b / 256;
        self.duty.rgb_b = numerator.try_into().unwrap();

        (Some(self.duty.clone()), None)
    }
}

/// Give 8-bit RGB values for various gripheat level indicator colors.
pub fn color_for_heat_level(level: u8) -> [u8; 3] {
    match level {
        0 => [0, 0, 0],       // off
        1 => [170, 50, 50],   // dull red
        2 => [255, 128, 0],   // Orange
        3 => [255, 255, 0],   // Bright yellow
        _ => [255, 255, 255], // White
    }
}
