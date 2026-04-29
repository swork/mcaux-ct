use assign_resources::assign_resources;
use defmt::*;
use embassy_executor::{Spawner, task};
use embassy_futures::select::{Either, select};
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::pwm::{Pwm, PwmOutput, SetDutyCycle};
use embassy_rp::{Peri, peripherals, pwm};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Sender};
use embassy_time::{Duration, Timer};
use fixed::FixedU16;
use fixed::types::extra::U4;
use mcaux_indicators::{IndicatorController, LedsSituation};
use momentary::{AbstractInput, SwitchOutputController, SwitchesState};

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

static SWITCHES_CHANNEL: Channel<CriticalSectionRawMutex, AbstractInput, SWITCH_CHANNEL_DEPTH> =
    Channel::new();

/// The indicators hardware interfaces
struct IndicatorsInstances {
    usb: PwmOutput<'static>,
    auxlight: PwmOutput<'static>,
    gripheat: PwmOutput<'static>,
    rgb_r: PwmOutput<'static>,
    rgb_g: PwmOutput<'static>,
    rgb_b: PwmOutput<'static>,
}

struct SwitchActive {
    active_level: Level,
}

///
/// Is a switch whose input is a given level Active, true or false?
///
/// assert!(SwitchActive(true).active(true), true);
/// assert!(SwitchActive(true).active(false), false);
/// assert!(SwitchActive(false).active(true), false);
/// assert!(SwitchActive(false).active(false), true);
///
impl SwitchActive {
    pub fn new(active_level: Level) -> Self {
        Self { active_level }
    }

    pub fn active(&self, level: Level) -> bool {
        level == self.active_level
    }

    /// Block until input changes state to active, or not active
    pub async fn wait_for(&self, sw: &mut Input<'static>, active: bool) -> () {
        if sw.get_level() == Level::Low {
            if active {
                sw.wait_for_low().await;
            } else {
                sw.wait_for_high().await;
            }
        } else if active {
            sw.wait_for_high().await;
        } else {
            sw.wait_for_low().await;
        }
    }
}

///
/// Debounced switch state reporter. No races here, I promise.
/// Peek at embassy/examples/rp/bin/src/debounce.rs for a counterexample.
///
#[task(pool_size = 4)]
async fn switch_state_observer(
    abstract_input: AbstractInput,
    mut sw: Input<'static>,
    active_level: Level,
) -> ! {
    let sender = SWITCHES_CHANNEL.dyn_sender();
    let switch = SwitchActive::new(active_level);
    let mut active = switch.active(sw.get_level());

    loop {
        switch.wait_for(&mut sw, !active).await;
        Timer::after(DEBOUNCE).await;
        let message = if active != switch.active(sw.get_level()) {
            active = !active;
            AbstractInput::new(active, abstract_input)
        } else {
            continue; // glitched, no actual change, don't send a message
        };
        sender.send(message).await;
    }
}

/// Report to outermost task: aliveness; do telemetry/DFU and reset
pub enum Telemetry {
    Alive,
    Update,
}

pub const TELEMETRY_CHANNEL_DEPTH: usize = 1;

assign_resources! {
    switching: SwitchingResources {
        led_blinker: PIN_2,
        /*
        pin_uart_tx: PIN_0,
        pin_uart_rx: PIN_1,
        uart: UART0,
        */
        sw_usb: PIN_20,
        sw_aux: PIN_21,
        sw_grp: PIN_22,
        sw_hbm: PIN_26,
        led_usb: PIN_4,
        led_aux: PIN_5,
        pwm_usb_aux: PWM_SLICE2,
        led_grp: PIN_6,
        led_r: PIN_7,
        pwm_grp_r: PWM_SLICE3,
        led_g: PIN_8,
        led_b: PIN_9,
        pwm_g_b: PWM_SLICE4,
        out_usb: PIN_16,
        out_aux: PIN_17,
        out_grp: PIN_18,
        out_nav: PIN_19,
        pwm_outgrp: PWM_SLICE1,
        _one_wire: PIN_10,
    }
}

#[embassy_executor::task]
pub async fn main_rp(
    spawner: Spawner,
    p: SwitchingResources,
    tc: Sender<'static, CriticalSectionRawMutex, Telemetry, TELEMETRY_CHANNEL_DEPTH>,
) -> () {
    let mut blinker = Output::new(p.led_blinker, Level::High);

    /*
        // serial trace output - sidestep unresolved RTT woes
        let mut config = uart::Config::default();
        config.baudrate = 38400; // default for my TTL/serial donglen
        let mut uart = uart::Uart::new_blocking(p.uart, p.pin_uart_tx, p.pin_uart_rx, config);
        //    let result = defmt_serial::defmt_serial(SERIAL.init(uart));
        let _ = uart.blocking_write(b"!");
    */

    // State machine tells which outputs are on as inputs change.
    // TODO Isn't this an Advisor, as we do the controlling in our loop?
    let mut switch_controller = SwitchOutputController::new(DOUBLE_PRESS, LONG_PRESS);

    // Three pushbuttons and the high-beam follower
    let (sw_usb_i, out_usb_i) = switch_controller.add_switch("usb", 2, 1); // off/on
    spawner.spawn(
        switch_state_observer(
            switch_controller.switch[sw_usb_i],
            Input::new(p.sw_usb, Pull::Up),
            Level::Low,
        )
        .expect("usb switch_state_observer "),
    );

    let (sw_auxlight_i, out_auxlight_i) = switch_controller.add_switch("auxlight", 2, 0); // off/on
    spawner.spawn(
        switch_state_observer(
            switch_controller.switch[sw_auxlight_i],
            Input::new(p.sw_aux, Pull::Up),
            Level::Low,
        )
        .expect("auxlight switch_state_observer"),
    );

    let (sw_gripheat_i, out_gripheat_i) = switch_controller.add_switch("gripheat", 5, 0); // off/low/lowmid/highmid/high
    spawner.spawn(
        switch_state_observer(
            switch_controller.switch[sw_gripheat_i],
            Input::new(p.sw_grp, Pull::Up),
            Level::Low,
        )
        .expect("gripheat switch_state_observer"),
    );

    // Auxiliary lights only come on with high beams when enabled
    let (sw_highbeam_i, out_highbeam_i) = switch_controller.add_switch_momentary("highbeam");
    spawner.spawn(
        switch_state_observer(
            switch_controller.switch[sw_highbeam_i],
            Input::new(p.sw_hbm, Pull::Down),
            Level::High,
        )
        .expect("highbeam switch_state_observer"),
    );

    // Fourth control: long-press of usb toggles nav
    let (_, out_nav_i) =
        switch_controller.augment_switch_longpress_add_output(sw_usb_i, "nav", 2, 0);

    // Fifth control: long-press of gripheat jumps grip heat output to HIGH.
    let _sw_gripheat_i =
        switch_controller.augment_switch_longpress_max_output(sw_gripheat_i, out_gripheat_i);

    // Four driven outputs (to gate of N-channel FETs). These three are binary
    let mut outio_usb = Output::new(p.out_usb, Level::Low);
    let mut outio_auxlight = Output::new(p.out_aux, Level::Low);
    let mut outio_nav = Output::new(p.out_nav, Level::Low);

    // common to all PWM setups
    let clock_hz = embassy_rp::clocks::clk_sys_freq();

    // Grip-heat, the fourth output, is PWM
    let target_hz = 8u32;
    let divider = 255u32; // 8.4 fractional
    let period = (clock_hz / (target_hz * divider)) as u16 - 1; // 61_274
    let mut grip_heat_pwm_config = pwm::Config::default();
    grip_heat_pwm_config.top = period;
    grip_heat_pwm_config.divider = FixedU16::<U4>::from_num(divider);
    let mut outpwm_gripheat = Pwm::new_output_a(p.pwm_outgrp, p.out_grp, grip_heat_pwm_config);

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

    // assert_eq!(period, 7812); // MARK_THIS_LINE_FOR_BRIGHTNESS_LOOKUP
    c.divider = FixedU16::<U4>::from_num(divider);

    let pwm = Pwm::new_output_ab(p.pwm_usb_aux, p.led_usb, p.led_aux, c.clone());
    let (led0, led1) = pwm.split();
    let led0 = led0.expect("split slice2a");
    let led1 = led1.expect("split slice2b");
    let pwm = Pwm::new_output_ab(p.pwm_grp_r, p.led_grp, p.led_r, c.clone());
    let (led2, led3r) = pwm.split();
    let led2 = led2.expect("split slice3a");
    let led3r = led3r.expect("split slice3b");
    let pwm = Pwm::new_output_ab(p.pwm_g_b, p.led_g, p.led_b, c.clone());
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
        indicator_handler(indicators, indicator_controller).expect("spawn indicator handler"),
    );

    let switches_receiver = SWITCHES_CHANNEL.dyn_receiver();
    let indicator_sender = INDICATOR_CHANNEL.sender();
    let mut one_full_loop_completed = false;

    loop {
        info!("Top of loop");

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
        blinker.set_low();

        // Get a switch state update from one of the hardware
        // observers, or timeout without one
        let abstract_input_update = match switch_controller.switches_state {
            SwitchesState::None => {
                match select(
                    switches_receiver.receive(),
                    Timer::after(Duration::from_millis(5000)),
                )
                .await
                {
                    Either::First(notice) => Some(notice),
                    _ => None,
                }
            }
            _ => {
                // Impatient mode: wait only 20mS for next press/release.
                // Otherwise we miss transition from One to Long.
                match select(
                    switches_receiver.receive(),
                    Timer::after(Duration::from_millis(20)),
                )
                .await
                {
                    Either::First(notice) => Some(notice),
                    _ => None,
                }
            }
        };

        blinker.set_high();
        if one_full_loop_completed {
            tc.send(Telemetry::Alive).await; // watchdog management, and marks new firmware Okay
        }
        one_full_loop_completed = true;

        // timeout case
        if abstract_input_update.is_none() {
            continue;
        }

        // Else update the model with the new switch state
        let abstract_input_update = abstract_input_update.unwrap();
        let idx = abstract_input_update.idx;
        let isclosed = abstract_input_update.isclosed;
        switch_controller.switch[idx].isclosed = isclosed;

        // TEMP but probably for a long time: If all three switches are down,
        // trigger telemetry update and DFU
        if switch_controller.switch[sw_usb_i].isclosed
            && switch_controller.switch[sw_auxlight_i].isclosed
            && switch_controller.switch[sw_gripheat_i].isclosed
        {
            info!("All three buttons are pushed, trigger comms");
            tc.send(Telemetry::Update).await;
        }

        // And around the loop. Processing switch changes is at the
        // top to establish initial conditions.
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
) -> ! {
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
