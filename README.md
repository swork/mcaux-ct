# MCAux-CT

[![dependency status](https://deps.rs/repo/github/swork/mcaux-ct/status.svg)](https://deps.rs/repo/github/swork/mcaux-ct)
[![Build Status](https://github.com/swork/mcaux-ct/workflows/CI/badge.svg)](https://github.com/swork/mcaux-ct/actions?workflow=CI)

This is the repo for mcaux-ct, my motorcycle auxiliary equipment controller, a quixotic art project for my KLR650.

Independent separate crates are subdirs - Rust doesn't make it easy to manage a workspace crate that builds to different architectures, so let's just duck the problems:

 - momentary/ abstracts some of the problems of doing state-changey things with momentary contact switches. It includes tests that run on the host, but also some conditional compilation so it can be used in the rp2XXX firmware.
 
 - demo/ builds an egui/eframe mockup of the switches mostly for testing and experimenting with momentary/. It does not try to match the indicator animations in mcaux-rp2040, as the needs of color adjustments in egui and PWM manipulations under LED hardware are awkwardly different.
 
 - mcaux-rp2040 builds the firmware to run the switches box on the motorcycle.
 
## Thoughts on long press

Long sw0 is now out3 on/off.

Long sw1 could be aux lights on regardless of headlight state (like during park-light operation). Any change of headlight low/high/off could revert aux lights to wherever they were when long-pressed.

Long sw2 could be unequivocal Off from any On, and On Full from off versus short-press Low.

Multi remains indicator brightness adjustment, need to work out persistence or not

Double-press is an open question... got it, use it?
