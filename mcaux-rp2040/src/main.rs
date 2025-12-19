#![no_std]
#![no_main]

use defmt_serial as _;
use embassy_executor::{Spawner, task};
use embassy_futures::select::{Either, select};
// use embassy_net::{Stack, StackResources, Config, dhcpv4::Dhcpv4Client};
use embassy_rp::Peri;
use embassy_rp::gpio::{AnyPin, Input, Level, Output, Pull};
use embassy_rp::pwm;
use embassy_rp::pwm::{Pwm, PwmOutput, SetDutyCycle};
use embassy_rp::uart;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use fixed::FixedU16;
use fixed::types::extra::U4;
use mcaux_indicators::{IndicatorController, LedsSituation};
use momentary::{AbstractInput, SwitchOutputController, SwitchesState};
use static_cell::StaticCell;

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
    Option<SwitchOutputController>,
    INDICATOR_CHANNEL_DEPTH,
> = Channel::new();

static SWITCHES_CHANNEL: Channel<
    CriticalSectionRawMutex,
    AbstractInput,
    SWITCH_CHANNEL_DEPTH,
> = Channel::new();

/// The indicators hardware interfaces
struct IndicatorsInstances {
    usb: PwmOutput<'static>,
    auxlight: PwmOutput<'static>,
    gripheat: PwmOutput<'static>,
    rgb_r: PwmOutput<'static>,
    rgb_g: PwmOutput<'static>,
    rgb_b: PwmOutput<'static>,
}

///
/// Debounced switch state reporter. No races here, I promise.
/// Peek at embassy/examples/rp/bin/src/debounce.rs for a counterexample.
///
#[task(pool_size = 4)]
async fn switch_state_observer(
    abstract_input: AbstractInput,
    pin: Peri<'static, AnyPin>,
) {
    let sender = SWITCHES_CHANNEL.dyn_sender();
    let mut switch = Input::new(pin, Pull::Down);
    let mut level = switch.get_level();

    loop {
        let message = match level {
            Level::Low => {
                defmt::trace!("waiting for high");
                switch.wait_for_high().await;
                Timer::after(DEBOUNCE).await;
                level = switch.get_level();
                if level == Level::Low {
                    defmt::trace!("nope, bounce");
                    continue; // bounced, don't send a message
                } else {
                    defmt::trace!("have high");
                    AbstractInput::new(true, abstract_input)
                }
            }
            Level::High => {
                defmt::trace!("waiting for low");
                switch.wait_for_low().await;
                Timer::after(DEBOUNCE).await;
                level = switch.get_level();
                if level == Level::High {
                    defmt::trace!("nope, bounce");
                    continue;
                } else {
                    defmt::trace!("have low");
                    AbstractInput::new(false, abstract_input)
                }
            }
        };
        sender.send(message).await;
    }
}

/// Entry point. Initialize hardware and abstract state machines
/// representing I/O elements and indicators, then loop mediating
/// between them.
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // serial trace output - sidestep unresolved RTT woes
    let mut config = uart::Config::default();
    config.baudrate = 38400; // default for my TTL/serial donglen
    let mut uart = uart::Uart::new_blocking(p.UART0, p.PIN_0, p.PIN_1, config);
    //    let result = defmt_serial::defmt_serial(SERIAL.init(uart));
    defmt::warn!("Startup");
    let _ = uart.blocking_write(b"!");

    // State machine tells which outputs are on as inputs change.
    // TODO Isn't this an Advisor, as we do the controlling in our loop?
    let mut switch_controller = SwitchOutputController::new(DOUBLE_PRESS, LONG_PRESS);

    // Three pushbuttons and the high-beam follower
    let (sw_usb_i, out_usb_i) = switch_controller.add_switch("usb", 2, 1); // off/on
    spawner.spawn(
        switch_state_observer(switch_controller.switch[sw_usb_i], p.PIN_12.into())
            .expect("spawn switch_state_observer for USB power on pin 12"),
    );

    let (sw_auxlight_i, out_auxlight_i) = switch_controller.add_switch("auxlight", 2, 0); // off/on
    spawner.spawn(
        switch_state_observer(switch_controller.switch[sw_auxlight_i], p.PIN_13.into())
            .expect("spawn switch_state_observer for auxlights on pin 13"),
    );

    let (sw_gripheat_i, out_gripheat_i) = switch_controller.add_switch("gripheat", 5, 0); // off/low/lowmid/highmid/high
    spawner.spawn(
        switch_state_observer(switch_controller.switch[sw_gripheat_i], p.PIN_14.into())
            .expect("spawn switch_state_observer for grip heat on pin 14"),
    );

    // Auxiliary lights only come on with high beams when enabled
    let (sw_highbeam_i, out_highbeam_i) = switch_controller.add_switch_momentary("highbeam");
    spawner.spawn(
        switch_state_observer(switch_controller.switch[sw_highbeam_i], p.PIN_15.into())
            .expect("spawn switch_state_observer for highbeam on pin 15"),
    );

    // Fourth control: long-press of usb toggles nav
    let (_, out_nav_i) =
        switch_controller.augment_switch_longpress_add_output(sw_usb_i, "nav", 2, 0);

    // Fifth control: long-press of gripheat jumps grip heat output to HIGH.
    let _sw_gripheat_i =
        switch_controller.augment_switch_longpress_max_output(sw_gripheat_i, out_gripheat_i);

    // Four driven outputs (to gate of N-channel FETs). These three are binary
    let mut outio_usb = Output::new(p.PIN_16, Level::Low);
    let mut outio_auxlight = Output::new(p.PIN_17, Level::Low);
    let mut outio_nav = Output::new(p.PIN_19, Level::Low);

    // common to all PWM setups
    let clock_hz = embassy_rp::clocks::clk_sys_freq(); // 125 MHz std.

    // Grip-heat, the fourth output, is PWM
    let target_hz = 8u32;
    let divider = 255u32; // 8.4 fractional
    let period = (clock_hz / (target_hz * divider)) as u16 - 1; // 61_274
    let mut grip_heat_pwm_config = pwm::Config::default();
    grip_heat_pwm_config.top = period;
    grip_heat_pwm_config.divider = FixedU16::<U4>::from_num(divider);
    let mut outpwm_gripheat = Pwm::new_output_a(p.PWM_SLICE1, p.PIN_18, grip_heat_pwm_config);

    // Six PWM LEDs, one associated with each switch (but independent of switch state).
    let target_hz = 1000u32;
    let divider = 16u32;

    // Correctly we should subtract one, but this complicates math
    // elsewhere. Leaving this number unadjusted puts our max duty
    // cycle just below absolutely fully on, but we won't be using
    // that value anyway (the LEDs don't look brighter at the highest
    // settings).
    let period = (clock_hz / (target_hz * divider)) as u16;

    let mut c = pwm::Config::default();
    c.top = period;
    assert_eq!(period, 7812); // MARK_THIS_LINE_FOR_BRIGHTNESS_LOOKUP
    c.divider = FixedU16::<U4>::from_num(divider);
    // PWM_SLICEx: see rp2040 datasheet section 4.5.2, table 515.
    //
    //  Pwm::new_output_a (or _b I bet) blows up. Dunno why. _ab and then split()
    // works, and retvals are PwmOutput. Seems like I'm missing something.
    // Try to avoid loner pins here, but out2 is TBD.

    let pwm = Pwm::new_output_ab(p.PWM_SLICE2, p.PIN_4, p.PIN_5, c.clone());
    let (led0, led1) = pwm.split();
    let led0 = led0.expect("split slice2a");
    let led1 = led1.expect("split slice2b");
    c.invert_b = true;
    let pwm = Pwm::new_output_ab(p.PWM_SLICE3, p.PIN_6, p.PIN_7, c.clone());
    let (led2, led3r) = pwm.split();
    let led2 = led2.expect("split slice3a");
    let led3r = led3r.expect("split slice3b");
    c.invert_a = true;
    let pwm = Pwm::new_output_ab(p.PWM_SLICE4, p.PIN_8, p.PIN_9, c.clone());
    let (led3g, led3b) = pwm.split();
    let led3g = led3g.expect("split slice4a");
    let led3b = led3b.expect("split slice4b");

    let indicators = IndicatorsInstances {
        usb: led0,
        auxlight: led1,
        gripheat: led2,
        rgb_r: led3r,
        rgb_g: led3g,
        rgb_b: led3b,
    };

    let indicator_controller = IndicatorController::new(
        indicators.usb.max_duty_cycle(),
        indicators.auxlight.max_duty_cycle(),
        indicators.gripheat.max_duty_cycle(),
        indicators.rgb_r.max_duty_cycle(),
        indicators.rgb_g.max_duty_cycle(),
        indicators.rgb_b.max_duty_cycle(),
    );

    spawner.spawn(
        indicator_handler(indicators, indicator_controller).expect("spawn indicator_handler"),
    );

    let switches_receiver = SWITCHES_CHANNEL.dyn_receiver();
    let indicator_sender = INDICATOR_CHANNEL.sender();

    loop {
        switch_controller.remap();

        // Reflect model to output hardware
        outio_usb.set_level(if switch_controller.output[out_usb_i].value != 0 {
            Level::High
        } else {
            Level::Low
        });
        outio_auxlight.set_level(
            if switch_controller.output[out_auxlight_i].value != 0
                && switch_controller.output[out_highbeam_i].value != 0
            {
                Level::High
            } else {
                Level::Low
            },
        );
        outio_nav.set_level(if switch_controller.output[out_nav_i].value != 0 {
            Level::High
        } else {
            Level::Low
        });
        outpwm_gripheat
            .set_duty_cycle_percent(percent_for_grip_heat_value(
                switch_controller.output[out_gripheat_i].value,
            ))
            .expect("grip duty cycle");

        // Update the indicators.
        indicator_sender.send(Some(switch_controller.clone())).await;

        // Get a switch state update from one of the hardware
        // observers, or timeout without one
        let abstract_input_update = match switch_controller.switches_state {
            SwitchesState::None => switches_receiver.receive().await,
            _ => {
                // Impatient mode: wait only 20mS for next press/release.
                // Otherwise we miss transition from One to Long.
                match select(
                    switches_receiver.receive(),
                    Timer::after(Duration::from_millis(20)),
                )
                .await
                {
                    Either::First(notice) => notice,
                    _ => continue,
                }
            }
        };

        // Update the model with the new switch state
        let idx = abstract_input_update.idx;
        let isclosed = abstract_input_update.isclosed;
        switch_controller.switch[idx].isclosed = isclosed;

        // And around the loop. Processing switch changes is at the top to establish initial conditions.
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

#[task]
async fn indicator_handler(
    mut indicators_instances: IndicatorsInstances,
    mut indicator_controller: IndicatorController,
) {
    let receiver = INDICATOR_CHANNEL.dyn_receiver();

    // Animated light changes happen at a sloppy 50Hz - we wait 1/50s
    // then do what we're going to do then wait again. Otherwise, if
    // we're not animating something, we just wait for a message.

    let mut next_update: Option<Duration> = None;
    let mut leds: Option<LedsSituation> = None;
    loop {
        // set indicators
        match leds {
            None => (),
            Some(situation) => {
                // Minimal:
                indicators_instances
                    .usb
                    .set_duty_cycle(situation.usb)
                    .unwrap();
                indicators_instances
                    .auxlight
                    .set_duty_cycle(situation.auxlight)
                    .unwrap();
                indicators_instances
                    .gripheat
                    .set_duty_cycle(situation.gripheat)
                    .unwrap();
                indicators_instances
                    .rgb_r
                    .set_duty_cycle(situation.rgb_r)
                    .unwrap();
                indicators_instances
                    .rgb_g
                    .set_duty_cycle(situation.rgb_g)
                    .unwrap();
                indicators_instances
                    .rgb_b
                    .set_duty_cycle(situation.rgb_b)
                    .unwrap();
            }
        }

        (leds, next_update) = match next_update {
            None => {
                let switch_situation = receiver.receive().await;
                indicator_controller.cycle(switch_situation)
            }
            Some(when) => match select(Timer::after(when), receiver.receive()).await {
                Either::First(_) => indicator_controller.cycle(None),
                Either::Second(switch_situation) => indicator_controller.cycle(switch_situation),
            },
        };
    }
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
