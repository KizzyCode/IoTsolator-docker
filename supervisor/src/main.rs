#![doc = include_str!("../README.md")]
// Clippy lints
#![warn(clippy::large_stack_arrays)]
#![warn(clippy::arithmetic_side_effects)]
#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::indexing_slicing)]
#![warn(clippy::panic)]
#![warn(clippy::todo)]
#![warn(clippy::unimplemented)]
#![warn(clippy::unreachable)]
#![warn(clippy::missing_panics_doc)]
#![warn(clippy::allow_attributes_without_reason)]
#![warn(clippy::cognitive_complexity)]

// Fail compilation on non-unix systems
#[cfg(not(target_family = "unix"))]
compile_error!("application is unix only");

mod config;
mod services;
mod supervisor;

use crate::config::ConfigFiles;
use crate::services::{Hostapd, Ifconfig};
use crate::supervisor::PanicWatchdog;
use services::Dhcpd;
use std::thread;
use std::time::Duration;

pub fn main() {
    // Setup config files from environment
    let config = ConfigFiles::from_env();

    // Spawn child processes
    let watchdog = PanicWatchdog::start();
    let interface = Ifconfig::init(&config);
    let dhcpd = Dhcpd::init(&config);
    let hostapd = Hostapd::init(&config);

    // Create watch-thread for each daemon
    thread::spawn({
        // Raise a watchdog alert if the process exits
        let dhcpd = dhcpd.clone();
        move || dhcpd.expect_never()
    });
    thread::spawn({
        // Raise a watchdog alert if the process exits
        let hostapd = hostapd.clone();
        move || hostapd.expect_never()
    });

    // Spin until we get an alert
    while !watchdog.has_alert() {
        /// Spin interval to avoid a tight loop
        const SPIN_INTERVAL: Duration = Duration::from_millis(333);
        thread::sleep(SPIN_INTERVAL);
    }

    // Tear down everything
    hostapd.kill();
    dhcpd.kill();
    interface.kill();
}
