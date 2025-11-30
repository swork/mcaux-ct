#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio;
use embassy_time::Timer;
use gpio::{Level, Output, OutputOpenDrain};
use {defmt_rtt as _, panic_probe as _};

use embassy_time::{Duration};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let out0 = Output::new(p.PIN_16, Level::Low);
    let out1 = Output::new(p.PIN_17, Level::Low);
    let out2 = Output::new(p.PIN_18, Level::Low);
    let out3 = Output::new(p.PIN_19, Level::Low);

    let led0 = Output::new(p.PIN_4, Level::Low);
    let led1 = Output::new(p.PIN_5, Level::Low);
    let led2 = Output::new(p.PIN_6, Level::Low);
    let led3r = OutputOpenDrain::new(p.PIN_7, Level::High);
    let led3g = OutputOpenDrain::new(p.PIN_8, Level::High);
    let led3b = OutputOpenDrain::new(p.PIN_9, Level::High);

    let mut outs = [out0, out1, out2, out3,
		    led0, led1, led2];
    let mut rgbs = [led3r, led3g, led3b];
    
    loop {
	for out in &mut outs {
            out.set_high();
            Timer::after(Duration::from_millis(300)).await;
	}
	for rgb in &mut rgbs {
	    rgb.set_low();
            Timer::after(Duration::from_millis(300)).await;
	}
	for out in &mut outs {
	    out.set_low();
            Timer::after(Duration::from_millis(300)).await;
	}
	for rgb in &mut rgbs {
	    rgb.set_high();
            Timer::after(Duration::from_millis(300)).await;
	}
    }
}
