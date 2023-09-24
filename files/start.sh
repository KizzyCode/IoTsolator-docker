#!/bin/sh
set -euo pipefail

# Ensures that an environment variable is set
function has_env() {
    # Get varname
    VARNAME="$1"

    # Check if var exists in the environment
    if ! printenv "$VARNAME" > /dev/null; then
        echo "!> Missing required environment variable \$$VARNAME"
        exit 1
    fi
}

# Create DHCPD lease database if necessary
if ! test -f "/var/dhcpd/dhcpd.leases"; then
    touch "/var/dhcpd/dhcpd.leases"
fi

# Configure security options
if test "${SECURITY:-}" = "LEGACY"; then
    # Use WPA2 with AES-128 and optional protected management frames
    echo "!> Using SECURITY=LEGACY"
    export WPA="2"
    export WPA_KEY_MGMT="WPA-PSK"
    export RSN_PAIRWISE="CCMP"
    export IEEE80211W="1"
else
    # Use WPA2 with AES-256 and protected management frames
    export WPA="2"
    export WPA_KEY_MGMT="WPA-PSK-SHA256"
    export RSN_PAIRWISE="CCMP-256"
    export IEEE80211W="2"
fi

# Template hostapd.conf
has_env "INTERFACE"
has_env "COUNTRY"
has_env "CHANNEL"
has_env "SSID"
has_env "PASSPHRASE"
cat "/etc/hostapd.conf.template" | envsubst > "/etc/hostapd.conf"

# Template dhcpd.conf
has_env "SUBNET"
has_env "GATEWAY"
has_env "NETMASK"
has_env "DHCP"
cat "/etc/dhcpd.conf.template" | envsubst > "/etc/dhcpd.conf"

# Template supervisord.conf
has_env "INTERFACE"
cat "/etc/supervisord.conf.template" | envsubst > "/etc/supervisord.conf"

# Start interface
has_env "INTERFACE"
has_env "GATEWAY"
has_env "NETMASK"
ifconfig "$INTERFACE" up "$GATEWAY" netmask "$NETMASK"

# Become supervisord
exec /usr/bin/supervisord -c "/etc/supervisord.conf"
