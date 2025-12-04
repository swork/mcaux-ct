#![no_std]
#![no_main]

use defmt_rtt as _;
use embassy_executor::{Spawner, task};
use embassy_futures::select::{Either, select};
use embassy_rp::Peri;
use embassy_rp::gpio::{AnyPin, Input, Level, Output, Pull};
use embassy_rp::pwm::{Config, Pwm, PwmOutput, SetDutyCycle};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use fixed::FixedU16;
use fixed::types::extra::U4;
use indicators::color_for_heat_level;
use momentary::{MomentaryController, OUTPUTS_MAX, SWITCHES_MAX, SwitchesState}; //, panic_probe as _};

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("Panic occurred: {:?}", defmt::Display2Format(info));
    // whatever else to do
    loop {} // Halt the program
}

/// How many receive slots for inter-task Channels
const SWITCH_CHANNEL_DEPTH: usize = 5; // TODO: 1 should be sufficient, experiment
const INDICATOR_CHANNEL_DEPTH: usize = 5;

/// How long to wait after a switch edge to decide it's fer realz.
const DEBOUNCE: Duration = Duration::from_millis(40); // TODO configurable?

/// How long to wait, max, until deciding a close after an open isn't a double-press?
const DOUBLE_PRESS: Duration = Duration::from_millis(500);

/// How long to wait, min, until deciding a held-closed switch is a long-press?
const LONG_PRESS: Duration = Duration::from_millis(900);

/// Channel for driving indicators, and its receiver
static INDICATOR_CHANNEL: Channel<
    CriticalSectionRawMutex,
    SystemStateUpdate,
    INDICATOR_CHANNEL_DEPTH,
> = Channel::new();
static SWITCHES_CHANNEL: Channel<CriticalSectionRawMutex, SwitchStateReport, SWITCH_CHANNEL_DEPTH> =
    Channel::new();

/// Switch index values, from MomentaryController's .add... functions
/// These map to GPIO inputs (.usb and .gripheat have additional
/// long-press functions, and highbeam tracks the actual headlight
/// beam switch).
#[derive(Copy, Clone, Debug, Default)]
struct SwitchIdxs {
    usb: usize,
    auxlights: usize,
    gripheat: usize,
    highbeam: usize,
}

/// Output index values, from MomentaryController's .add... functions
/// These do NOT map directly to GPIO outputs - auxlights only come on
/// when enabled AND highbeam is on.
#[derive(Copy, Clone, Debug, Default)]
struct OutputIdxs {
    usb: usize,
    auxlights: usize, // enable. AND with .highbeam for actual output state
    gripheat: usize,
    nav: usize,
    highbeam: usize,
}

/// Collected index values from MomentaryController's
/// .add... functions, for indexing into its inputs and reports.
#[derive(Copy, Clone, Debug, Default)]
struct Idx {
    sw: SwitchIdxs,
    out: OutputIdxs,
}

///
/// Debounced switch state reporter. No races here, I promise.
/// Peek at embassy/examples/rp/bin/src/debounce.rs for a counterexample.
///
#[task(pool_size = 4)]
async fn switch_state_reporter(idx: usize, pin: Peri<'static, AnyPin>) {
    let sender = SWITCHES_CHANNEL.dyn_sender();
    let mut switch = Input::new(pin, Pull::Down);
    let mut level = switch.get_level();

    loop {
        let message = match level {
            Level::Low => {
                switch.wait_for_high().await;
                Timer::after(DEBOUNCE).await;
                level = switch.get_level();
                if level == Level::Low {
                    continue; // bounced, don't send a message
                } else {
                    SwitchStateReport {
                        level, // high
                        swhich: idx,
                    }
                }
            }
            Level::High => {
                switch.wait_for_low().await;
                Timer::after(DEBOUNCE).await;
                level = switch.get_level();
                if level == Level::High {
                    continue;
                } else {
                    SwitchStateReport {
                        level, // low
                        swhich: idx,
                    }
                }
            }
        };
        sender.send(message).await;
    }
}

/// The messages sent by switch_state_reporter.
#[derive(Clone, Copy)]
struct SwitchStateReport {
    /// GPIO Input level, Low for open/unpressed.
    level: Level,
    /// Which switch? (Are you a good switch, or a bad switch?)
    swhich: usize,
}

/// Entry point
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    defmt::trace!("Startup");

    // State machine tells which outputs are on as inputs change.
    // TODO Isn't this an Advisor, as we do the controlling in our loop?
    let mut switch_controller = MomentaryController::new(DOUBLE_PRESS, LONG_PRESS);
    let mut idx: Idx = Default::default(); // collect idx values

    // Three pushbuttons and the high-beam follower
    (idx.sw.usb, idx.out.usb) = switch_controller.add_switch(2, 1); // off/on
    spawner.spawn(
        switch_state_reporter(idx.sw.usb, p.PIN_12.into())
            .expect("spawn switch_state_reporter for USB power on pin 12"),
    );

    (idx.sw.auxlights, idx.out.auxlights) = switch_controller.add_switch(2, 0); // off/on
    spawner.spawn(
        switch_state_reporter(idx.sw.auxlights, p.PIN_13.into())
            .expect("spawn switch_state_reporter for auxlights on pin 13"),
    );

    (idx.sw.gripheat, idx.out.gripheat) = switch_controller.add_switch(5, 0); // off/low/lowmid/highmid/high
    spawner.spawn(
        switch_state_reporter(idx.sw.gripheat, p.PIN_14.into())
            .expect("spawn switch_state_reporter for grip heat on pin 14"),
    );

    // Auxiliary lights only come on with high beams when enabled
    (idx.sw.highbeam, idx.out.highbeam) = switch_controller.add_switch_momentary();
    spawner.spawn(
        switch_state_reporter(idx.sw.highbeam, p.PIN_15.into())
            .expect("spawn switch_state_reporter for highbeam on pin 15"),
    );

    // Fourth control: long-press of usb toggles nav
    (_, idx.out.nav) = switch_controller.augment_switch_longpress_add_output(idx.sw.usb, 2, 0);

    // Fifth control: long-press of gripheat jumps grip heat output to HIGH.
    let (_, _) =
        switch_controller.augment_switch_longpress_max_output(idx.sw.gripheat, idx.out.gripheat);

    // Four driven outputs (to gate of N-channel FETs). These three are binary
    let mut outio_usb = Output::new(p.PIN_16, Level::Low);
    let mut outio_auxlights = Output::new(p.PIN_17, Level::Low);
    let mut outio_nav = Output::new(p.PIN_19, Level::Low);

    // common to all PWM setups
    let clock_hz = embassy_rp::clocks::clk_sys_freq(); // 125 MHz std.

    // Grip-heat, the fourth output, is PWM
    let target_hz = 8u32;
    let divider = 255u32; // 8.4 fractional
    let period = (clock_hz / (target_hz * divider)) as u16 - 1; // 61_274
    let mut grip_heat_pwm_config = Config::default();
    grip_heat_pwm_config.top = period;
    grip_heat_pwm_config.divider = FixedU16::<U4>::from_num(divider);
    let mut outpwm_gripheat = Pwm::new_output_a(p.PWM_SLICE1, p.PIN_18, grip_heat_pwm_config);

    // Six PWM LEDs, one associated with each switch (but independent of switch state).
    let target_hz = 1000u32;
    let divider = 16u32;
    let period = (clock_hz / (target_hz * divider)) as u16 - 1;
    let mut c = Config::default();
    c.top = period;
    c.divider = FixedU16::<U4>::from_num(divider);
    // PWM_SLICEx: see rp2040 datasheet section 4.5.2, table 515.
    //
    //  Pwm::new_output_a (or _b I bet) blows up. Dunno why. _ab and then split()
    // works, and retvals are PwmOutput. Seems like I'm missing something.
    // Try to avoid loner pins here, but out2 is TBD.

    let pwm = Pwm::new_output_ab(p.PWM_SLICE2, p.PIN_4, p.PIN_5, c.clone());
    let (led0, led1) = pwm.split();
    let mut led0 = led0.expect("split slice2a");
    let mut led1 = led1.expect("split slice2b");
    c.invert_b = true;
    let pwm = Pwm::new_output_ab(p.PWM_SLICE3, p.PIN_6, p.PIN_7, c.clone());
    let (led2, led3r) = pwm.split();
    let mut led2 = led2.expect("split slice3a");
    let mut led3r = led3r.expect("split slice3b");
    c.invert_a = true;
    let pwm = Pwm::new_output_ab(p.PWM_SLICE4, p.PIN_8, p.PIN_9, c.clone());
    let (led3g, led3b) = pwm.split();
    let mut led3g = led3g.expect("split slice4a");
    let mut led3b = led3b.expect("split slice4b");

    led0.set_duty_cycle_percent(1).expect("sdc0");
    led1.set_duty_cycle_percent(2).expect("sdc1");
    led2.set_duty_cycle_percent(3).expect("sdc2");
    led3r.set_duty_cycle_percent(4).expect("sdc3r");
    led3g.set_duty_cycle_percent(5).expect("sdc3g");
    led3b.set_duty_cycle_percent(6).expect("sdc3b");

    spawner.spawn(
        indicator_controller(idx, led0, led1, led2, led3r, led3g, led3b)
            .expect("spawn indicator_controller"),
    );

    let switches_receiver = SWITCHES_CHANNEL.dyn_receiver();
    let indicator_sender = INDICATOR_CHANNEL.sender();
    let mut ins: [bool; SWITCHES_MAX] = [false; SWITCHES_MAX];
    loop {
        let (outs, switches_state) = switch_controller.report(ins);

        indicator_sender
            .send(SystemStateUpdate {
                _ins: ins,
                outs,
                _switches_state: switches_state,
            })
            .await; // queuing only - not waiting for receipt.

        // Reflect intent to outputs (set-same doesn't affect the output state)
        outio_usb.set_level(if outs[idx.out.usb] != 0 {
            Level::High
        } else {
            Level::Low
        });
        outio_auxlights.set_level(
            if outs[idx.out.auxlights] != 0 && outs[idx.out.highbeam] != 0 {
                Level::High
            } else {
                Level::Low
            },
        );
        outio_nav.set_level(if outs[idx.out.nav] != 0 {
            Level::High
        } else {
            Level::Low
        });

        outpwm_gripheat
            .set_duty_cycle_percent(percent_for_grip_heat_value(outs[idx.out.gripheat]))
            .expect("grip duty cycle");

        match switches_state {
            SwitchesState::None => {
                let notice = switches_receiver.receive().await;
                ins[notice.swhich] = notice.level == Level::High;
            } // wait for next press
            _ => {
                // Impatient mode: wait only 20mS for next press/release.
                // Otherwise we miss transition from One to Long.
                match select(
                    switches_receiver.receive(),
                    Timer::after(Duration::from_millis(20)),
                )
                .await
                {
                    Either::First(notice) => {
                        ins[notice.swhich] = notice.level == Level::High;
                    }
                    _ => (),
                };
            }
        };

        // And around the loop. Processing switch notices is at the top to establish initial conditions.
    }
}

fn percent_for_grip_heat_value(val: u8) -> u8 {
    match val {
        0 => 0,
        1 => 25,
        2 => 50,
        3 => 75,
        4 => 100,
        _ => 0, // val should range 0..=4 so this should probably panic TODO
    }
}

enum AnimationState {}

#[derive(Clone, Copy)]
struct SystemStateUpdate {
    _ins: [bool; SWITCHES_MAX],
    outs: [u8; OUTPUTS_MAX],
    _switches_state: SwitchesState,
}

#[task]
async fn indicator_controller(
    idx: Idx,
    mut led0: PwmOutput<'static>,
    mut led1: PwmOutput<'static>,
    mut led2: PwmOutput<'static>,
    mut led3r: PwmOutput<'static>,
    mut led3g: PwmOutput<'static>,
    mut led3b: PwmOutput<'static>,
) {
    let receiver = INDICATOR_CHANNEL.dyn_receiver();

    // Animated light changes happen at a sloppy 50Hz - we wait 1/50s
    // then do what we're going to do then wait again. Otherwise, if
    // we're not animating something, we just wait for a message.

    let _prev_report: Option<SystemStateUpdate> = None;
    let animation: Option<AnimationState> = None;
    loop {
        let report = if animation.is_some() {
            // make animation changes, record new animation state
            receiver.try_receive().ok()
        } else {
            Some(receiver.receive().await)
        };

        match report {
            None => (),
            Some(rep) => {
                // Minimal:
                if rep.outs[idx.out.usb] != 0 {
                    led0.set_duty_cycle_percent(100).unwrap();
                } else {
                    led0.set_duty_cycle_percent(0).unwrap();
                }
                if rep.outs[idx.out.auxlights] != 0 {
                    led1.set_duty_cycle_percent(100).unwrap();
                } else {
                    led1.set_duty_cycle_percent(0).unwrap();
                }
                if rep.outs[idx.out.gripheat] != 0 {
                    led2.set_duty_cycle_percent(100).unwrap();
                    let rgb: [u8; 3] = color_for_heat_level(rep.outs[2]);
                    // scale full-range u8 to percent
                    let pct: u16 = rgb[0] as u16 * 100 / 256;
                    led3r
                        .set_duty_cycle_percent(pct.try_into().unwrap())
                        .unwrap();
                    let pct: u16 = rgb[1] as u16 * 100 / 256;
                    led3g
                        .set_duty_cycle_percent(pct.try_into().unwrap())
                        .unwrap();
                    let pct: u16 = rgb[2] as u16 * 100 / 256;
                    led3b
                        .set_duty_cycle_percent(pct.try_into().unwrap())
                        .unwrap();
                } else {
                    led2.set_duty_cycle_percent(0).unwrap();
                    led3r.set_duty_cycle_percent(0).unwrap();
                    led3g.set_duty_cycle_percent(0).unwrap();
                    led3b.set_duty_cycle_percent(0).unwrap();
                }

                // TODO:
                // If no button was down in the previous report
                // and no button is down in the current report
                // make no changes - continue animation in progress, or go to None.

                // If no button was down in the previous report and a button
                // is down now and the report's SwitchesState is One start
                // AnimationOne (all rings drop quickly from current state to
                // zero then quickly come on full, then all fade toward
                // corresponding output state; and if the one down has a
                // long-press capability, fade all back up as the long-press
                // threshold approaches).

                // If a button was down in the previous report
                // and that same button is still down
                // and the report's SwitchesState is One
                // make no changes - continue animation.

                // If a button was down in the previous report and the same
                // button is now up and the previous report's SwitchesState was
                // One and the current report's SwitchesState is None begin
                // AnimationNone from current levels: fade all toward output
                // state indication, wait a few seconds, then dim the bright
                // ones to their resting On levels.

                // If a button was down in the previous report and the state
                // was One, and the same button is still down and the state is
                // now Long, begin AnimationLong (all rings hard-blink several
                // times quickly, then all but the down button go immediately
                // to their dimmed resting state; the down button continues to
                // blink quickly on the same cadence indefinitely)

                // If a button was down previously and the state was Long and
                // that button is now up and the state is None, continue
                // blinking the button's ring on the same cadence for one more
                // second, then end by immediately putting that ring into the
                // dimmed resting state corresponding to its primary output.
            }
        }
    }
}
