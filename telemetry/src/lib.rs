#![no_std]

use  embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

pub enum TelemetryOperation {
    Run(u32),
}

pub const TELEMETRY_CHANNEL_DEPTH: usize = 5;
pub static TELEMETRY_CHANNEL: Channel<CriticalSectionRawMutex, TelemetryOperation, TELEMETRY_CHANNEL_DEPTH> = Channel::new();

