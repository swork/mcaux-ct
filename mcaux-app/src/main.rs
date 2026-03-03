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
use embassy_rp::watchdog::Watchdog;
use embassy_rp::{bind_interrupts, dma};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Duration;
use embedded_io_async::Read;
use heapless::String;
use mcaux::{
    AssignedResources, SwitchingResources, TELEMETRY_CHANNEL_DEPTH, Telemetry, main_rp,
    split_resources,
};
use reqwless::client::{HttpClient, TlsConfig, TlsVerify};
use reqwless::request::RequestBuilder;
use static_cell::StaticCell;
use utility_section::conf;
use zerocopy::IntoBytes;

#[cfg(feature = "rp235xa")]
const DFU_PATH: &str = "mcaux-app/pico2w/latest/";
#[cfg(feature = "rp2040")]
const DFU_PATH: &str = "mcaux-app/picow/latest/";

#[cfg(feature = "rp235xa")]
const FLASH_SIZE: usize = 4 * 1024 * 1024;
#[cfg(feature = "rp2040")]
const FLASH_SIZE: usize = 2 * 1024 * 1024;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BIN_FILE_NAME: &str = "release.bin";

// Alternative to panic_probe, which has yet to make sense to me
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("Panic, {}", info);
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
        info!("stem v{}", VERSION);
    } else {
        info!("main v{}", VERSION);
    }

    info!("1");
    let p = embassy_rp::init(Default::default());
    info!("2");
    let r = split_resources!(p);
    info!("split_resources done");

    const WATCHDOG_TIMEOUT: Duration = Duration::from_secs(8);

    // Override bootloader watchdog
    let mut watchdog = Watchdog::new(p.WATCHDOG);
    watchdog.start(WATCHDOG_TIMEOUT);

    // Find the UTILITY section containing separately-loaded config data
    unsafe extern "C" {
        static utility_block_starts_here: u8;
        static utility_block_ends_here: u8;
    }

    unsafe {
        info!(
            "utility address: {:?}",
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
    let dfu_prefix: [&str; 1] = ["https://httpbin.org/json"];
    /*
    let dfu_prefix: [&str; 3] = [0u8, 1, 2].map(|i| {
        if let Some(url) = config.get_value_by_key_n(b"DFU", i) {
            info!("dfu {} {}", i, url);
            str::from_utf8(url).expect("utf8")
        } else {
            ""
        }
    });
    */
    info!("dfu {:?}", dfu_prefix);
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
        type TelemetryChannel =
            Channel<CriticalSectionRawMutex, Telemetry, TELEMETRY_CHANNEL_DEPTH>;
        static TELEMETRY_CHANNEL: StaticCell<TelemetryChannel> = StaticCell::new();
        let telemetry_channel = TELEMETRY_CHANNEL.init(TelemetryChannel::new());
        let telemetry_sender = telemetry_channel.sender();
        let telemetry_receiver = telemetry_channel.receiver();

        // Establish the core application: switching aux equipment
        spawner.spawn(main_rp(spawner, r.switching, telemetry_sender).expect("Main switcher task"));

        //////////////////////////////////////////////////////////////////////////////
        // The motorcycle switch manager expects to do networking very rarely,      //
        // triggered by a Vulcan-pinch of some kind when firmware update is needed  //
        // or telemetry is of interest (temperature of the bike's voltage regulator //
        // for example). In most runs we'll stay here until power-off, tickling the //
        // watchdog on behalf of the application's main loop.                       //
        //////////////////////////////////////////////////////////////////////////////
        while let Telemetry::Alive = telemetry_receiver.receive().await {
            watchdog.feed(WATCHDOG_TIMEOUT);
        }
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
                watchdog.feed(WATCHDOG_TIMEOUT);
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
    watchdog.start(WATCHDOG_TIMEOUT);
    stack.wait_link_up().await;
    info!("waiting for DHCP...");
    watchdog.start(WATCHDOG_TIMEOUT);
    stack.wait_config_up().await;
    // RGB green
    info!("Stack is up!");
    watchdog.start(WATCHDOG_TIMEOUT);
    // END NETWORKING_SETUP

    let mut rx_buffer = [0u8; 4096]; // TODO changed from "0", check for consequence
    let mut tls_read_buffer = [0; 16640];
    let mut tls_write_buffer = [0; 16640];
    let client_state = TcpClientState::<1, 4096, 4096>::new();
    let tcp_client = TcpClient::new(stack, &client_state);
    let dns_client = DnsSocket::new(stack);
    let tls_config = TlsConfig::new(
        seed,
        &mut tls_read_buffer,
        &mut tls_write_buffer,
        TlsVerify::None,
    );
    let mut http_client = HttpClient::new_with_tls(&tcp_client, &dns_client, tls_config);

    // Do telemetry dump interactions
    // telemetry(&mut http_client).await.expect("succeeded");

    // Firmware update, after check.
    let version: usize = VERSION
        .split('.')
        .next()
        .expect("str")
        .parse::<usize>()
        .expect("number");
    info!("VERSION:{}", version);
    let mut req_buf: String<80, u8> = String::new();
    let mut i = 0;
    if let Some(mut resource) = loop {
        info!("loop on {} with prefix {:?}", i, dfu_prefix[i]);
        if !dfu_prefix[i].is_empty() {
            req_buf.truncate(0);
            req_buf.push_str(dfu_prefix[i]).expect("capacity");
            req_buf.push_str(DFU_PATH).expect("capacity");
            info!(" full path {}", req_buf);
            if let Ok(mut resource) = http_client.resource(req_buf.as_str()).await {
                let request = resource.get("version");
                watchdog.start(WATCHDOG_TIMEOUT);
                if let Ok(response) = request.send(&mut rx_buffer).await {
                    if response.status.is_successful() {
                        info!(
                            "Response status:{} type:{:?} len:{:?} tenc:{:?}, ka:{:?}",
                            response.status.0,
                            response.content_type,
                            response.content_length,
                            response.transfer_encoding,
                            response.keep_alive,
                        );
                        let ver_as_str = str::from_utf8(
                            response
                                .body()
                                .read_to_end()
                                .await
                                .expect("version in body"),
                        )
                        .expect("utf8");
                        let ver_as_num = ver_as_str
                            .split('.')
                            .next()
                            .expect("str")
                            .parse::<usize>()
                            .expect("number");
                        if ver_as_num > version {
                            info!(
                                "DFU {} newer than {}, proceed with update",
                                ver_as_str, version
                            );
                            break Some(resource);
                        } else {
                            info!("DFU {} not newer than {}, sit pat", ver_as_str, version);
                            break None;
                        }
                    } else {
                        warn!("Unexpected status {} from {}", response.status.0, req_buf);
                    };
                } else {
                    warn!("Failed to send HTTP request to {} {}", req_buf, "version");
                }
            } else {
                warn!("Failed to make resource at {}", req_buf)
            }
        } else {
            warn!("No resource");
        }
        i += 1;
        if i >= dfu_prefix.len() {
            break None;
        }
    } {
        // Get size
        watchdog.start(WATCHDOG_TIMEOUT);
        let size_rsp = resource
            .get("size")
            .send(&mut rx_buffer)
            .await
            .expect("size rsp");
        let size = str::from_utf8(
            if size_rsp.status.is_successful() {
                watchdog.start(WATCHDOG_TIMEOUT);
                size_rsp.body().read_to_end().await
            } else {
                panic!("unexpected size rsp status {}", size_rsp.status.0);
            }
            .expect("size in body"),
        )
        .expect("utf8")
        .parse::<usize>()
        .expect("usize");

        // get binary, one page at a time. Be sure the resource doesn't change part way.
        let mut etag: String<64, u8> = String::new();
        for start_byte in (0..size).step_by(4096) {
            let end_byte = if start_byte + 4096 < size {
                start_byte + 4096
            } else {
                size
            };
            let mut start_buf = itoa::Buffer::new();
            let mut end_buf = itoa::Buffer::new();
            let mut range_value: String<32, u8> = String::new();
            let _ = range_value.push_str("bytes=");
            let _ = range_value.push_str(start_buf.format(start_byte));
            let _ = range_value.push('-');
            let _ = range_value.push_str(end_buf.format(end_byte));
            let range_tuple = ("Range", range_value.as_str());
            let mut etag_value: String<64, u8> = String::new();
            let _ = etag_value.push_str(&etag);
            let etag_tuple = if etag.is_empty() {
                ("X-Placeholder", "no-value")
            } else {
                ("If-Range", etag_value.as_str())
            };
            let h = [range_tuple, etag_tuple];
            watchdog.start(WATCHDOG_TIMEOUT);
            let chunk_rsp = resource
                .get(BIN_FILE_NAME)
                .headers(&h)
                .send(&mut rx_buffer)
                .await
                .expect("send chunk req");
            if chunk_rsp.status.is_successful() {
                // Ensure response is 206 not 200. Conditional req failed if 200.
                if chunk_rsp.status.0 == 200 {
                    error!("If-Range request rejected {}", range_tuple.1);
                    panic!("DFU fail");
                }

                for h in chunk_rsp.headers() {
                    match h {
                        ("Content-Range", value) => {
                            let val_str = str::from_utf8(value).expect("utf8");
                            let slash = val_str.find('/').expect("size trails");
                            let size_here: usize =
                                val_str[slash + 1..].parse::<usize>().expect("num");
                            if size_here != size {
                                error!("size in {} isn't {}", value, size);
                                panic!("DFU changed");
                            }
                        }
                        ("ETag", value) => {
                            if etag.is_empty() {
                                let _ = etag.push_str(str::from_utf8(value).expect("utf8"));
                            }
                        }
                        (_, _) => (),
                    };
                }

                let mut chunk_buffer: [_; 4096] = [0u8; 4096];
                watchdog.start(WATCHDOG_TIMEOUT);
                chunk_rsp
                    .body()
                    .reader()
                    .read_exact(&mut chunk_buffer)
                    .await
                    .expect("read_exact");
                watchdog.start(WATCHDOG_TIMEOUT);
                updater
                    .write_firmware(start_byte, &chunk_buffer)
                    .expect("write_firmware");
            } else {
                error!(
                    "DFU fail status:{} at {}",
                    chunk_rsp.status.0, range_tuple.1
                );
                panic!("DFU fail");
            }
        }
        watchdog.start(WATCHDOG_TIMEOUT);
        updater.mark_updated().expect("mark_updated");
    } else {
        error!("Found no usable DFUx url");
    }

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
