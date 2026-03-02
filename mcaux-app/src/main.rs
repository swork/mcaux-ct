#![no_std]
#![no_main]

#[cfg(all(feature = "rp2040", feature = "rp235xa"))]
compile_error!("feature \"rp2040\" and feature \"rp235xa\" must not both be specified");

#[cfg(not(any(feature = "rp2040", feature = "rp235xa")))]
compile_error!("one or the other of feature \"rp2040\" and \"rp235xa\" must be specified");

use aligned::A4;
use core::cell::RefCell;
use cyw43::JoinOptions;
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::{error, info, warn};
use defmt_rtt as _;
use embassy_boot::{AlignedBuffer, BlockingFirmwareUpdater, FirmwareUpdaterConfig, State};
use embassy_executor::Spawner;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::{Config, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_rp::flash::Flash;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::{bind_interrupts, dma};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use mcaux::{AssignedResources, SwitchingResources, main_rp, split_resources};

use reqwless::client::HttpClient;
use reqwless::request::Method;
use static_cell::StaticCell;
use utility_section::conf;
use zerocopy::IntoBytes;

#[cfg(feature = "rp235xa")]
const FLASH_SIZE: usize = 4 * 1024 * 1024;
#[cfg(feature = "rp2040")]
const FLASH_SIZE: usize = 2 * 1024 * 1024;

// Alternative to panic_probe, which has yet to make sense to me
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    error!("Panic.");
    error!("PanicInfo, if it formats: {:?}", _info);
    cortex_m::asm::udf();
    #[allow(unreachable_code)] // else they complain about "-> !" above
    loop {}
}

// from wifi_blinky, setup to twiddle the PicoW LED (and for that matter to use
// the wifi subsystem at all)
bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>,
) -> ! {
    runner.run().await
}

/// Entry point. Initialize hardware and abstract state machines
/// representing I/O elements and indicators, then loop mediating
/// between them.
#[embassy_executor::main]
async fn main(spawner: Spawner) -> () {
    if cfg!(feature = "stem") {
        info!("stem");
    } else {
        info!("main.");
    }

    info!("1");
    let p = embassy_rp::init(Default::default());
    info!("2");
    let r = split_resources!(p);
    info!("split_resources done");

    // Find the UTILITY section containing separately-loaded config data
    unsafe extern "C" {
        static utility_block_starts_here: u8;
        static utility_block_ends_here: u8;
    }

    unsafe {
        info!(
            "d: utility address: {:?}",
            (&raw const utility_block_starts_here).add(0x10000000)
        );
    }
    static UTILITY_SECTION_PTR: StaticCell<*const u8> = StaticCell::new();
    let utility_len = unsafe {
        (&raw const utility_block_ends_here).offset_from(&raw const utility_block_starts_here)
    };
    let utility_section: &[u8] = unsafe {
        core::slice::from_raw_parts(
            *UTILITY_SECTION_PTR.init((&raw const utility_block_starts_here).add(0x10000000)),
            utility_len as usize,
        )
    };

    // param is max item count, strings plus blobs
    let config: conf::Conf<20> = conf::Conf::new(utility_section);
    let dfu: &str = str::from_utf8(
        config
            .get_value_by_key("DFU".as_bytes())
            .expect("DFU existence"),
    )
    .expect("utf8");
    info!("dfu: {:?}", &dfu);
    let mut ap = [""; 5];
    let mut pw = ["".as_bytes(); 5];
    for i in 0usize..5 {
        let key = [b'A', b'P', b'0' + i as u8];
        if let Some(ap_name) = config.get_value_by_key(&key) {
            ap[i] = str::from_utf8(ap_name.as_bytes()).expect("utf8");
            let key = [b'P', b'W', b'0' + i as u8];
            pw[i] = config.get_value_by_key(&key).expect("existence");
        } else {
            break;
        }
        info!("ap, pw: {:?} {:?}", &ap[i], &pw[i]);
    }

    let fw = config.get_blob_by_id::<A4>(1).expect("fw existence");
    let clm = config.get_blob_by_id::<A4>(2).expect("clm existence");
    let nvram = config.get_blob_by_id::<A4>(3).expect("nvram existence");

    if !cfg!(feature = "stem") {
        const TELEMETRY_CHANNEL_DEPTH: usize = 1;
        type TelemetryChannel = Channel<CriticalSectionRawMutex, (), TELEMETRY_CHANNEL_DEPTH>;
        static TELEMETRY_CHANNEL: StaticCell<TelemetryChannel> = StaticCell::new();

        let telemetry_channel = TELEMETRY_CHANNEL.init(TelemetryChannel::new());
        let telemetry_receiver = telemetry_channel.receiver();

        // Establish the core application: switching aux equipment
        spawner.spawn(main_rp(spawner, r.switching).expect("Main switcher task"));

        //////////////////////////////////////////////////////////////////////////////
        // The motorcycle switch manager expects to do networking very rarely,      //
        // triggered by a Vulcan-pinch of some kind when firmware update is needed  //
        // or telemetry is of interest (temperature of the bike's voltage regulator //
        // for example). In most runs this funtion will wait here until power-off.  //
        //////////////////////////////////////////////////////////////////////////////
        telemetry_receiver.receive().await;
    }

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
        dma::Channel::new(p.DMA_CH0, Irqs),
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
    spawner.spawn(cyw43_task(runner).expect("spawn cyw43_task"));
    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // BEGIN NETWORK SETUP BLOCK
    let config = Config::dhcpv4(Default::default());
    let mut rng = RoscRng;
    let seed = rng.next_u64();
    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );
    // END NETWORK SETUP BLOCK

    let flash = Flash::<_, _, FLASH_SIZE>::new_blocking(p.FLASH);
    let flash = Mutex::new(RefCell::new(flash));
    let config = FirmwareUpdaterConfig::from_linkerfile_blocking(&flash, &flash);
    let mut aligned = AlignedBuffer([0; 1]);
    let mut updater = BlockingFirmwareUpdater::new(config, &mut aligned.0);

    if !cfg!(feature = "stem")
        && !matches!(updater.get_state().expect("DFU get_state"), State::Boot)
    {
        updater.mark_booted().unwrap();
    }

    // Delayed until network is requested. Saves dhcp operations when link is
    // down. Another application, one that expects to do networking on an
    // ongoing basis, would start this earlier and look for operation requests
    // in a loop, but we don't need to loop.
    spawner.spawn(net_task(runner).expect("spawn net_task(runner)"));

    // TODO: Put a pulser on the indicators, overriding all else.
    // RGB black

    // BEGIN NETWORKING SETUP, from embassy rp wifi_webrequest example

    // TODO Loop over several: home wifi, my phone's hotspot
    #[allow(clippy::never_loop)]
    'outer: loop {
        for i in 0usize..5 {
            if !ap[i].is_empty() {
                if let Err(err) = control.join(ap[i], JoinOptions::new(pw[i])).await {
                    info!("join ssid {:?} failed: {:?}", ap[i], err);
                    continue;
                }
                info!("Connected to access point {:?}", ap[i]);
                break 'outer;
            }
        }
        // RGB red
        warn!("No access point connection succeeded so far");
    }

    // RGB blue
    info!("waiting for link...");
    stack.wait_link_up().await;
    info!("waiting for DHCP...");
    stack.wait_config_up().await;
    // RGB green
    info!("Stack is up!");
    // END NETWORKING_SETUP

    // TEMP just retrieve a URL, anything
    let mut rx_buffer = [0; 4096];
    let client_state = TcpClientState::<1, 4096, 4096>::new();
    let tcp_client = TcpClient::new(stack, &client_state);
    let dns_client = DnsSocket::new(stack);
    let mut http_client = HttpClient::new(&tcp_client, &dns_client);

    if let Ok(mut request) = http_client.request(Method::GET, dfu).await {
        if let Ok(response) = request.send(&mut rx_buffer).await {
            match response.status.0 {
                200 => {
                    info!("Response status {}", response.status.0);
                }
                _ => panic!("Unexpected DFU response status {}", response.status.0),
            };
        } else {
            error!("Failed to send HTTP request");
        }
        error!("Failed to create HTTP request.");
    }

    // from here, green RGB blinks off when network operations are in .await
    // take_indicators();
    // if connect() {
    // indicators_sending();
    // send_telemetry();
    // indicators_retrieving();
    // if retrieve_dfu_state() {
    //   retrieve_and_write_dfu();
    // reset();
    //   }
    // }
    // release_indicators();

    // Motorcycle: reset the controller after one network cycle.
    // Other uses would want other action.
    //reset();
    panic!("Deliberate panic to stop the show");
}

#[allow(dead_code)]
async fn blink(on_p: bool, control: &mut cyw43::Control<'_>) -> () {
    control.gpio_set(0, on_p).await;
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}
