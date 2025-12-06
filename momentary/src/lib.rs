#![no_std]
#[cfg(not(any(target_os = "none", target_family = "wasm")))]
extern crate std;
#[cfg(any(target_os = "none", target_family = "wasm"))]
use core::panic;
#[cfg(target_os = "none")]
use embassy_time::{Duration, Instant};
#[cfg(not(any(target_os = "none", target_family = "wasm")))]
use std::panic;
#[cfg(not(any(target_os = "none", target_family = "wasm")))]
use std::time::{Duration, Instant};
#[cfg(target_family = "wasm")]
use web_time::{Duration, Instant};

use heapless::LinearMap;

// Avoiding a heap implies some fixed-size arrays
pub const SWITCHES_MAX: usize = 8;
pub const OUTPUTS_MAX: usize = 8;

const EMPTY_NAME: &str = "";

/// Info about the state at a previous state transition: when it
/// happened, and what the switches looked like
#[derive(Clone, Copy)]
struct StateDetail {
    stamp: Instant,
    switch: [AbstractInput; SWITCHES_MAX],
}

#[derive(Clone, Copy, Debug, Default)]
pub enum SwitchesState {
    #[default]
    None,
    One,
    Long,
    Jammed, // two switches went down at the same time. Nothing to do, do nothing.
            /*
            Double(DoubleState),
            Multi(MultiState),
            */
}

/// Details regarding one input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AbstractInput {
    /// Backreference (for code here), and an opaque ID (for callers)
    pub idx: usize,

    /// Name for app lookup
    name: &'static str,

    /// Fn here instead of when-last-checked bool? Caller modifies.
    pub isclosed: bool,

    /// Which output is associated with this switch?
    output_idx: usize,

    /// Has a separate long-press output been associated with this switch?
    has_long_output: bool,

    /// If has_long_output, which?
    long_output_idx: usize,

    /// If has_long_output, bump (false) or max (true)?
    /// Notice for a 2-cycle switch this amounts to unconditional On, and for any
    /// switch long followed by short is an unconditional Off.
    long_specifies_max: bool,

    /// Is this to be treated as a momentary switch, whose 2-state
    /// output follows the input state?  Note that we remove the input
    /// state from the array of inputs during output/state
    /// calculation, so our state machine and calling code doesn't
    /// have to special-case MULTI.
    momentary: bool,
}

impl Default for AbstractInput {
    fn default() -> AbstractInput {
        AbstractInput {
            idx: 0,
            name: &EMPTY_NAME,
            isclosed: false,
            output_idx: 0,
            has_long_output: false,
            long_output_idx: 0,
            long_specifies_max: false,
            momentary: false,
        }
    }
}

impl AbstractInput {
    pub fn new(isclosed: bool, model: AbstractInput) -> AbstractInput {
        AbstractInput { isclosed, ..model }
    }
}

/// Abstract details regarding one output.
#[derive(Clone, Copy, Debug, Default)]
pub struct AbstractOutput {
    /// Where in our array of outputs does this item live?
    pub idx: usize,

    /// Name for lookup
    #[allow(unused)]
    name: &'static str,

    /// Current output state, 0..values. Caller does NOT modify.
    pub value: u8,

    /// Count of output states. Caller does NOT modify.
    pub values: u8,
}

#[derive(Clone)]
pub struct SwitchOutputController {
    /// Current reported state of the system, None at start
    pub switches_state: SwitchesState,

    /// Info like timing needed in various states
    state_detail: Option<StateDetail>,

    /// How many input contacts?
    switches: usize,

    /// Array of input details
    pub switch: [AbstractInput; SWITCHES_MAX],

    /// Lookup for inputs
    switch_byname: LinearMap<&'static str, usize, SWITCHES_MAX>,

    /// How many output channels?
    outputs: usize,

    /// Our record of outputs
    pub output: [AbstractOutput; OUTPUTS_MAX],

    /// Lookup for outputs
    output_byname: LinearMap<&'static str, usize, OUTPUTS_MAX>,

    /// Minimum closed time to register as long-press event
    long_closed: Duration,
}

impl Default for SwitchOutputController {
    fn default() -> Self {
        SwitchOutputController {
            switches: 0,
            switch: [Default::default(); SWITCHES_MAX],
            switch_byname: LinearMap::new(),
            outputs: 0,
            output: [Default::default(); OUTPUTS_MAX],
            output_byname: LinearMap::new(),
            long_closed: Duration::from_millis(900),
            switches_state: SwitchesState::None,
            state_detail: None,
        }
    }
}

impl SwitchOutputController {
    pub fn new(_double_duration: Duration, long_duration: Duration) -> SwitchOutputController {
        SwitchOutputController {
            long_closed: long_duration,
            ..Default::default()
        }
    }

    /// Get a reference to an input by name
    pub fn iidx(&self, name: &str) -> usize {
        *self
            .switch_byname
            .get(name)
            .expect("Lookup by constant name string failed")
    }

    /// Get a reference to an output by name
    pub fn oidx(&self, name: &str) -> usize {
        *self
            .output_byname
            .get(name)
            .expect("Lookup by constant name string failed")
    }

    /// General case add-a-switch.
    /// Return input and output structs (controller.input[x], controller.output[x])
    /// output_values: count of possible states. 2 for on/off, 4 might be off/low/med/high
    pub fn add_switch(
        &mut self,
        name: &'static str,
        output_values: u8,
        value_init: u8,
    ) -> (usize, usize) {
        let idx = self.switches;
        self.switches += 1;
        let output_idx = self.outputs;
        self.outputs += 1;
        self.output[output_idx] = AbstractOutput {
            idx: output_idx, // back-reference
            name,
            values: output_values,
            value: value_init,
        };
        self.output_byname
            .insert(name, output_idx)
            .expect("Don't reuse names!");
        self.switch[idx] = AbstractInput {
            idx, // back-reference
            name,
            output_idx,
            ..Default::default()
        };
        self.switch_byname
            .insert(name, idx)
            .expect("Don't reuse names!");
        (idx, output_idx)
    }

    /// Add a two-value switch whose output value 0/1 follows input false/true
    pub fn add_switch_momentary(&mut self, name: &'static str) -> (usize, usize) {
        let idx = self.switches;
        self.switches += 1;
        let output_idx = self.outputs;
        self.outputs += 1;
        self.output[output_idx] = AbstractOutput {
            idx: output_idx,
            name,
            values: 2,
            ..Default::default()
        };
        self.output_byname
            .insert(name, output_idx)
            .expect("Don't reuse names.");
        let momentary = true;
        self.switch[idx] = AbstractInput {
            idx,
            name,
            output_idx,
            momentary,
            ..Default::default()
        };
        self.switch_byname
            .insert(name, idx)
            .expect("Don't reuse names");
        (idx, output_idx)
    }

    /// Modify an already-added switch to control another output via
    /// long-press.
    pub fn augment_switch_longpress_add_output(
        &mut self,
        switch_idx: usize,
        name: &'static str,
        output_values: u8,
        value_init: u8,
    ) -> (usize, usize) {
        if self.switch[switch_idx].has_long_output {
            panic!("Only one long-press action can be assigned to a switch.");
        }
        let output_idx = self.outputs;
        self.outputs += 1;
        self.output[output_idx] = AbstractOutput {
            idx: output_idx,
            name,
            values: output_values,
            value: value_init,
            ..Default::default()
        };
        self.output_byname
            .insert(name, output_idx)
            .expect("Names must be unique");
        self.switch[switch_idx] = AbstractInput {
            has_long_output: true,
            long_output_idx: output_idx,
            long_specifies_max: false,
            ..self.switch[switch_idx]
        };
        (switch_idx, output_idx)
    }

    /// Modify an already-added switch to jump an output to its
    /// numerically-highest level on long-press.
    pub fn augment_switch_longpress_max_output(
        &mut self,
        switch_idx: usize,
        output_idx: usize,
    ) -> usize {
        if self.switch[switch_idx].has_long_output {
            panic!("Only one long-press action can be assigned to a switch.");
        }
        self.switch[switch_idx] = AbstractInput {
            has_long_output: true,
            long_output_idx: output_idx,
            long_specifies_max: true,
            ..self.switch[switch_idx]
        };
        switch_idx
    }

    // Map ins and current state onto outs and new state
    pub fn remap(&mut self) {
        // Momentaries: reflect incoming state to directly
        // corresponding output and then remove them from
        // consideration by our state machine by forcing the input
        // false.
        let saved_switches = self.switch; // we'll put them back below
        for sw in self.switch.iter_mut() {
            if sw.momentary {
                self.output[sw.output_idx].value = if sw.isclosed { 1 } else { 0 };
                sw.isclosed = false;
            }
        }

        // Adjust our internals
        match self.switches_state {
            SwitchesState::None => self.remap_from_none(),
            SwitchesState::One => self.remap_from_one(),
            SwitchesState::Long => self.remap_from_long(),
            SwitchesState::Jammed => self.remap_from_jammed(),
            /* multi-press, double-press could be implemented */
        };

        // Reinstate the original switches state. A caller following
        // guidelines won't care, but let's burn a few cycles to save
        // confusion.
        self.switch = saved_switches;
    }

    fn remap_from_none(&mut self) {
        if let Some(first_idx) = self
            .switch
            .iter()
            .enumerate()
            .find(|&(_, &x)| x.isclosed)
            .map(|(index, _)| index)
        {
            if self.switch[first_idx + 1..].iter().any(|&x| x.isclosed) {
                // multiple switches closed at the same time, relative to remap() calls.
                // We don't know which was pressed first, and don't have an action for this case.
                // Caller should be less generous with debounce waits?
                self.switches_state = SwitchesState::Jammed;
                self.state_detail = None;
            } else {
                self.switches_state = SwitchesState::One;
                self.state_detail = Some(StateDetail {
                    stamp: Instant::now(),
                    switch: self.switch,
                });
            }
        }
        // else no changes
    }

    fn remap_from_jammed(&mut self) {
        if !self.switch.iter().any(|&x| x.isclosed) {
            // No button is down. Back to None.
            self.switches_state = SwitchesState::None;
            self.state_detail = None;
        }
        // Else a button is down so we're still jammed, no changes.
    }

    fn remap_from_one(&mut self) {
        if let Some(first_idx) = self
            .switch
            .iter()
            .enumerate()
            .find(|&(_, &x)| x.isclosed)
            .map(|(index, _)| index)
        {
            // No change to any switch? Long-press, or do nothing.
            if self.switch == self.state_detail.unwrap().switch {
                // Check for long-press
                let interval =
                    Instant::now().saturating_duration_since(self.state_detail.unwrap().stamp);
                if self.switch[first_idx].has_long_output && interval > self.long_closed {
                    // This is a long-press. It can mean cycle an output, or max an output.
                    let output_idx: usize = self.switch[first_idx].long_output_idx;
                    if self.switch[first_idx].long_specifies_max {
                        self.output[output_idx].value = self.output[output_idx].values - 1;
                    } else {
                        self.output[output_idx].value += 1;
                        if self.output[output_idx].value >= self.output[output_idx].values {
                            self.output[output_idx].value = 0;
                        }
                    }
                    self.switches_state = SwitchesState::Long;
                    self.state_detail = None;
                }
                // Else do nothing, keep counting time.
            } else {
                // Switches changed, and at least one is still down.

                if self.switch[first_idx + 1..].iter().any(|&x| x.isclosed) {
                    // multiple switches are now closed. We're jammed until all released (no MULTI yet)
                    self.switches_state = SwitchesState::Jammed;
                    self.state_detail = None;
                } else {
                    // They report one switch is closed. Was it reported closed already?
                    if self.state_detail.unwrap().switch[first_idx].isclosed {
                        panic!("Trouble: should have already caught the no-change case");
                    }

                    // Yikes, they released the switch but a different
                    // switch is down. Treat this like a second button
                    // press after the first.

                    // First handle the output of the switch that just opened
                    let old_idx = self
                        .state_detail
                        .unwrap()
                        .switch
                        .iter()
                        .enumerate()
                        .find(|&(_, &x)| x.isclosed)
                        .map(|(index, _)| index)
                        .unwrap();
                    let old_output_idx = self.switch[old_idx].output_idx;
                    self.output[old_output_idx].value += 1;
                    if self.output[old_output_idx].value >= self.output[old_output_idx].values {
                        self.output[old_output_idx].value = 0
                    }

                    // Then make a new start with the new button down. Just one, right? (Yes, checked above)
                    self.switches_state = SwitchesState::One;
                    self.state_detail = Some(StateDetail {
                        stamp: Instant::now(),
                        switch: self.switch,
                    });
                }
            }
        } else {
            // Which switch was closed previously? It's now open, prior to Long, so cycle its output.
            if let Some(first_idx) = self
                .state_detail
                .unwrap()
                .switch
                .iter()
                .enumerate()
                .find(|&(_, &x)| x.isclosed)
                .map(|(index, _)| index)
            {
                // They released the only switch that was down, before the long-press timer expired.
                // (Learning this requires the caller to remap() repeatedly with no-change reports
                //  while switches are closed.)

                // Check our work: be sure there wasn't a second switch down previously,
                // with both released at the same moment. This is just paranoia.
                if self.state_detail.unwrap().switch[first_idx + 1..]
                    .iter()
                    .any(|&x| x.isclosed)
                {
                    panic!("Logic problem: in state One we found 2 or more switches closed.");
                }

                // Toggle the output.
                let output_idx = self.switch[first_idx].output_idx;
                self.output[output_idx].value += 1;
                if self.output[output_idx].value >= self.output[output_idx].values {
                    self.output[output_idx].value = 0
                }
                self.switches_state = SwitchesState::None;
                self.state_detail = None;
            } else {
                // more paranoia
                panic!(
                    "Logic trouble, we were in state One but with no switches closed previously."
                );
            }
        }
    }

    fn remap_from_long(&mut self) {
        if !self.switch.iter().any(|&x| x.isclosed) {
            // End the long-press state, during which no other switch changes have any effect.
            self.switches_state = SwitchesState::None;
            self.state_detail = None;
        }
        // Any other change or no-change, do nothing.
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn none_from_none() {
        let mut c: SwitchOutputController = Default::default();
        c.add_switch(2, 0);
        c.remap();
        matches!(c.switches_state, SwitchesState::None);
        assert_eq!(c.output[0].value, 0);
        assert_eq!(c.output[1].value, 0);
        assert_eq!(c.output[2].value, 0);
        assert_eq!(c.output[3].value, 0);
        assert_eq!(c.output[4].value, 0);
        // ...
        assert_eq!(c.output[OUTPUTS_MAX - 1].value, 0);
    }

    #[test]
    fn one_from_none() {
        let mut c: SwitchOutputController = Default::default();
        let (sw0, out0) = c.add_switch(2, 0);
        assert_eq!(sw0.idx, 0);
        assert_eq!(out0.idx, 0);
        c.switch[0].isclosed = true;
        c.remap();
        matches!(c.switches_state, SwitchesState::One);
        assert_eq!(c.output[0].value, 0);
        assert_eq!(c.output[1].value, 0);
        assert_eq!(c.output[2].value, 0);
        assert_eq!(c.output[3].value, 0);
        assert_eq!(c.output[4].value, 0);
        // ...
        assert_eq!(c.output[OUTPUTS_MAX - 1].value, 0);
    }

    fn state_one_from_scratch() -> SwitchOutputController {
        let mut c: SwitchOutputController = Default::default();
        let (_, _) = c.add_switch(2, 0);
        c.switch[0].isclosed = true;
        c.remap();
        validate_setup_state_one(c);
        c
    }

    fn validate_setup_state_one(c: SwitchOutputController) {
        matches!(c.switches_state, SwitchesState::One);
        assert!(c.switch[0].isclosed);
        assert!(!c.switch[1].isclosed);
        assert!(!c.switch[2].isclosed);
        assert!(!c.switch[3].isclosed);
        assert!(!c.switch[4].isclosed);
        // ...
        assert!(!c.switch[SWITCHES_MAX - 1].isclosed);

        assert_eq!(c.output[0].value, 0);
        assert_eq!(c.output[1].value, 0);
        assert_eq!(c.output[2].value, 0);
        assert_eq!(c.output[3].value, 0);
        assert_eq!(c.output[4].value, 0);
        // ...
        assert_eq!(c.output[OUTPUTS_MAX - 1].value, 0);
    }

    #[test]
    fn one_from_one() {
        let mut c = state_one_from_scratch();

        // repeat same input
        c.remap();
        validate_setup_state_one(c);
    }

    #[test]
    fn none_from_one() {
        let mut c = state_one_from_scratch();

        // open the switch
        c.switch[0].isclosed = false;
        c.remap();

        matches!(c.switches_state, SwitchesState::None);
        assert_eq!(c.output[0].value, 1);
        assert_eq!(c.output[1].value, 0);
        assert_eq!(c.output[2].value, 0);
        assert_eq!(c.output[3].value, 0);
        assert_eq!(c.output[4].value, 0);
        // ...
        assert_eq!(c.output[OUTPUTS_MAX - 1].value, 0);
    }

    #[test]
    fn jammed_from_one() {
        let mut c = state_one_from_scratch();

        // close another switch
        c.switch[1].isclosed = true;
        c.remap();

        matches!(c.switches_state, SwitchesState::Jammed);
        // no change to output state for original switch, since it wasn't released
        assert_eq!(c.output[0].value, 0);
        // no change to any other output
        assert_eq!(c.output[1].value, 0);
        assert_eq!(c.output[2].value, 0);
        assert_eq!(c.output[3].value, 0);
        assert_eq!(c.output[4].value, 0);
        // ...
        assert_eq!(c.output[OUTPUTS_MAX - 1].value, 0);
    }

    #[test]
    fn still_jammed_on_changes_and_unjammed_when_all_released() {
        let mut c = state_one_from_scratch();

        // close another switch to get jammed
        c.switch[1].isclosed = true;
        c.remap();
        matches!(c.switches_state, SwitchesState::Jammed);
        assert_eq!(c.output[0].value, 0);
        assert_eq!(c.output[1].value, 0);
        assert_eq!(c.output[2].value, 0);
        assert_eq!(c.output[3].value, 0);
        assert_eq!(c.output[4].value, 0);
        // ...
        assert_eq!(c.output[OUTPUTS_MAX - 1].value, 0);

        // open that other switch
        c.switch[1].isclosed = false;
        c.remap();
        matches!(c.switches_state, SwitchesState::Jammed);
        assert_eq!(c.output[0].value, 0);
        assert_eq!(c.output[1].value, 0);
        assert_eq!(c.output[2].value, 0);
        assert_eq!(c.output[3].value, 0);
        assert_eq!(c.output[4].value, 0);
        // ...
        assert_eq!(c.output[OUTPUTS_MAX - 1].value, 0);

        // close a different switch
        c.switch[2].isclosed = true;
        c.remap();
        matches!(c.switches_state, SwitchesState::Jammed);
        assert_eq!(c.output[0].value, 0);
        assert_eq!(c.output[1].value, 0);
        assert_eq!(c.output[2].value, 0);
        assert_eq!(c.output[3].value, 0);
        assert_eq!(c.output[4].value, 0);
        // ...
        assert_eq!(c.output[OUTPUTS_MAX - 1].value, 0);

        // open the original switch, stay jammed
        c.switch[0].isclosed = false;
        c.remap();
        matches!(c.switches_state, SwitchesState::Jammed);
        assert_eq!(c.output[0].value, 0);
        assert_eq!(c.output[1].value, 0);
        assert_eq!(c.output[2].value, 0);
        assert_eq!(c.output[3].value, 0);
        assert_eq!(c.output[4].value, 0);
        // ...
        assert_eq!(c.output[OUTPUTS_MAX - 1].value, 0);

        // One switch is still closed. Open it, get unjammed
        c.switch[2].isclosed = false;
        c.remap();
        matches!(c.switches_state, SwitchesState::None);

        // No outputs were turned on through that entire process.
        assert_eq!(c.output[0].value, 0);
        assert_eq!(c.output[1].value, 0);
        assert_eq!(c.output[2].value, 0);
        assert_eq!(c.output[3].value, 0);
        assert_eq!(c.output[4].value, 0);
        // ...
        assert_eq!(c.output[OUTPUTS_MAX - 1].value, 0);
    }
}
