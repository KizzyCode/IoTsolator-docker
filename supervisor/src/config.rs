//! Environment-variable-based config file builder

use crate::services::{Dhcpd, Hostapd};
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::ops::Deref;

/// Config file deployment
#[derive(Debug, Clone)]
pub struct ConfigFiles {
    /// Config file contents by path
    files: HashMap<String, String>,
}
impl ConfigFiles {
    /// Loads the config files from the environment variables
    pub fn from_env() -> Self {
        // `#TARGET:`-path regex
        let regex = Regex::new(r"^#\s*TARGET:(.+)$").expect("failed to compile regex");

        // Scan vars
        let mut files = HashMap::new();
        'scan_vars: for (key, value) in env::vars() {
            // Check variable
            let true = key.starts_with("CONFIGFILE_") else {
                // Not a config file variable
                continue 'scan_vars;
            };
            let Some((header, contents)) = value.split_once("\n") else {
                // Config file doesn't have a header or contents section
                panic!("config variable ${key} has invalid format");
            };

            // Extract the target path directive (`#TARGET:`)
            let Some(match_) = regex.captures(header) else {
                // Config file doesn't start with a target header
                panic!("config variable ${key} has invalid format");
            };

            // Write the config file
            let path_untrimmed = match_.get(1).expect("failed to extract target path from regex match");
            let path = path_untrimmed.as_str().trim();
            files.insert(path.to_string(), contents.to_string());
        }

        // Init self
        Self { files }
    }

    /// Returns the interface value of the first interface directive from the hostapd config
    pub fn hostapd_interface(&self) -> String {
        // Get config file
        let Some(contents) = self.files.get(Hostapd::CONFIG_FILE) else {
            // Our config file is not defined
            panic!("missing config file {}", Hostapd::CONFIG_FILE);
        };

        // Try to match the first "interface="-line
        let regex = Regex::new(r"\s*interface\s*=\s*(\S+)\s*").expect("failed to compile regex");
        let Some(match_) = regex.captures(&contents) else {
            // Our regex didn't match any interface config line
            panic!("hostapd config does not contain an interface entry");
        };

        // Return match as string
        let interface = match_.get(1).expect("failed to extract interface from regex match");
        interface.as_str().to_string()
    }

    /// Returns `(netmask, gateway)` of the first subnet directive from the dhcpd config
    pub fn dhcpd_subnet(&self) -> (String, String) {
        // Get config file
        let Some(contents) = self.files.get(Dhcpd::CONFIG_FILE) else {
            // Our config file is not defined
            panic!("missing config file {}", Dhcpd::CONFIG_FILE);
        };

        // Try to match the first subnet declaration
        let regex = Regex::new(r"\s*subnet\s+\S+\s+netmask\s+(\S+)\s+\{([^}]+)\}").expect("failed to compile regex");
        let Some(match_) = regex.captures(&contents) else {
            // Our regex didn't match any interface config line
            panic!("dhcpd config does not contain a subnet declaration");
        };

        // Destructure first match
        let netmask = match_.get(1).expect("failed to extract netmask from regex match");
        let block = match_.get(2).expect("failed to extract the config block from the regex match");

        // Read gateway from config block
        let regex = Regex::new(r"\s*option\s+routers\s+(\S+).*;\s*").expect("failed to compile regex");
        let Some(match_) = regex.captures(block.as_str()) else {
            // Our regex didn't match any interface config line
            panic!("dhcpd config does not contain a router entry");
        };

        // Extract gateway and return tuple
        let gateway = match_.get(1).expect("failed to extract gateway from regex match");
        (netmask.as_str().to_string(), gateway.as_str().to_string())
    }
}
impl Deref for ConfigFiles {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.files
    }
}
