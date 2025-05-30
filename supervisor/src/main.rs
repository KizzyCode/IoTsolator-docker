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
mod watchdog;

use crate::watchdog::Watchdog;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub fn main() {
    /// Grace-period to give a service some time to setup their business
    const SETUP_GRACEPERIOD: Duration = Duration::from_secs(5);
    /// Grace-period to wait for the child processes after SIGTERM
    const TEARDOWN_GRACEPERIOD: Duration = Duration::from_secs(2);

    // Spawn watchdog and deploy config
    Watchdog::scope(
        // Main logic under watchdog supervision
        |watchdog| {
            // Deploy config
            config::deploy_from_env();

            // Spawn hostapd and specify the config file
            watchdog.spawn_child("hostapd", ["/etc/hostapd.conf"], None);
            thread::sleep(SETUP_GRACEPERIOD);

            // Spawn ifup in verbose mode to bring up all interfaces
            watchdog.spawn_child("ifup", ["-v", "-a"], Some(0));
            thread::sleep(SETUP_GRACEPERIOD);

            // Spawn dhcpd, keep it in foreground, and specify the config file
            watchdog.spawn_child("dhcpd", ["-d", "-cf", "/etc/dhcpd.conf"], None);
        },
        // On-alert handler to shutdown application
        || {
            // Send sigterm to our own process group to kill the childs
            // Note: We catch SIGTERM, so this doesn't kill ourselves
            unsafe { libc::kill(0, libc::SIGTERM) };
            thread::sleep(TEARDOWN_GRACEPERIOD);

            // Configure ifup in verbose mode to bring sown all interfaces...
            Command::new("ifdown").args(["-v", "-s"])
                // ... and spawn process synchronously...
                .spawn().expect("failed to spawn ifdown")
                // ...and wait until ifdown completes or we are terminated
                .wait().expect("failed to teardown interfaces");
        },
    );
}
