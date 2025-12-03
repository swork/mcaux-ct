#![no_std]
#[cfg(feature = "not-no-std")]
extern crate std;
#[cfg(feature = "not-no-std")]
use std::time::{Duration, Instant};
#[cfg(feature = "wasm")]
use web_time::{Duration, Instant};
#[cfg(feature = "embassy")]
use embassy_time::{Duration, Instant};
#[cfg(feature = "not-no-std")]
use std::panic;
#[cfg(not(feature = "not-no-std"))]
use core::panic;

// Avoiding a heap implies some fixed-size arrays
// This lets the compiler help enforce bounds too
pub const SWITCHES_MAX: usize = 3;
pub const OUTPUTS_MAX: usize = 4;

fn report_from_none(incoming: [bool; SWITCHES_MAX]) -> (SwitchesState, Option<StateDetail>) {
    if let Some(first_idx) = incoming
        .iter()
        .enumerate()
        .find(|&(_, &x)| x)
        .map(|(index, _)| index)
    {
        if incoming[first_idx + 1..].iter().any(|&x| x) {
            // multiple switches closed at the same time, relative to report() calls.
            // We don't know which was pressed first, and don't have an action for this case.
	    // Caller should be less generous with debounce waits?
	    (SwitchesState::Jammed, None,)
        } else {
            (
		SwitchesState::One,
		Some(StateDetail {
                    stamp: Instant::now(),
                    switches: incoming,
		}),
            )
	}
    } else {
	(SwitchesState::None, None)
    }
}

/// State while one button is held closed briefly.
#[derive(Clone, Copy)]
struct StateDetail {
    stamp: Instant,
    switches: [bool; SWITCHES_MAX],
}

fn report_from_jammed(
    incoming: [bool; SWITCHES_MAX],
) -> (SwitchesState, Option<StateDetail>) {
    if incoming.iter().any(|&x| x) {
	// A button is down so we're still jammed.
	(SwitchesState::Jammed, None,)
    } else {
	// No button is down. Back to None.
	(SwitchesState::None, None,)
    }
}

fn report_from_one(
    incoming: [bool; SWITCHES_MAX],
    parent: &mut MomentaryController,
) -> (SwitchesState, Option<StateDetail>) {
    let deets_before = parent.state_detail.unwrap();

    if let Some(first_idx) = incoming
        .iter()
        .enumerate()
        .find(|&(_, &x)| x)
        .map(|(index, _)| index)
    {
        // No change? Long-press, or do nothing.
        if incoming == deets_before.switches {
            // Check for long-press
            let interval = Instant::now().saturating_duration_since(deets_before.stamp);
            if parent.has_long[first_idx] && interval > parent.long_closed {
                // This is a long-press. It can mean cycle an output, or max an output.
                let output_idx: usize = parent.long[first_idx];
		if parent.long_specifies_max[first_idx] {
		    parent.output[output_idx] = parent.output_cycles[output_idx] - 1;
		} else {
                    parent.output[output_idx] += 1;
                    if parent.output[output_idx] >= parent.output_cycles[output_idx] {
			parent.output[output_idx] = 0;
		    }
                }
                (SwitchesState::Long, None)
            } else {
                // do nothing, keep counting time.
                (SwitchesState::One, parent.state_detail)
            }
        } else {
            // Switches changed, and at least one is still down.

            if incoming[first_idx + 1..].iter().any(|&x| x) {
                // multiple switches are now closed. We're jammed until all released (no MULTI yet)
		(SwitchesState::Jammed, None,)
            } else {
                // They report one switch is closed. Was it reported closed already?
                if deets_before.switches[first_idx] {
                    panic!("Trouble: should have already caught the no-change case");
                }

                // Yikes, they released the switch but a different switch is down. Treat this like a second button press.
		// First toggle the output of the switch that just opened
		parent.output[first_idx] += 1;
		if parent.output[first_idx] >= parent.output_cycles[first_idx] {
                    parent.output[first_idx] = 0
		}
		// Then make a new start with the new button down.
		(
		    SwitchesState::One,
		    Some(StateDetail {
			stamp: Instant::now(),
			switches: incoming,
		    }),
		)
            }
        }
    } else {
	// Which switch was closed previously? It's now open, prior to Long, so cycle its output.
        if let Some(first_idx) = deets_before
            .switches
            .iter()
            .enumerate()
            .find(|&(_, &x)| x)
            .map(|(index, _)| index)
        {
            // They released the only switch that was down, before the long-press timer expired.
            // (Learning this requires the caller to report() repeatedly with no-change reports
            //  while switches are closed.)
	    
            // Check our work: be sure there wasn't a second switch down previously,
            // with both released at the same moment. This is just paranoia.
            if deets_before.switches[first_idx + 1..].iter().any(|&x| x) {
                panic!("Logic problem: in state One we found 2 or more switches closed.");
            }

            // Toggle the output.
            parent.output[first_idx] += 1;
            if parent.output[first_idx] >= parent.output_cycles[first_idx] {
                parent.output[first_idx] = 0
            }
            (SwitchesState::None, None)
        } else {
	    // more paranoia
            panic!("Logic trouble, we were in state One but with no switches closed previously.");
        }
    }
}

fn report_from_long(incoming: [bool; SWITCHES_MAX]) -> (SwitchesState, Option<StateDetail>) {
    if incoming.iter().find(|x| **x).into_iter().count() == 0 {
        // End the long-press state, during which no other switch changes have any effect.
        (SwitchesState::None, None)
    } else {
        // Any other change, do nothing.
        (SwitchesState::Long, None)
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
pub enum SwitchesState {
    None,
    One,
    Long,
    Jammed,  // two switches went down at the same time. Nothing to do, do nothing.
    /*
    Double(DoubleState),
    Multi(MultiState),
    */
}

pub struct MomentaryController {
    /// false: configuring. true: running. One way trip between them.
    started: bool,

    /// Current reported state of the system, None at start
    switches_state: SwitchesState,

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

    /// If a long-press output has been established for an output, bump (false) or max (true)?
    /// Notice for a 2-cycle switch this amounts to unconditional On, and for any
    /// switch long followed by short is an unconditional Off.
    long_specifies_max: [bool; SWITCHES_MAX],

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
	    long_specifies_max: [false; SWITCHES_MAX],
            //            double_open: Duration::from_millis(500),
            long_closed: Duration::from_millis(900),
            switches_state: SwitchesState::None,
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
	    long_specifies_max: [false; SWITCHES_MAX],
            //            double_open: double_duration,
            long_closed: long_duration,
            switches_state: SwitchesState::None,
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
    pub fn augment_switch_longpress_add_output(
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

    /// Modify an already-added switch to jump an output to its numerically-highest level on long-press.
    pub fn augment_switch_longpress_max_output(
	&mut self,
	switch_idx: usize,
	output_idx: usize,
    ) -> (usize, usize) {
	if self.started {
	    panic!("Don't augment switches after first .report()");
	}
	if switch_idx >= self.switches {
	    panic!("Don't specify long-press for a switch that has not yet been added");
	}
	self.has_long[switch_idx] = true;
	self.long[switch_idx] = output_idx;
	self.long_specifies_max[switch_idx] = true;
	(switch_idx, output_idx)
    }

    pub fn report(&mut self, incoming: [bool; SWITCHES_MAX]) -> ([u8; OUTPUTS_MAX], SwitchesState) {
        if !self.started {
            self.output = self.output_init;
            self.started = true;
        }
        (self.switches_state, self.state_detail) = match self.switches_state {
            SwitchesState::None => report_from_none(incoming),
            SwitchesState::One => report_from_one(incoming, self),
            SwitchesState::Long => report_from_long(incoming),
	    SwitchesState::Jammed => report_from_jammed(incoming),
	    /*
                                                             SwitchesState::Multi(..) => {
                                                                 panic!("not implemented")
                                                             }
                                                             SwitchesState::Double(..) => {
                                                                 panic!("not implemented")
                                                             }
                                                             */
        };
        (self.output, self.switches_state)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn none_from_none() {
        let mut c: MomentaryController = Default::default();
        c.add_switch(2, 0);
        let ins: [bool; SWITCHES_MAX] = [false; SWITCHES_MAX];

        c.report(ins);
        assert_eq!(c.output, [0; OUTPUTS_MAX]);
        matches!(c.switches_state, SwitchesState::None);
    }

    #[test]
    fn one_from_none() {
        let mut c: MomentaryController = Default::default();
        let (sw0_idx, out0_idx) = c.add_switch(2, 0);
	assert_eq!(sw0_idx, 0);
	assert_eq!(out0_idx, 0);
        let mut ins: [bool; SWITCHES_MAX] = [false; SWITCHES_MAX];
        ins[0] = true;
        c.report(ins);
        matches!(c.switches_state, SwitchesState::One);
        assert_eq!(c.output, [0; OUTPUTS_MAX]);
    }

    fn state_one_from_scratch() -> (
        MomentaryController,
        [bool; SWITCHES_MAX],
        [u8; OUTPUTS_MAX],
        SwitchesState,
    ) {
        let mut c: MomentaryController = Default::default();
        let (sw0_idx, _out0_idx) = c.add_switch(2, 0);
        let mut ins: [bool; SWITCHES_MAX] = [false; SWITCHES_MAX];
        ins[sw0_idx] = true; // idx is zero, tested in one_from_none above. out0_idx too.
        let (output, state) = c.report(ins);
        (c, ins, output, state)
    }

    #[test]
    fn validate_setup_state_one() {
        let (_c, ins, output, state) = state_one_from_scratch();
        matches!(state, SwitchesState::One);
        assert!(ins[0]);
        assert_eq!(ins[1..], [false; SWITCHES_MAX - 1]);
        assert_eq!(output, [0; OUTPUTS_MAX]);
    }

    #[test]
    fn one_from_one() {
        let (mut c, ins, _output, _state) = state_one_from_scratch();

        // repeat same input
        let (output, state) = c.report(ins);

        matches!(state, SwitchesState::One);
        assert!(ins[0]);
        assert_eq!(ins[1..], [false; SWITCHES_MAX - 1]);
        assert_eq!(output, [0; OUTPUTS_MAX]);
    }

    #[test]
    fn none_from_one() {
        let (mut c, mut ins, _output, _state) = state_one_from_scratch();

        // open the switch
        ins[0] = false;
        let (output, state) = c.report(ins);

        matches!(state, SwitchesState::None);
        assert_eq!(output[0], 1);
        assert_eq!(output[1..], [0; OUTPUTS_MAX - 1]);
    }

    #[test]
    fn jammed_from_one() {
        let (mut c, mut ins, _output, _state) = state_one_from_scratch();

        // close another switch
        ins[1] = true;
        let (output, state) = c.report(ins);

        matches!(state, SwitchesState::Jammed);
	// no change to output state for original switch, since it wasn't released
        assert_eq!(output[0], 0);
	// no change to any other output
        assert_eq!(output[1..], [0; OUTPUTS_MAX - 1]);
    }

    #[test]
    fn still_jammed_on_changes_and_unjammed_when_all_released() {
        let (mut c, mut ins, output, _state) = state_one_from_scratch();
        assert_eq!(output[..], [0; OUTPUTS_MAX]);

        // close another switch to get jammed
        ins[1] = true;
        let (output, state) = c.report(ins);
        matches!(state, SwitchesState::Jammed);
        assert_eq!(output[..], [0; OUTPUTS_MAX]);

	// open that other switch
        ins[1] = false;
        let (output, state) = c.report(ins);
        matches!(state, SwitchesState::Jammed);
        assert_eq!(output[..], [0; OUTPUTS_MAX]);

	// close a different switch
	ins[2] = true;
        let (output, state) = c.report(ins);
        matches!(state, SwitchesState::Jammed);
        assert_eq!(output[..], [0; OUTPUTS_MAX]);

	// open the original switch, stay jammed
	ins[0] = false;
        let (output, state) = c.report(ins);
        matches!(state, SwitchesState::Jammed);
        assert_eq!(output[..], [0; OUTPUTS_MAX]);

	// One switch is still closed. Open it, get unjammed
	ins[2] = false;
        let (output, state) = c.report(ins);
        matches!(state, SwitchesState::None);

	// No outputs were turned on through that entire process.
        assert_eq!(output[..], [0; OUTPUTS_MAX]);
    }
}
