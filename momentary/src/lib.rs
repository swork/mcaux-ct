use core::panic;
use std::time::{Duration, Instant};

use tracing::warn;

const SWITCHES: usize = 16;
const OUTPUTS: usize = 16;

/// State when no input switches are closed; also, with recent:None, the initial state.
#[derive(Clone, Copy)]
struct NoneState {
    _stamp: Instant,
}

impl Default for NoneState {
    fn default() -> Self {
        NoneState {
            _stamp: Instant::now(),
        }
    }
}

impl NoneState {
    fn report(
        &self,
        incoming: [bool; 16],
    ) -> Item {
        if let Some(first_idx) = incoming
                .iter()
                .enumerate()
                .find(|&(_, &x)| x)
                .map(|(index, _)| index)
        {
            if let Some(_) = incoming[first_idx+1..].iter().find(|&&x| x) {
                // multiple switches closed at the same time, unlikely in hardware but let's be robust.
                // Note the possibility that this event could be within the double-click time of one
                // switch or the other, making it ambiguous - did they mean to press both switches, or a double-click of that
                // switch, or a multi-click following a single click? We simply assume the latter here.
                //
                panic!("MULTI not yet implemented");
            }

            return Item::One(
                OneState {
                    stamp: Instant::now(),
                    switches: incoming,
                });
        }

        Item::None(self.clone())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn none_from_none() {
        let mut c: MomentaryController = Default::default();
        c.add_switch(2);
        let ins: [bool; SWITCHES] = [false; SWITCHES];

        c.report(ins);
        assert_eq!(c.output, [0; OUTPUTS]);
        assert!(match c.state {
            Item::None(_) => true,
            _ => false,
        });
    }

    #[test]
    fn one_from_none() {
        let mut c: MomentaryController = Default::default();
        c.add_switch(2);
        let mut ins: [bool; SWITCHES] = [false; SWITCHES];
        ins[0] = true;
        c.report(ins);
        assert!(match c.state {
            Item::One(_) => true,
            _ => false,
        });
        assert_eq!(c.output, [0; OUTPUTS]);
    }
}


/// State while one button is held closed briefly.
#[derive(Clone, Copy)]
struct OneState {
    stamp: Instant,
    switches: [bool; SWITCHES],
}

impl OneState {
    fn report(
        &self,
        incoming: [bool; SWITCHES],
        parent: &mut MomentaryController,
    ) -> Item {
        if let Some(first_idx) = incoming
                .iter()
                .enumerate()
                .find(|&(_, &x)| x)
                .map(|(index, _)| index)
        {
            // first handle MULTI. Else:

            // No change? Long-press, or do nothing.
            warn!("OneState report: incoming: {:?}, self.switches: {:?}", incoming, self.switches);
            if incoming == self.switches {
                // Check for long-press
                let interval = Instant::now().saturating_duration_since(self.stamp);
                if parent.has_long[first_idx] && interval > parent.long_closed {
                    warn!("Long-press detected on switch {}, duration was {:?}", first_idx, interval);
                    // This is a long-press.
                    let output_idx: usize = parent.long[first_idx];
                    parent.output[output_idx] += 1;
                    if parent.output[output_idx] >= parent.output_cycles[output_idx] {
                        parent.output[output_idx] = 0;
                    }
                    return Item::Long(
                        LongState {
                        });
                } else {
                    // do nothing, keep counting time.
                    return Item::One(self.clone());
                }
            } else {
                // Switches changed, and at least one is still down.

                if let Some(_) = incoming[first_idx+1..].iter().find(|&&x| x == true) {
                    // multiple switches closed at the same time, unlikely in hardware but let's be robust.
                    // Note the possibility that this event could be within the double-click time of one
                    // switch or the other, making it ambiguous - did they mean to press both switches, or a double-click of that
                    // switch, or a multi-click following a single click? We simply assume the latter here.
                    //  
                    panic!("MULTI not yet implemented");
                } else {
                    // They report one switch is closed. Was it reported closed already?
                    if self.switches[first_idx] {
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
            if let Some(first_idx) = self.switches
                .iter()
                .enumerate()
                .find(|&(_, &x)| x)
                .map(|(index, _)| index)
            {
                // Check our work: be sure there wasn't a second switch down previously,
                // with both released at the same moment
                if let Some(_) = self.switches[first_idx+1..].iter().find(|&&x| x) {
                    panic!("Logic problem: in state One we found 2 or more switches closed.");
                }

                // Toggle the output.
                parent.output[first_idx] += 1;
                if parent.output[first_idx] >= parent.output_cycles[first_idx] {
                    parent.output[first_idx] = 0
                }
                return Item::None(
                    NoneState {
                        _stamp: Instant::now(),
                    });
            } else {
                panic!("Logic trouble, no-switches-before case should have been caught above");
            }
        }
    }
}

/// State when one switch has been held closed longer than the long-press duration. This state holds until that switch is opened.
#[derive(Clone, Copy)]
struct LongState {
}

impl Default for LongState {
    fn default() -> Self {
        LongState {
        }
    }
}

impl LongState {
    fn report(
        &self,
        incoming: [bool; SWITCHES],
    ) -> Item {
        if incoming.iter().count() == 0 {
            // End the long-press state, during which no other switch changes have any effect.
            Item::None(Default::default())
        } else {
            // Any other change, do nothing.
            Item::Long(self.clone())
        }
    }
}

/*
/// State when one switch has been held closed briefly (less than the long-press duration), opened before the long-press duration has passed, then closed again before the double-press duration has passed; all without another switch being closed. In this state, with that initial switch closed, other switches may then be closed subsequently (but we will not recognize double- or long-presses of those subsequent switches).
struct DoubleState {
    stamp: Instant,
    switches: [bool; SWITCHES],
}

/// State when one switch has been held closed briefly (less than the long-press duration), and during this interval another switch is closed. This state holds until the first switch is opened.
struct MultiState {
    stamp: Instant,
    switches: [bool; SWITCHES],
}
*/

enum Item {
    None(NoneState),
    One(OneState),
    Long(LongState),
    /*
    Double(DoubleState),
    Multi(MultiState),
    */
}

pub struct MomentaryController {
    /// false: configuring. true: running. One way trip between them.
    started: bool,

    /// Current reported state of the system, None at start
    state: Item,

    /// How many input momentary-contacts?
    switches: usize,

    /// How many output channels?
    outputs: usize,

    /// Our record of the outputs themselves
    output: [u8; OUTPUTS],

    /// Output state from which first report generates first change. Moved to output at first report, invalid after that.
    output_init: [u8; OUTPUTS],

    /// Has a long-press output been established for this switch?
    has_long: [bool; SWITCHES],

    /// If a long-press output has been established for this switch, which output?
    long: [usize; SWITCHES],

    /// For each output, how many possible states? On/off: 2, low/med/high: 4, for example.
    output_cycles: [u8; OUTPUTS],

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
            output: [0; OUTPUTS],
            output_cycles: [0; OUTPUTS],
            output_init: [0; OUTPUTS],
            has_long: [false; SWITCHES],
            long: [0; SWITCHES],
//            double_open: Duration::from_millis(500),
            long_closed: Duration::from_millis(2000),
            state: Item::None(
                NoneState {
                    _stamp: Instant::now().checked_sub(Duration::from_secs(60)).expect("System clock trouble"),
                }),
        }
    }
}

impl MomentaryController {
    pub fn new(
        _double_duration: Duration,
        long_duration: Duration,
    ) -> MomentaryController {
        MomentaryController {
            started: false,
            switches: 0,
            outputs: 0,
            output: [0; OUTPUTS],
            output_cycles: [0; OUTPUTS],
            output_init: [0; OUTPUTS],
            has_long: [false; SWITCHES],
            long: [0; SWITCHES],
//            double_open: double_duration,
            long_closed: long_duration,
            state: Item::None(
                NoneState {
                    _stamp: Instant::now().checked_sub(Duration::from_secs(60)).expect("System clock trouble"),
                }),
        }
    }

    /// General case add-a-switch with all parameters.
    /// Return the index of the switch added (same as output index)
    pub fn add_switch(&mut self, output_cycle: u8) -> (usize, usize) {
        if self.started {
            panic!("Don't add switches after first .report()");
        }
        let switch_idx = self.switches;
        self.switches += 1;
        let output_idx = self.outputs;
        self.output_cycles[output_idx] = output_cycle;
        self.outputs += 1;
        (switch_idx, output_idx)
    }

    /// Modify an already-added switch to control another output via long-press.
    pub fn augment_switch_longpress(&mut self, switch_idx: usize, output_cycle: u8) -> (usize, usize) {
        if self.started {
            panic!("Don't augment switches after first .report()");
        }
        if switch_idx >= self.switches {
            panic!("Don't specify long-press on a switch that has not yet been added");
        }
        let output_idx = self.outputs;
        self.outputs += 1;
        self.output_cycles[output_idx] = output_cycle;
        self.has_long[switch_idx] = true;
        self.long[switch_idx] = output_idx;
        (switch_idx, output_idx)
    }

    pub fn report(
        &mut self,
        incoming: [bool; SWITCHES],
    ) -> [u8; OUTPUTS] {
        if ! self.started {
            self.output = self.output_init;
            self.started = true;
        }
        self.state = match self.state {
            Item::None(deets) => {
                deets.report(incoming)
            }
            Item::One(deets) => {
                deets.report(incoming, self)
            }
            Item::Long(deets) => {
                deets.report(incoming)
            }
            /*
            Item::Multi(..) => {
                panic!("not implemented")
            }
            Item::Double(..) => {
                panic!("not implemented")
            }
            */
        };
        self.output.clone()
    }
}

