#[cfg(not(or(target_arch = "wasm32", target_arch = "thumbv6m")))]
use std::time::{Duration, Instant};
use log::warn;
use std::panic;
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};
use log::warn;
use std::panic;
#[cfg(target_arch = "thumbv6m")]
use embassy_time::{Duration, Instant};
use defmt::warn;
use core::panic;

// Avoiding a heap implies some fixed-size arrays
// This lets the compiler help enforce bounds too
pub const SWITCHES_MAX: usize = 3;
pub const OUTPUTS_MAX: usize = 4;

fn report_from_none(incoming: [bool; SWITCHES_MAX]) -> (SwitchState, Option<StateDetail>) {
    if let Some(first_idx) = incoming
        .iter()
        .enumerate()
        .find(|&(_, &x)| x)
        .map(|(index, _)| index)
    {
        if incoming[first_idx + 1..].iter().any(|&x| x) {
            // multiple switches closed at the same time, unlikely in hardware but let's be robust.
            // Note the possibility that this event could be within the double-click time of one
            // switch or the other, making it ambiguous - did they mean to press both switches, or a double-click of that
            // switch, or a multi-click following a single click? We simply assume the latter here.
            //
            panic!("MULTI not yet implemented");
        }

        return (
            SwitchState::One,
            Some(StateDetail {
                stamp: Instant::now(),
                switches: incoming,
            }),
        );
    }

    (SwitchState::None, None)
}

/// State while one button is held closed briefly.
#[derive(Clone, Copy)]
struct StateDetail {
    stamp: Instant,
    switches: [bool; SWITCHES_MAX],
}

fn report_from_one(
    incoming: [bool; SWITCHES_MAX],
    parent: &mut MomentaryController,
) -> (SwitchState, Option<StateDetail>) {
    let deets_before = parent.state_detail.unwrap();

    if let Some(first_idx) = incoming
        .iter()
        .enumerate()
        .find(|&(_, &x)| x)
        .map(|(index, _)| index)
    {
        // first handle MULTI. Else:

        // No change? Long-press, or do nothing.
        if incoming == deets_before.switches {
            // Check for long-press
            let interval = Instant::now().saturating_duration_since(deets_before.stamp);
            if parent.has_long[first_idx] && interval > parent.long_closed {
                warn!(
                    "Long-press detected on switch {}, duration was {:?}",
                    first_idx, interval
                );
                // This is a long-press.
                let output_idx: usize = parent.long[first_idx];
                parent.output[output_idx] += 1;
                if parent.output[output_idx] >= parent.output_cycles[output_idx] {
                    parent.output[output_idx] = 0;
                }
                (SwitchState::Long, None)
            } else {
                // do nothing, keep counting time.
                (SwitchState::One, parent.state_detail)
            }
        } else {
            // Switches changed, and at least one is still down.

            if incoming[first_idx + 1..].iter().any(|&x| x) {
                // multiple switches closed at the same time, unlikely in hardware but let's be robust.
                // Note the possibility that this event could be within the double-click time of one
                // switch or the other, making it ambiguous - did they mean to press both switches, or a double-click of that
                // switch, or a multi-click following a single click? We simply assume the latter here.
                //
                panic!("MULTI not yet implemented");
            } else {
                // They report one switch is closed. Was it reported closed already?
                if deets_before.switches[first_idx] {
                    panic!("Trouble: should have already caught the no-change case");
                }

                // Yikes, they released the switch but a different switch is down. Treat this like a second report
                panic!("Simultaneous release and press of two switches not yet implemented");
            }
        }
    } else {
        // They released the only switch that was down, before the long-press timer expired.
        // (Learning this requires the caller to report() repeatedly with no-change reports
        //  while switches are closed.)
        if let Some(first_idx) = deets_before
            .switches
            .iter()
            .enumerate()
            .find(|&(_, &x)| x)
            .map(|(index, _)| index)
        {
            // Check our work: be sure there wasn't a second switch down previously,
            // with both released at the same moment
            if deets_before.switches[first_idx + 1..].iter().any(|&x| x) {
                panic!("Logic problem: in state One we found 2 or more switches closed.");
            }

            // Toggle the output.
            parent.output[first_idx] += 1;
            if parent.output[first_idx] >= parent.output_cycles[first_idx] {
                parent.output[first_idx] = 0
            }
            (SwitchState::None, None)
        } else {
            panic!("Logic trouble, no-switches-before case should have been caught above");
        }
    }
}

fn report_from_long(incoming: [bool; SWITCHES_MAX]) -> (SwitchState, Option<StateDetail>) {
    if incoming.iter().find(|x| **x).into_iter().count() == 0 {
        // End the long-press state, during which no other switch changes have any effect.
        (SwitchState::None, None)
    } else {
        // Any other change, do nothing.
        (SwitchState::Long, None)
    }
}

/*
/// State when one switch has been held closed briefly (less than the long-press duration), opened before the long-press duration has passed, then closed again before the double-press duration has passed; all without another switch being closed. In this state, with that initial switch closed, other switches may then be closed subsequently (but we will not recognize double- or long-presses of those subsequent switches).
struct DoubleState {
    stamp: Instant,
    switches: [bool; SWITCHES_MAX],
}

/// State when one switch has been held closed briefly (less than the long-press duration), and during this interval another switch is closed. This state holds until the first switch is opened.
struct MultiState {
    stamp: Instant,
    switches: [bool; SWITCHES_MAX],
}
*/

#[derive(Clone, Copy, Debug)]
pub enum SwitchState {
    None,
    One,
    Long,
    /*
    Double(DoubleState),
    Multi(MultiState),
    */
}

pub struct MomentaryController {
    /// false: configuring. true: running. One way trip between them.
    started: bool,

    /// Current reported state of the system, None at start
    state: SwitchState,

    /// Info like timing needed in various states
    state_detail: Option<StateDetail>,

    /// How many input momentary-contacts?
    switches: usize,

    /// How many output channels?
    outputs: usize,

    /// Our record of the outputs themselves
    output: [u8; OUTPUTS_MAX],

    /// Output state from which first report generates first change. Moved to output at first report, invalid after that.
    output_init: [u8; OUTPUTS_MAX],

    /// Has a long-press output been established for this switch?
    has_long: [bool; SWITCHES_MAX],

    /// If a long-press output has been established for this switch, which output?
    long: [usize; SWITCHES_MAX],

    /// For each output, how many possible states? On/off: 2, low/med/high: 4, for example.
    output_cycles: [u8; OUTPUTS_MAX],

    /// Maximum open time between input closes to register a double-press event
    //    double_open: Duration,

    /// Minimum closed time to register as long-press event
    long_closed: Duration,
}

impl Default for MomentaryController {
    fn default() -> Self {
        MomentaryController {
            started: false,
            switches: 0,
            outputs: 0,
            output: [0; OUTPUTS_MAX],
            output_cycles: [0; OUTPUTS_MAX],
            output_init: [0; OUTPUTS_MAX],
            has_long: [false; SWITCHES_MAX],
            long: [0; SWITCHES_MAX],
            //            double_open: Duration::from_millis(500),
            long_closed: Duration::from_millis(1500),
            state: SwitchState::None,
            state_detail: None,
        }
    }
}

impl MomentaryController {
    pub fn new(_double_duration: Duration, long_duration: Duration) -> MomentaryController {
        MomentaryController {
            started: false,
            switches: 0,
            outputs: 0,
            output: [0; OUTPUTS_MAX],
            output_cycles: [0; OUTPUTS_MAX],
            output_init: [0; OUTPUTS_MAX],
            has_long: [false; SWITCHES_MAX],
            long: [0; SWITCHES_MAX],
            //            double_open: double_duration,
            long_closed: long_duration,
            state: SwitchState::None,
            state_detail: None,
        }
    }

    /// General case add-a-switch with all parameters.
    /// Return the index of the switch added (same as output index)
    pub fn add_switch(&mut self, output_cycle: u8, output_init: u8) -> (usize, usize) {
        if self.started {
            panic!("Don't add switches after first .report()");
        }
        let switch_idx = self.switches;
        self.switches += 1;
        let output_idx = self.outputs;
        self.output_cycles[output_idx] = output_cycle;
	self.output_init[output_idx] = output_init;
        self.outputs += 1;
        (switch_idx, output_idx)
    }

    /// Modify an already-added switch to control another output via long-press.
    pub fn augment_switch_longpress(
        &mut self,
        switch_idx: usize,
        output_cycle: u8,
	output_init: u8,
    ) -> (usize, usize) {
        if self.started {
            panic!("Don't augment switches after first .report()");
        }
        if switch_idx >= self.switches {
            panic!("Don't specify long-press on a switch that has not yet been added");
        }
        let output_idx = self.outputs;
        self.outputs += 1;
        self.output_cycles[output_idx] = output_cycle;
	self.output_init[output_idx] = output_init;
        self.has_long[switch_idx] = true;
        self.long[switch_idx] = output_idx;
        (switch_idx, output_idx)
    }

    pub fn report(&mut self, incoming: [bool; SWITCHES_MAX]) -> ([u8; OUTPUTS_MAX], SwitchState) {
        if !self.started {
            self.output = self.output_init;
            self.started = true;
        }
        (self.state, self.state_detail) = match self.state {
            SwitchState::None => report_from_none(incoming),
            SwitchState::One => report_from_one(incoming, self),
            SwitchState::Long => report_from_long(incoming), /*
                                                             SwitchState::Multi(..) => {
                                                                 panic!("not implemented")
                                                             }
                                                             SwitchState::Double(..) => {
                                                                 panic!("not implemented")
                                                             }
                                                             */
        };
        (self.output, self.state)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn none_from_none() {
        let mut c: MomentaryController = Default::default();
        c.add_switch(2);
        let ins: [bool; SWITCHES_MAX] = [false; SWITCHES_MAX];

        c.report(ins);
        assert_eq!(c.output, [0; OUTPUTS_MAX]);
        matches!(c.state, SwitchState::None);
    }

    #[test]
    fn one_from_none() {
        let mut c: MomentaryController = Default::default();
        c.add_switch(2);
        let mut ins: [bool; SWITCHES_MAX] = [false; SWITCHES_MAX];
        ins[0] = true;
        c.report(ins);
        matches!(c.state, SwitchState::One);
        assert_eq!(c.output, [0; OUTPUTS_MAX]);
    }

    fn state_one_from_scratch() -> (
        MomentaryController,
        usize,
        usize,
        [bool; SWITCHES_MAX],
        [u8; OUTPUTS_MAX],
        SwitchState,
    ) {
        let mut c: MomentaryController = Default::default();
        let (sw0, out0) = c.add_switch(2);
        let mut ins: [bool; SWITCHES_MAX] = [false; SWITCHES_MAX];
        ins[sw0] = true;
        let (output, state) = c.report(ins);
        (c, sw0, out0, ins, output, state)
    }

    #[test]
    fn validate_setup_state_one() {
        let (_c, sw0, out0, ins, output, state) = state_one_from_scratch();
        assert_eq!(sw0, 0);
        assert_eq!(out0, 0);
        matches!(state, SwitchState::One);
        assert!(ins[0]);
        assert_eq!(ins[1..], [false; SWITCHES_MAX - 1]);
        assert_eq!(output, [0; OUTPUTS_MAX]);
    }

    #[test]
    fn one_from_one() {
        let (mut c, _sw0, _out0, ins, _output, _state) = state_one_from_scratch();

        // repeat same input
        let (output, state) = c.report(ins);

        matches!(state, SwitchState::One);
        assert!(ins[0]);
        assert_eq!(ins[1..], [false; SWITCHES_MAX - 1]);
        assert_eq!(output, [0; OUTPUTS_MAX]);
    }

    #[test]
    fn none_from_one() {
        let (mut c, _sw0, out0, mut ins, _output, _state) = state_one_from_scratch();

        // open the switch
        ins[0] = false;
        let (output, state) = c.report(ins);

        matches!(state, SwitchState::None);
        assert_eq!(output[out0], 1);
        assert_eq!(c.output[1..], [0; OUTPUTS_MAX - 1]);
    }
}
