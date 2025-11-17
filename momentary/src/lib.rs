use core::panic;
use std::time::{Duration, Instant};

const SWITCHES: usize = 16;
const OUTPUTS: usize = 16;

#[derive(Clone, Copy)]
struct PrevState {
    stamp: Instant,
    switches: [bool; SWITCHES],
}

impl Default for PrevState {
    fn default() -> Self {
        PrevState {
            stamp: Instant::now(),
            switches: [false; SWITCHES],
        }
    }
}

impl PrevState {
    fn reset() -> Self {
        PrevState {
            stamp: Instant::now().checked_sub(Duration::from_secs(60)).unwrap(),
            switches: [false; SWITCHES],
        }
    }
}

/// State when no input switches are closed; also, with recent:None, the initial state.
#[derive(Clone, Copy)]
struct NoneState {
    stamp: Instant,
    switches: [bool; SWITCHES],
    previous: PrevState,
}

impl Default for NoneState {
    fn default() -> Self {
        NoneState {
            stamp: Instant::now(),
            switches: [false; SWITCHES],
            previous: Default::default(),
        }
    }
}

impl NoneState {
    fn report(
        &self,
        incoming: [bool; 16],
        parent: &mut MomentaryController,
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

            // There is no second switch closed, just the one. Toggle output.
            parent.output[first_idx] += 1;
            if parent.output[first_idx] >= parent.output_cycles[first_idx] {
                parent.output[first_idx] = 0
            }

            return Item::One(
                OneState {
                    stamp: Instant::now(),
                    switches: incoming,
                    previous: Default::default(),
                });
        }

        // No switches are closed, same as last report. No change if we're inside double-click window (we might be waiting to see if they double-click),
        // or if there's no history. Else lose all history as it's no longer needed.
        let interval = Instant::now().saturating_duration_since(self.previous.stamp);
        if interval < parent.double_open {
            return Item::None(self.clone());
        }
            
        // All switches have been open long enough that we don't need history any more.
        return Item::None(
            NoneState {
                stamp: Instant::now(),
                switches: incoming,
                previous: PrevState::reset(),
            });
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn none_from_none() {
        let mut c: MomentaryController = Default::default();
        c.add_switch();
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
        c.add_switch();
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
    previous: PrevState,
}

impl OneState {
    fn report(
        &self,
        incoming: [bool; SWITCHES],
        parent: &mut MomentaryController,
    ) -> Item {
        // No change? Long-press, or do nothing.
        if incoming == self.previous.switches {
            let interval = Instant::now().saturating_duration_since(self.previous.stamp);
            if interval > parent.long_closed {
                panic!("long-press not yet implemented");
            }
            return Item::One(
                OneState {
                    stamp: Instant::now(),
                    switches: incoming,
                    previous: PrevState {
                        stamp: self.stamp,
                        switches: self.switches,
                    },
                }
            );
        }

        // Something changed; what? Most likely, they released a switch.
        // Other possibilities: they pressed another switch (MULTI)
        // or (least likely) they simultaneously released a switch and pressed another.
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
            } else {
                // They report one switch is closed. Was it reported closed already?
                if self.switches[first_idx] {
                    panic!("Trouble: should have already caught the no-change case");
                }

                // Yikes, they released the switch but a different switch is down. Treat this like a second report
                panic!("Simultaneous release and press of two switches not yet implemented");
            }
        } else {
            // They released the only switch that was down. Toggle the output, 
            // and remember the situation in case they intend a double-click.
            if let Some(first_idx) = self.previous.switches
                .iter()
                .enumerate()
                .find(|&(_, &x)| x)
                .map(|(index, _)| index)
            {
                // Check our work: be sure there wasn't a second switch down previously,
                // with both released at the same moment
                if let Some(_) = self.previous.switches[first_idx+1..].iter().find(|&&x| x) {
                    panic!("Logic problem: in state One we found 2 or more switches closed.");
                }
                
                parent.output[first_idx] += 1;
                if parent.output[first_idx] >= parent.output_cycles[first_idx] {
                    parent.output[first_idx] = 0
                }
                return Item::None(
                    NoneState {
                        stamp: Instant::now(),
                        switches: incoming,
                        previous: PrevState {
                            switches: self.switches,
                            stamp: Instant::now(),
                        },
                    });
            } else {
                panic!("Logic trouble, no-prev-switches case should have been caught above");
            }
        }
    }
}



/// State when one switch has been held closed briefly (less than the long-press duration), opened before the long-press duration has passed, then closed again before the double-press duration has passed; all without another switch being closed. In this state, with that initial switch closed, other switches may then be closed subsequently (but we will not recognize double- or long-presses of those subsequent switches).
struct DoubleState {
    stamp: Instant,
    switches: [bool; SWITCHES],
    previous: Box<Item>,
}

/// State when one switch has been held closed briefly (less than the long-press duration), and during this interval another switch is closed. This state holds until the first switch is opened.
struct MultiState {
    stamp: Instant,
    switches: [bool; SWITCHES],
    previous: Box<Item>,
}

/// State when one switch has been held closed longer than the long-press duration. This state holds until that switch is opened.
struct LongState {
    stamp: Instant,
    switches: [bool; SWITCHES],
    previous: Box<Item>,
}

enum Item {
    None(NoneState),
    One(OneState),
    Double(DoubleState),
    Multi(MultiState),
    Long(LongState),
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

    /// For each output, how many possible states? On/off: 2, low/med/high: 4, for example.
    output_cycles: [u8; OUTPUTS],

    /// Maximum open time between input closes to register a double-press event
    double_open: Duration,

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
            double_open: Duration::from_millis(500),
            long_closed: Duration::from_millis(2000),
            state: Item::None(
                NoneState {
                    stamp: Instant::now().checked_sub(Duration::from_secs(60)).expect("System clock trouble"),
                    switches: [false; SWITCHES],
                    previous: PrevState::reset(),
                }),
        }
    }
}

impl MomentaryController {
    pub fn new(
        double_duration: Duration,
        long_duration: Duration,
    ) -> MomentaryController {
        MomentaryController {
            started: false,
            switches: 0,
            outputs: 0,
            output: [0; OUTPUTS],
            output_cycles: [0; OUTPUTS],
            output_init: [0; OUTPUTS],
            double_open: double_duration,
            long_closed: long_duration,
            state: Item::None(
                NoneState {
                    stamp: Instant::now().checked_sub(Duration::from_secs(60)).expect("System clock trouble"),
                    switches: [false; SWITCHES],
                    previous: PrevState::reset(),
                }),
        }
    }

    /// Add a simple switch to the system, press-on/press-off for the corresponding output.
    /// Return the index of the switch added.
    pub fn add_switch(&mut self) -> usize {
        if self.started {
            panic!("Don't add switches after first .report()");
        }
        let idx = self.switches;
        self.switches += 1;
        self.outputs += 1;
        idx
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
                deets.report(incoming, self)
            }
            Item::One(deets) => {
                deets.report(incoming, self)
            }
            Item::Multi(..) => {
                panic!("not implemented")
            }
            Item::Double(..) => {
                panic!("not implemented")
            }
            Item::Long(..) => {
                panic!("not implemented")
            }
        };
        self.output.clone()
    }
}

