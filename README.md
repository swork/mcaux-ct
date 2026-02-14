# MCAux-CT

 - [![CI rp235x](https://github.com/swork/mcaux-ct/actions/workflows/ci-rp235x.yml/badge.svg)](https://github.com/swork/mcaux-ct/actions/workflows/ci-rp235x.yml)[![dependency status](https://deps.rs/repo/github/swork/mcaux-ct/status.svg?path=mcaux-rp235x)](https://deps.rs/repo/github/swork/mcaux-ct?path=mcaux-rp235x)
 - [![CI rp2040](https://github.com/swork/mcaux-ct/actions/workflows/ci-rp2040.yml/badge.svg)](https://github.com/swork/mcaux-ct/actions/workflows/ci-rp2040.yml)[![dependency status](https://deps.rs/repo/github/swork/mcaux-ct/status.svg?path=mcaux-rp2040)](https://deps.rs/repo/github/swork/mcaux-ct?path=mcaux-rp2040)
 - [![CI demo](https://github.com/swork/mcaux-ct/actions/workflows/ci-demo.yml/badge.svg)](https://github.com/swork/mcaux-ct/actions/workflows/ci-demo.yml)[![dependency status](https://deps.rs/repo/github/swork/mcaux-ct/status.svg?path=demo)](https://deps.rs/repo/github/swork/mcaux-ct?path=demo)

This is the repo for mcaux-ct, a motorcycle auxiliary equipment controller, a quixotic art project for my KLR650.

Independent separate crates are subdirs - Rust doesn't make it easy to manage a workspace crate that builds to different architectures, so let's just duck the problems:

 - momentary/ abstracts some of the problems of doing state-changey things with momentary contact switches. It includes tests that run on the host, but also some conditional compilation so it can be used in the rp2XXX firmware, and as part of a host-native demo app that can also be built to run on WASM in a web browser. (egui/eframe make this last bit possible.)
 
 - demo/ builds that egui/eframe mockup of the switches mostly for testing and experimenting with momentary/. It does not (yet?) try to match the indicator animations in mcaux-rp2040, as the needs of color adjustments in egui and PWM manipulations under LED hardware are awkwardly different.
 
 - mcaux-rp2040 and -rp235x build the firmware to run the switches box on the motorcycle.
 
## Thoughts on long press

Long sw0 is now out3 on/off.

Long sw2 is grip heat On Full, so a following short press is unequivocal Off.

Long sw1 could be aux lights on regardless of headlight state (like during park-light operation). Any change of headlight low/high/off could revert aux lights to wherever they were when long-pressed.

Multi remains indicator brightness adjustment, need to work out persistence or not

Considering all-three multi to trigger network interaction (upload telemetry, check for and maybe download new firmware).

Double-press is an open question... got it, use it?
