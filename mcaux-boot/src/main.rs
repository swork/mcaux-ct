#![no_std]
#![no_main]

use core::cell::RefCell;

use cortex_m_rt::{entry, exception};
#[cfg(feature = "defmt")]
#[allow(unused)]
use defmt::{debug, error, info, trace, warn};
#[cfg(feature = "defmt")]
use defmt_serial as _;
use embassy_boot_rp::*;
#[cfg(feature = "blink")]
use embassy_rp::gpio::{Level, Output};
#[cfg(feature = "defmt")]
use embassy_rp::uart::Uart;
use embassy_sync::blocking_mutex::Mutex;
use embassy_time::Duration;
#[cfg(feature = "defmt")]
use static_cell::StaticCell;

#[cfg(feature = "defmt")]
defmt::timestamp! {"{=u64:ms}", Instant::now().as_millis() }

const FLASH_SIZE: usize = 2 * 1024 * 1024;
#[allow(unused)]
const VERSION: &str = env!("CARGO_PKG_VERSION");
#[allow(unused)]
const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(feature = "defmt")]
static SERIAL: StaticCell<Uart<'_, embassy_rp::uart::Blocking>> = StaticCell::new();

#[entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());
    #[cfg(feature = "defmt")]
    let mut config = embassy_rp::uart::Config::default();
    #[cfg(feature = "defmt")]
    {
        config.baudrate = 115200;
        config.data_bits = embassy_rp::uart::DataBits::DataBits8;
        config.stop_bits = embassy_rp::uart::StopBits::STOP1;
        config.parity = embassy_rp::uart::Parity::ParityNone;
    }
    #[cfg(feature = "defmt")]
    let mut u = Uart::new_blocking(p.UART0, p.PIN_0, p.PIN_1, config);
    #[cfg(feature = "defmt")]
    defmt_serial::defmt_serial(SERIAL.init(u));

    #[cfg(feature = "defmt")]
    info!("{} {}", NAME, VERSION);

    #[cfg(feature = "defmt")]
    info!("Into long busy-wait, something about 'too early'");

    // Uncomment this if you are debugging the bootloader with debugger/RTT attached,
    // as it prevents a hard fault when accessing flash 'too early' after boot.
    for _i in 0..1000000 {
        cortex_m::asm::nop();
    }

    #[cfg(feature = "defmt")]
    info!("Done with long busy-wait");

    let flash = WatchdogFlash::<FLASH_SIZE>::start(p.FLASH, p.WATCHDOG, Duration::from_secs(14));
    let flash = Mutex::new(RefCell::new(flash));

    #[cfg(feature = "blink")]
    let mut led = Output::new(p.PIN_2, Level::Low);

    #[cfg(feature = "blink")]
    led.set_high();
    #[cfg(feature = "defmt")]
    info!("led on if configured");

    #[cfg(feature = "blink")]
    cortex_m::asm::delay(5000);

    #[cfg(feature = "defmt")]
    info!("led off if configured");
    #[cfg(feature = "blink")]
    led.set_low();

    let config = BootLoaderConfig::from_linkerfile_blocking(&flash, &flash, &flash);
    let active_offset = config.active.offset();

    #[cfg(feature = "defmt")]
    info!(
        "config.active.offset:{:08x} .dfu.offset:{:08x}",
        config.active.offset(),
        config.dfu.offset()
    );

    #[cfg(feature = "defmt")]
    info!("into prepare");
    let bl: BootLoader = BootLoader::prepare(config);
    #[cfg(feature = "defmt")]
    info!("back from prepare");

    #[cfg(feature = "defmt")]
    {
        info!(
            "Bootloader handoff @ {:08x}...",
            embassy_rp::flash::FLASH_BASE as u32 + active_offset
        );
        #[cfg(feature = "defmt")]
        defmt::flush();
    }

    unsafe { bl.load(embassy_rp::flash::FLASH_BASE as u32 + active_offset) }
}

#[unsafe(no_mangle)]
#[cfg_attr(target_os = "none", unsafe(link_section = ".HardFault.user"))]
unsafe extern "C" fn HardFault() {
    cortex_m::peripheral::SCB::sys_reset();
}

#[exception]
unsafe fn DefaultHandler(_: i16) -> ! {
    const SCB_ICSR: *const u32 = 0xE000_ED04 as *const u32;
    let irqn = unsafe { core::ptr::read_volatile(SCB_ICSR) } as u8 as i16 - 16;

    panic!("DefaultHandler #{:?}", irqn);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    #[cfg(feature = "defmt")]
    if let Some(l) = _info.location() {
        if let Some(m) = _info.message().as_str() {
            error!("Panic: {} (at {}:{}:{})", m, l.file(), l.line(), l.column());
        } else {
            error!(
                "Panic: {:?} (at {}:{}:{})",
                _info,
                l.file(),
                l.line(),
                l.column()
            );
        }
    } else {
        if let Some(m) = _info.message().as_str() {
            error!("Panic: {}", m);
        } else {
            error!("Panic: {}", _info);
        }
    }
    #[cfg(feature = "defmt")]
    defmt::flush();
    for _i in 0..10000000 {
        cortex_m::asm::nop();
    }
    cortex_m::asm::udf();
}
