#![no_std]

use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};
use watchdog::WatchdogReport;

pub const TELEMETRY_CHANNEL_DEPTH: usize = 2;

pub enum TelemetryOperation {
    Run(u32),
}

impl TelemetryOperation {
    pub async fn run(watchdog_channel: &mut Sender<'static, CriticalSectionRawMutex, (), TELEMETRY_CHANNEL_DEPTH>, watchdog_pulse: &Duration) -> ! {
        loop {
            watchdog_channel.send(WatchdogReport::Telemetry);
            Timer::after(watchdog_pulse).await;
        }
    }
}

