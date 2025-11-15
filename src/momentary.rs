use std::time::{Duration, Instant};

enum ButtonClosedState {
    None(out: [u8; 16], last: MomentaryControllerState, time: Instant),
    One(in: [bool; 16], out: [u8; 16], last: MomentaryControllerState, prev: MomentaryControllerState, time: Instant),
    Double(in: [bool; 3], out: [u8; 4], time: Timestamp, last: AuxState, first: AuxState),
    Multi(in: [bool; 3], out: [u8; 4], first: AuxState, last: AuxState, time: Timestamp),
}

struct MomentaryControllerState {
    closed: ButtonClosedState,
}
