# MCAux-CT

[![dependency status](https://deps.rs/repo/github/swork/mcaux-ct/status.svg)](https://deps.rs/repo/github/swork/mcaux-ct)
[![Build Status](https://github.com/swork/mcaux-ct/workflows/CI/badge.svg)](https://github.com/swork/mcaux-ct/actions?workflow=CI)

This is the repo for mcaux-ct, my motorcycle auxiliary equipment controller, a quixotic art project for my KLR650.

## temp: state machine

"AuxState" below should be more abstract: "MomentaryControllerState"

### State management in the abstract

Bools for momentary-contact buttons, u8s for output, and a last-change timestamp make up the dynamic system state. A cycle count for each output makes up configured state: 2 for on/off, 4 might be low/med/high/off. Changed state (inputs and time) and previous state (inputs, outputs and last change time) entirely determine next state (outputs).

```
// macro these for array sizes etc.
// AuxState::INS: u8
// AuxState:: OUTS: [u8; N]
// AuxState::OUTS_INIT: [u8; N]
// AuxState::DOUBLE_MS: u32
// AuxState::LONG_MS: u32

enum AuxState {
    None(out: [u8; 4], time: Timestamp, last: AuxState),
    One(in: [bool; 3], out: [u8; 4], time: Timestamp, last: AuxState, prev: AuxState),
    Double(in: [bool; 3], out: [u8; 4], time: Timestamp, last: AuxState, first: AuxState),
    Multi(in: [bool; 3], out: [u8; 4], first: AuxState, last: AuxState, time: Timestamp),
}

trait Button for AuxState
where
    Self: Sized
{
    fn run(io: AuxState) -> AuxState,
}
```

### Concrete implementation for MC controller, with abstract driver

```
// use motorcycle_controller_hardware::Driver;
use motorcycle_controller_web_harness::Driver;
use momentary_controller::{AuxState, Button};

impl Button for AuxState::None



/*    
    
    {
        match io {
        None(out, time, prev) if prev is None {
            None(out, time, prev), // nothing happened
        },
        None(out, time, prev) if prev is Multi {
            
            AuxState_AnyDown{ bits: io.bits }
         },
         _ {
             AuxState_NoneClosed { io: io.bits }
         }
     }
}


fn st(n: AuxState): AuxState {


*/
```
