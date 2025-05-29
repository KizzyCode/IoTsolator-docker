//! The hostapd service

use crate::config::ConfigFiles;
use crate::supervisor::Process;
use std::fs;
use std::fs::File;
use std::ops::Deref;

/// A network interface handle
#[derive(Debug)]
pub struct Ifconfig {
    /// The interface name
    interface: String,
}
impl Ifconfig {
    /// Executable path
    const EXECUTABLE: &str = "/usr/sbin/ifconfig";

    /// Brings the given interface up
    pub fn init(config: &ConfigFiles) -> Self {
        // Get relevant config
        let interface = config.hostapd_interface();
        let (netmask, gateway) = config.dhcpd_subnet();

        // Bring the interface up
        println!("{} {interface} up {gateway} netmask {netmask}", Self::EXECUTABLE);
        Process::spawn(Self::EXECUTABLE, [&interface, "up", &gateway, "netmask", &netmask]).expect_zero();
        Self { interface }
    }

    /// Brings the interface down
    pub fn kill(self) {
        // Remove the network association
        println!("{} {} 0.0.0.0", Self::EXECUTABLE, self.interface);
        Process::spawn("/usr/sbin/ifconfig", [&self.interface, "0.0.0.0"]).expect_zero();

        // Bring the interface down
        println!("{} {} down", Self::EXECUTABLE, self.interface);
        Process::spawn("/usr/sbin/ifconfig", [&self.interface, "down"]).expect_zero();
    }
}

/// dhcpd service
#[derive(Debug)]
pub struct Dhcpd {
    /// The dhcpd process
    process: Process,
}
impl Dhcpd {
    /// dhcpd config file path
    pub const CONFIG_FILE: &str = "/etc/dhcpd.conf";
    /// dhcpd leases file
    const LEASES_FILE: &str = "/var/dhcpd/dhcpd.leases";
    /// Executable path
    const EXECUTABLE: &str = "/usr/sbin/dhcpd";

    /// Deploys the hostapd config and spawns the process
    pub fn init(config: &ConfigFiles) -> Self {
        // Get config file
        let Some(contents) = config.get(Self::CONFIG_FILE) else {
            // Our config file is not defined
            panic!("missing config file {}", Self::CONFIG_FILE);
        };
        if let Err(e) = fs::write(Self::CONFIG_FILE, contents) {
            // Panic if we cannot create our config
            panic!("failed to write config file {}: {e}", Self::CONFIG_FILE);
        }

        // Touch dhcpd leases
        if let Err(e) = File::options().write(true).create(true).open(Self::LEASES_FILE) {
            // Panic if we couldn't touch the dhcpd leases database
            panic!("failed to touch dhcpd leases file {}: {e}", Self::LEASES_FILE);
        };

        // Get relevant config and start dhcpd
        let interface = config.hostapd_interface();
        println!("{} -d -lf {} -cf {} {interface}", Self::EXECUTABLE, Self::LEASES_FILE, Self::CONFIG_FILE);
        let process =
            Process::spawn(Self::EXECUTABLE, ["-d", "-lf", Self::LEASES_FILE, "-cf", Self::CONFIG_FILE, &interface]);
        Self { process }
    }
}
impl Deref for Dhcpd {
    type Target = Process;

    fn deref(&self) -> &Self::Target {
        &self.process
    }
}

/// hostapd service
#[derive(Debug)]
pub struct Hostapd {
    /// The hostapd process
    process: Process,
}
impl Hostapd {
    /// hostapd config file path
    pub const CONFIG_FILE: &str = "/etc/hostapd.conf";
    /// Executable path
    const EXECUTABLE: &str = "/usr/sbin/hostapd";

    /// Deploys the hostapd config and spawns the process
    pub fn init(config: &ConfigFiles) -> Self {
        // Get config file
        let Some(contents) = config.get(Self::CONFIG_FILE) else {
            // Our config file is not defined
            panic!("missing config file {}", Self::CONFIG_FILE);
        };
        if let Err(e) = fs::write(Self::CONFIG_FILE, contents) {
            // Panic if we cannot create our config
            panic!("failed to write config file {}: {e}", Self::CONFIG_FILE);
        }

        // Start hostapd
        println!("{} {}", Self::EXECUTABLE, Self::CONFIG_FILE);
        let process = Process::spawn(Self::EXECUTABLE, [Self::CONFIG_FILE]);
        Self { process }
    }
}
impl Deref for Hostapd {
    type Target = Process;

    fn deref(&self) -> &Self::Target {
        &self.process
    }
}
