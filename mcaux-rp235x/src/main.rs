#![no_std]
#![no_main]

use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use mcaux::{main_rp, AssignedResources, SwitchingResources, split_resources};
use static_cell::StaticCell;

#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"swork/mcaux-ct"),
    embassy_rp::binary_info::rp_program_description!(
        c"Momentary contact switching for motorcycle aux equipment"
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("Panic occurred: {:?}", defmt::Display2Format(info));
    // whatever else to do
    loop {} // Halt the program
}

// from wifi_blinky, setup to twiddle the PicoW LED (and for that matter to use
// the wifi subsystem at all)
bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

/// Entry point. Initialize hardware and abstract state machines
/// representing I/O elements and indicators, then loop mediating
/// between them.
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let r = split_resources!(p);
    main_rp(spawner, r.switching).await; // "forever", but return for networking

    // Move these to fixed sections in memory map, per wifi_blinky.rs
    // to save space
    let fw = include_bytes!("../../../../Github/embassy/cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../../../Github/embassy/cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    spawner.spawn(cyw43_task(runner)).expect("spawn cyw43_task");

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    network_telemetry(&mut control).await;
    network_dfu(&mut control).await;  // ends in reset regardless of processing outcome
}

#[allow(dead_code)]
async fn blink(on_p: bool, control: &mut cyw43::Control<'_>) -> () {
    control.gpio_set(0, on_p).await;
}

async fn network_telemetry(_control: &mut cyw43::Control<'_>) -> () {
}

async fn network_dfu(_control: &mut cyw43::Control<'_>) -> () {
}
