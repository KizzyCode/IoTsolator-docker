//! Environment-variable-based config file builder

use std::{env, fs};

/// Loads the config files from the environment variables
pub fn deploy_from_env() {
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
        let Some(path) = header.strip_prefix("#TARGET:") else {
            // Config file doesn't start with a target header
            panic!("config variable ${key} has invalid format");
        };

        // Write the config file
        let path = path.trim();
        if let Err(e) = fs::write(path, contents) {
            // Failed to write mandatory config file
            panic!("failed to write config file {path}: {e}");
        };
    }
}
