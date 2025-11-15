use core::panic;
use std::time::{Duration, Instant};

const SWITCHES: usize = 3;
const OUTPUTS: usize = 3;

/// State when no input switches are closed; also, with recent:None, the initial state.
struct NoneState {
    stamp: Instant,
    output: [u8; OUTPUTS],
    recent: Box<Item>,
}

impl NoneState {
    fn report(
        from: &Option<NoneState>,
        incoming: &[bool; SWITCHES],
        state: &mut MomentaryControllerState,
    ) -> Item {
        if let Some(first_idx) = incoming
                .iter()
                .enumerate()
                .find(|&(_, &x)| x)
                .map(|index, _| index)
        { /// TODO WORKING HERE WHEN DECIDING TO MOVE TO SEPARATE CRATE
            if let Some(second_idx) = incoming[first_idx+1..].find(|&&x| x) {
                // multiple switches closed at the same time, unlikely in hardware but let's be robust.
                // Note the possibility that this event could be within the double-click time of one
                // switch or the other, making it ambiguous - did they mean a double-click of that
                // switch, or a multi-click following a single click? We simply assume the latter here.
                //
            } else {
                //if let Some(is_doubleclick) = double_click_q(later: Item>, earlier: Option<Item)
                //output[first_idx] += 1;
                Item::One(
                    OneState {
                        stamp: Instant::now(),
                        switch: incoming,
                        initial: Box::new(Item::None(from)),
                        recent: Box::new(Item::None(from)),
                        output: 
                    }
                        
                )
            }
        }
        match incoming.iter().count() {
            0 => Item::None(*from),  // they reported nothing happened
            1 => {
                if let Some(first_idx) = incoming.iter().find(|&&x| x);
                if second_idx = incoming[first_idx..].iter().find(|&&x| x);
                Item::One(
                    OneState {
                        stamp: Instant::now(),
                        switch: incoming,
                        initial: Box::new(Item::None(from)),
                        recent: Box::new(Item::None(from)),
                        output: 
                    }
                )
            }
            _ => Item::Multi(MultiState),  // odd condition for hardware: more than one switch simultaneously. Handle it robustly.
        }
    }
}

/// State while one button is held closed briefly.
struct OneState {
    stamp: Instant,
    switch: [bool; SWITCHES],
    output: [u8; OUTPUTS],
    initial: Box<Item>,
    recent: Box<Item>,
}

/// State when one switch has been held closed briefly (less than the long-press duration), opened before the long-press duration has passed, then closed again before the double-press duration has passed; all without another switch being closed. In this state, with that initial switch closed, other switches may then be closed subsequently (but we will not recognize double- or long-presses of those subsequent switches).
struct DoubleState {
    stamp: Instant,
    switch: [bool; SWITCHES],
    output: [u8; OUTPUTS],
    initial: Box<Item>,
    recent: Box<Item>,
}

/// State when one switch has been held closed briefly (less than the long-press duration), and during this interval another switch is closed. This state holds until the first switch is opened.
struct MultiState {
    stamp: Instant,
    switch: [bool; SWITCHES],
    output: [u8; OUTPUTS],
    initial: Box<Item>,
    recent: Box<Item>,
}

/// State when one switch has been held closed longer than the long-press duration. This state holds until that switch is opened.
struct LongState {
    stamp: Instant,
    switch: [bool; SWITCHES],
    output: [u8; OUTPUTS],
    initial: Box<Item>,
    recent: Box<Item>,
}

enum Item {
    None(Option<NoneState>),
    One(OneState),
    Double(DoubleState),
    Multi(MultiState),
    Long(LongState),
}

pub struct MomentaryControllerState {
    /// Current reported state of the system, None at start
    state: Item,

    /// How many input momentary-contacts?
    switches: usize,

    /// How many output channels?
    outputs: usize,

    /// For each output, how many possible states? On/off: 2, low/med/high: 4, for example.
    output_cycles: [u8; OUTPUTS],

    /// Maximum open time between input closes to register a double-press event
    double_open_time: Duration,

    /// Minimum closed time to register as long-press event
    long_close_time: Duration,
}

impl MomentaryControllerState {
    pub fn new(
        switches: &usize,
        outputs: &usize,
        output_cycles: &[u8; OUTPUTS],
        output_inits: &[u8; OUTPUTS],
        double_open_time: &Duration,
        long_close_time: &Duration,
    ) -> MomentaryControllerState {
        return MomentaryControllerState {
            switches: *switches,
            outputs: *outputs,
            output_cycles: *output_cycles,
            double_open_time: *double_open_time,
            long_close_time: *long_close_time,
            state: Item::None(None),
        };
    }

    pub fn report(
        &mut self,
        incoming: &[bool; SWITCHES],
    ) -> [u8; OUTPUTS] {
        match self.state {
            Item::None(..) => {
                self.next_state(Item::None::report(self.state[0], incoming))
            }
            Item::One(..) => {
                self.next_state(Item::One::report(self, incoming))
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
        self.outputs
    }

    fn next_state(&self, determined: Item) {
        let mut new_state = self.clone();
        new_state.state = determined;
        new_state
    }
}
