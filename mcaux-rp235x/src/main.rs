#![no_std]
#![no_main]

use cyw43::JoinOptions;
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::info;
use defmt_rtt as _;
use embassy_boot::{AlignedBuffer, FirmwareUpdater, FirmwareUpdaterConfig};
use embassy_rp::clocks::RoscRng;
use embassy_executor::Spawner;
use embassy_net::{Config, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::flash::Flash;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::channel::Channel;
use mcaux::{main_rp, AssignedResources, SwitchingResources, split_resources};
use panic_probe as _;
use static_cell::StaticCell;
use telemetry::{TelemetryOperation, TELEMETRY_CHANNEL};

// rp235x has 4MB storage
const FLASH_SIZE: usize = 4 * 1024 * 1024;

/*
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
*/

/*  For when "use panic_probe as _;" is not enabled above.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("Panic occurred: {:?}", defmt::Display2Format(info));
    // whatever else to do
    loop {} // Halt the program
}
*/

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
async fn main(spawner: Spawner) -> () {
    info!("main");
    let p = embassy_rp::init(Default::default());
    let r = split_resources!(p);

    // Parse secrets up front, so bad format etc. won't be committed on update.
    const SECRETS_TXT: &[u8] = include_bytes!("../access_points.txt");
    let secrets_colon_idx = match SECRETS_TXT[..].iter().position(|&b| b == b':') {
        Some(idx) => idx,
        _ => panic!("Check access_points.txt for colon"),
    };
    let ap: &str = str::from_utf8(&SECRETS_TXT[..secrets_colon_idx]).unwrap();
    let pw: &[u8] = &SECRETS_TXT[secrets_colon_idx+1..];

    spawner.spawn(main_rp(spawner, r.switching, TELEMETRY_CHANNEL.sender())).expect("Main switcher task");

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
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    spawner.spawn(cyw43_task(runner)).expect("spawn cyw43_task");

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let telemetry_receiver = TELEMETRY_CHANNEL.receiver();

    // BEGIN NETWORK SETUP BLOCK
    let config = Config::dhcpv4(Default::default());
    let mut rng = RoscRng;
    let seed = rng.next_u64();
    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(net_device, config, RESOURCES.init(StackResources::new()), seed);
    // END NETWORK SETUP BLOCK

    // The motorcycle switch manager expects to do networking very
    // rarely, via a Vulcan-pinch of some kind when firmware update is
    // needed or telemetry is of interest (temperature of the bike's
    // voltage regulator for example). In most runs this loop will wait
    // at this receive() operation until power-off.
    let operation = telemetry_receiver.receive().await;

    // Delay this until network is requested. Saves dhcp operations when link is
    // down. Another application, one that expects to do networking on an
    // ongoing basis, would start this earlier and look for operation requests
    // in a loop, but we don't need to loop.
    spawner.spawn(net_task(runner)).expect("spawn net_task(runner)");

    // TODO: Put a pulser on the indicators, overriding all else.
    // RGB black

    // BEGIN NETWORKING SETUP, from embassy rp wifi_webrequest example

    // TODO Loop over several: home wifi, my phone's hotspot
    'outer: loop {  // not a loop, just a place for this label
        for i in 0..5 {
            if let Err(err) = control.join(ap, JoinOptions::new(pw)).await {
                info!("join ssid {:?} failed: {:?}", ap, err.status);
                continue;
            }
            info!("Connected to access point {:?}", ap);
            break 'outer;
        }
        // RGB red
        panic!("No access point connection succeeded");
    }

    // RGB blue
    info!("waiting for link...");
    stack.wait_link_up().await;
    info!("waiting for DHCP...");
    stack.wait_config_up().await;
    // RGB green
    info!("Stack is up!");
    // END NETWORKING_SETUP

    // from here, green RGB blinks off when network operations are in .await

/*
        match operation {
            TelemetryOperation::Run(_) => {
                // take_indicators();
                if connect() {
                    indicators_sending();
                    send_telemetry();
                    indicators_retrieving();
                    if retrieve_dfu_state() {
                        retrieve_and_write_dfu();
                        reset();
                    }
                }
                release_indicators();
            }
        }
*/

    // Motorcycle: reset the controller after one network cycle.
    // Other uses would want other action.
    //reset();
}

#[allow(dead_code)]
async fn blink(on_p: bool, control: &mut cyw43::Control<'_>) -> () {
    control.gpio_set(0, on_p).await;
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

