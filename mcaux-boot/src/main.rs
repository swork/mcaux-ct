#![no_std]
#![no_main]

use core::cell::RefCell;

use cortex_m_rt::{entry, exception};
#[cfg(feature = "defmt")]
#[allow(unused)]
use defmt::{trace, debug, info, warn, error};
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_boot_rp::*;
use embassy_sync::blocking_mutex::Mutex;
use embassy_time::Duration;
#[cfg(feature = "blink")]
use embassy_rp::gpio::{Level, Output};
#[cfg(feature = "uart")]
use embassy_rp::uart;

const FLASH_SIZE: usize = 2 * 1024 * 1024;
const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = env!("CARGO_PKG_NAME");

#[entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());
    #[cfg(feature = "uart")]
    let config = uart::Config::default();
    #[cfg(feature = "uart")]
    let mut u = uart::Uart::new_blocking(p.UART0, p.PIN_0, p.PIN_1, config);

    #[cfg(feature = "defmt")]
    info!("{} {}", NAME, VERSION);
    #[cfg(feature = "uart")]
    {
        u.blocking_write(NAME.as_bytes()).unwrap();
        u.blocking_write(" ".as_bytes()).unwrap();
        u.blocking_write(VERSION.as_bytes()).unwrap();
        u.blocking_write("\r\n".as_bytes()).unwrap();
    }

    // Uncomment this if you are debugging the bootloader with debugger/RTT attached,
    // as it prevents a hard fault when accessing flash 'too early' after boot.
    /*
    for i in 0..10000000 {
        cortex_m::asm::nop();
    }
    */

    let flash = WatchdogFlash::<FLASH_SIZE>::start(p.FLASH, p.WATCHDOG, Duration::from_secs(8));
    let flash = Mutex::new(RefCell::new(flash));

    #[cfg(feature = "uart")]
    u.blocking_write("I love my mother, God is in his Heaven and all is right with the world.\r\n".as_bytes()).unwrap();

    #[cfg(feature = "blink")]
    let mut led = Output::new(p.PIN_2, Level::Low);

    #[cfg(feature = "blink")]
    led.set_high();
    #[cfg(feature = "defmt")]
    info!("led on if configured");
    #[cfg(feature = "uart")]
    u.blocking_write("led on maybe\r\n".as_bytes()).expect("u.write");

    #[cfg(feature = "blink")]
    cortex_m::asm::delay(5000);

    #[cfg(feature = "defmt")]
    info!("led off if configured");
    #[cfg(feature = "uart")]
    u.blocking_write("led off maybe\r\n".as_bytes()).expect("u.write");
    #[cfg(feature = "blink")]
    led.set_low();

    let config = BootLoaderConfig::from_linkerfile_blocking(&flash, &flash, &flash);
    let active_offset = config.active.offset();

    #[cfg(feature = "defmt")]
    info!("config.active.offset: {:x}", config.active.offset());
    #[cfg(feature = "uart")]
    u.blocking_write("defmt did config.active.offset here".as_bytes()).expect("u.write");

    let bl: BootLoader = BootLoader::prepare(config);

    #[cfg(feature = "defmt")]
    info!("Loading application...");
    #[cfg(feature = "uart")]
    u.blocking_write("Loading application...\r\n".as_bytes()).expect("u.write");
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
fn panic(info: &core::panic::PanicInfo) -> ! {
    #[cfg(feature = "defmt")]
    if let Some(l) = info.location() {
        if let Some(m) = info.message().as_str() {
            error!("Panic: {} (at {}:{}:{})", m, l.file(), l.line(), l.column());
        } else {
            error!("Panic: {:?} (at {}:{}:{})", info, l.file(), l.line(), l.column());
        }
    } else {
        if let Some(m) = info.message().as_str() {
            error!("Panic: {}", m);
        } else {
            error!("Panic: {}", info);
        }
    }
    cortex_m::asm::udf();
}
