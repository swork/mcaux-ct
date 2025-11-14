# eframe template

[![dependency status](https://deps.rs/repo/github/swork/mcaux-ct/status.svg)](https://deps.rs/repo/github/swork/mcaux-ct)
[![Build Status](https://github.com/swork/mcaux-ct/workflows/CI/badge.svg)](https://github.com/swork/mcaux-ct/actions?workflow=CI)

This is the repo for mcaux-ct, my motorcycle auxiliary equipment controller, an art project for my KLR650.

## temp: state machine

Eight bits and a last-change timestamp make up the system state. Changed state (inputs and time) and previous state (inputs, outputs and last change time) entirely determine next state (outputs).

struct AuxState {
    bits: u8,
}
impl Button for AuxState
where
    Self: Sized
{
    fn run(io: AuxState) -> AuxState {
        match io.bits & 0xf0 {
         b if io.bits != 0 {
             AuxState_AnyDown{ bits: io.bits }
         },
         _ {
             AuxState_NoneClosed { io: io.bits }
         }
     }
}


fn st(n: AuxState): AuxState {
    
