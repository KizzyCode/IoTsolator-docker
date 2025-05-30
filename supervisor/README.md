[![License BSD-2-Clause](https://img.shields.io/badge/License-BSD--2--Clause-blue.svg)](https://opensource.org/licenses/BSD-2-Clause)
[![License MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

# `supervisor`
Supervisor is a small, tailored application to assemble a configuration file from the environment variables, and to
spawn and watch the associated services.

## How it works
0. Deploy config files from environment `CONFIGFILE_*`-vars
1. Start `hostapd` and monitor process
2. Sleep 5s to wait until `hostapd` is up and has configured all interfaces
3. Call `ifup` and expect success
4. Sleep 5s to wait until `ifup` has configured all interfaces
5. Start `dnsmasq` and monitor process

On `SIGINT`/`SIGTERM`, or if a monitored child dies:
1. Send `SIGTERM` to all child processes
2. Sleep 2s to give all children some time to terminate gracefully
3. Send `SIGKILL` to all child processes(?)
2. Sleep 1s to ensure child resources are released
4. Call `ifdown` and expect success
