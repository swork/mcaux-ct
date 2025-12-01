#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::{Spawner, task};
use embassy_rp::gpio::{Input, Level, Output, OutputOpenDrain, Pin, Pull};
use embassy_rp::pwm::{Config, Pwm};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::{Channel, DynamicSender, Receiver};
use embassy_time::{Duration, Timer};
use momentary::{MomentaryController, OUTPUTS_MAX, SWITCHES_MAX};
use {defmt_rtt as _, panic_probe as _};

/// How many receive slots for switch state-change reports.
const SWITCH_CHANNEL_DEPTH: usize = 5; // TODO: 1 should be sufficient, experiment

/// How long to wait after a switch edge to decide it's fer realz.
const DEBOUNCE: Duration = Duration::from_millis(40); // TODO configurable?

/// How long to wait, max, until deciding a close after an open isn't a double-press?
const DOUBLE_PRESS: Duration = Duration::from_millis(500);

/// How long to wait, min, until deciding a held-closed switch is a long-press?
const LONG_PRESS: Duration = Duration::from_millis(900);

///
/// Debounced switch state reporter. No races here, I promise.
/// Peek at embassy/examples/rp/bin/src/debounce.rs for a counterexample.
///
#[task]
async fn switch_state_reporter(
    idx: u8,
    mut switch: Input<'static, impl Pin>,
    sender: DynamicSender<'static, SwitchStateReport>,
) {
    let mut message: SwitchStateReport;
    let level = switch.get_level();
    loop {
        match level {
            Low => {
                switch.wait_for_high().await;
                Timer::after(DEBOUNCE).await;
                level = switch.get_level();
                if level == Level::Low {
                    continue; // bounced, don't send a message
                } else {
                    message = SwitchStateReport {
                        level: level, // high
                        swhich: idx,
                    };
                }
            }
            High => {
                switch.wait_for_low().await;
                Timer::after(DEBOUNCE).await;
                level = switch.get_level();
                if level == Level::High {
                    continue;
                } else {
                    message = SwitchStateReport {
                        level: level, // low
                        swhich: idx,
                    };
                }
            }
        }
        sender.send(message).await;
    }
}

/// The messages sent by switch_state_reporter.
#[derive(Clone, Copy)]
struct SwitchStateReport {
    /// GPIO Input level, Low for open/unpressed.
    level: Level,
    /// Which switch? (Are you a good switch, or a bad switch?)
    swhich: u8,
}

/// Entry point
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Channel for user button push events
    let mut switches_channel =
        Channel::<NoopRawMutex, SwitchStateReport, SWITCH_CHANNEL_DEPTH>::new();
    let receiver = switches_channel.get_dyn_receiver();

    // State machine tells which outputs are on as inputs change.
    // TODO Isn't this an Advisor, as we do the controlling in our loop?
    let mut switch_controller = MomentaryController::new(DOUBLE_PRESS, LONG_PRESS);

    // Three pushbuttons
    let sw0 = Input::new(p.PIN_X, Pull::Down);
    let sender = switches_channel.get_dyn_sender();
    let (sw0_idx, out0_idx) = switch_controller.add_switch(2, 1); // off/on
    spawner
        .spawn(switch_state_reporter, sw0_idx, sw0, sender)
        .expect("sw0");
    // These asserts make it safe to hard-code references to individual switches
    // below, rather than deferencing an array.
    assert!(sw0_idx == 0, "Unexpected index for sw0: {}", sw0_idx);
    assert!(out0_idx == 0, "Unexpected index for out0: {}", out0_idx);

    let sw1 = Input::new(p.PIN_Y, Pull::Down);
    let sender = switches_channel.get_dyn_sender();
    let (sw1_idx, out1_idx) = switch_controller.add_switch(2, 0); // off/on
    spawner
        .spawn(switch_state_reporter, sw1_idx, sw1, sender)
        .expect("sw1");
    assert!(sw1_idx == 1, "Unexpected index for sw1: {}", sw1_idx);
    assert!(out1_idx == 1, "Unexpected index for out1: {}", out1_idx);

    let sw2 = Input::new(p.PIN_Z, Pull::Down);
    let sender = switches_channel.get_dyn_sender();
    let (sw2_idx, out2_idx) = switch_controller.add_switch(5, 0); // off/low/lowmid/highmid/high
    spawner
        .spawn(switch_state_reporter, 2, sw0, sender)
        .expect("sw2");
    assert!(sw2_idx == 2, "Unexpected index for sw2: {}", sw2_idx);
    assert!(out2_idx == 2, "Unexpected index for out2: {}", out2_idx);

    // Fourth control: long-press of sw0 toggles out3
    let (_, out3_idx) = switch_controller.augment_switch_longpress(sw0_idx, 2, 0);
    assert!(out3_idx == 3, "Unexpected index for out3: {}", out3_idx);

    // Four driven outputs (to gate of N-channel FETs). 0, 1 and 3 are binary.
    let out0 = Output::new(p.PIN_16, Level::Low);
    let out1 = Output::new(p.PIN_17, Level::Low);
    let out3 = Output::new(p.PIN_19, Level::Low);

    // Grip-heat PWM, output #2
    let target_hz = 4u8;
    let clock_hz = embassy_rp::clocks::clk_sys_freq(); // 125 MHz std.
    let divider = 3000u16;
    let period = (clock_hz / (target_hz * divider as u32)) as u16 - 1; // 10,415
    let mut grip_heat_pwm_config = Config::default();
    grip_heat_pwm_config.top = period;
    grip_heat_pwm_config.divider = divider.into();
    let out2 = Pwm::new_output_a(p.SLICE_1, p.PIN_18, grip_heat_pwm_config);

    // PWM LEDs, one associated with each switch (but independent of switch state)
    let target_hz = 1000u16;
    // let clock_freq = embassy_rp::clocks::clk_sys_freq(); // above
    let divider = 16u8;
    let period = (clock_hz / (target_hz * divider as u32)) as u16 - 1;
    let mut c = Config::default();
    c.top = period;
    c.divider = divider.into();
    let slice4 = p.SLICE_4; // see rp2040 datasheet section 4.5.2, table 515.
    let slice5 = p.SLICE_5;
    let slice6 = p.SLICE_6;
    let mut led0 = Pwm::new_output_a(&slice4, p.PIN_4, c.clone());
    let mut led1 = Pwm::new_output_b(&slice4, p.PIN_5, c.clone());
    let mut led2 = Pwm::new_output_a(&slice5, p.PIN_6, c.clone());

    // One RGB LED.
    let mut led3r = Pwm::new_output_b(&slice5, p.PIN_7, c.clone());
    let mut led3g = Pwm::new_output_a(&slice6, p.PIN_8, c.clone());
    let mut led3b = Pwm::new_output_b(&slice6, p.PIN_9, c);

    led0.set_duty_cycle_fully_off();
    led1.set_duty_cycle_fully_off();
    led2.set_duty_cycle_fully_off();
    led3r.set_duty_cycle_fully_off();
    led3g.set_duty_cycle_fully_off();
    led3b.set_duty_cycle_fully_off();

    let indicator_channel = Channel::<NoopBlockingMutex, SystemStateUpdate, 5>::new();
    spawner.spawn_task(
        indicator_controller,
        led0,
        led1,
        led2,
        led3r,
        led3g,
        led3b,
        indicator_channel.get_dyn_receiver(),
    );
    let indicator_sender = indicator_channel.get_sender();

    let mut ins: [bool; SWITCHES_MAX] = [false; SWITCHES_MAX];
    let mut outs: [u8; OUTPUTS_MAX] = [1, 0, 0, ..]; // matches "add_switches" args

    loop {
        let notice = receiver.receive().await;
        ins[notice.swhich] = if notice.level == Level::High {
            true
        } else {
            false
        };
        let (outs, state) = switch_controller.report(ins);

        // Reflect intent to outputs (set-same doesn't affect the output state)
        out0.set_level(if outs[0] != 0 {
            Level::High
        } else {
            Level::Low
        });
        out1.set_level(if outs[1] != 0 {
            Level::High
        } else {
            Level::Low
        });
        out2.set_duty_cycle_percentage(percent_for_grip_heat_value(outs[2]));
        out3.set_level(if outs[3] != 0 {
            Level::High
        } else {
            Level::Low
        });

        indicator_sender
            .send(SystemStateUpdate {
                ins: ins.clone(),
                outs: outs.clone(),
                state: notice.state.clone(),
            })
            .await;
    }
}

enum AnimationState {}

#[derive(Clone, Copy)]
struct SystemStateUpdate {
    ins: [bool; SWITCHES_MAX],
    outs: [u8; SWITCHES_MAX],
    switch_state: SwitchState,
}

#[task]
async fn indicator_controller(
    mut led0: PwmOutput,
    mut led1: PwmOutput,
    mut led2: PwmOutput,
    mut led3r: PwmOutput,
    mut led3g: PwmOutput,
    mut led3b: PwmOutput,
    receiver: Receiver<'static, SwitchStateReport, SWITCH_CHANNEL_DEPTH>,
) {
    // Animated light changes happen at a sloppy 50Hz - we wait 1/50s
    // then do what we're going to do then wait again. Otherwise, if
    // we're not animating something, we just wait for a message.

    let prev_report: Option<SystemStateUpdate> = None;
    let animation: Option<AnimationState> = None;
    loop {
        let report = if animation.is_some() {
            // make animation changes, record new animation state
            match receiver.try_receive() {
                Ok(report) => Some(rep),
                _ => None,
            }
        } else {
            Some(receiver.receive().await)
        };
    }

    if report.is_some() {
        // If no button was down in the previous report
        // and no button is down in the current report
        // make no changes - continue animation in progress, or go to None.

        // If no button was down in the previous report and a button
        // is down now and the report's SwitchState is One start
        // AnimationOne (all rings drop quickly from current state to
        // zero then quickly come on full, then all fade toward
        // corresponding output state; and if the one down has a
        // long-press capability, fade all back up as the long-press
        // threshold approaches).

        // If a button was down in the previous report
        // and that same button is still down
        // and the report's SwitchState is One
        // make no changes - continue animation.

        // If a button was down in the previous report and the same
        // button is now up and the previous report's SwitchState was
        // One and the current report's SwitchState is None begin
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

        // BUT not today! Minimal implementation:
    }
}
