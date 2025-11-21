// mcaux-indicators/src/lib.rs

/// Provide LED levels and level-change animations for mcaux.
#[derive(Default)]
pub struct IndicatorController {
    //   ins: [bool; 3],
    //    outs: [u8; 4],
    //    animations: [Option<Animation>; 5],
    //    last_frame: Instant,
}

/// Provide LED levels and level-change animations for mcaux.
/// Not much abstraction here, maybe we make it more configurable
/// and less hard-coded later.
impl IndicatorController {
    pub fn get_duty_cycles(&mut self, _new_ins: [bool; 3], new_outs: [u8; 4]) -> [u8; 6] {
        let mut duties: [u8; 6] = [0; 6];

        /*
        let now = Instant::now();
        for anim in self.animations.iter_mut() {
            if let Some(animation) = anim {
                animation.run(now.duration_since(self.last_frame));
            }
        }
        */

        duties[0] = if new_outs[0] != 0 { 255 } else { 0 };
        duties[1] = if new_outs[1] != 0 { 255 } else { 0 };
        duties[2] = if new_outs[2] != 0 { 255 } else { 0 };
        let rgb = color_for_heat_level(new_outs[2]);
        duties[3] = rgb[0];
        duties[4] = rgb[1];
        duties[5] = rgb[2];
        duties
    }
}

fn color_for_heat_level(level: u8) -> [u8; 3] {
    match level {
        0 => [0, 0, 0],       // off
        1 => [170, 50, 50],   // dull red
        2 => [255, 128, 0],   // Orange
        3 => [255, 255, 0],   // Bright yellow
        _ => [255, 255, 255], // White
    }
}

/*
struct Animation {}

impl Animation {
    pub fn run(&mut self, delta: Duration) {
        // Animation logic here
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
*/
