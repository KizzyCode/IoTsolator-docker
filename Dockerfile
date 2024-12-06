FROM debian:stable-slim

ENV APT_PACKAGES gettext-base hostapd isc-dhcp-server net-tools supervisor
ENV DEBIAN_FRONTEND noninteractive
RUN apt-get update \
    && apt-get upgrade --yes \
    && apt-get install --yes --no-install-recommends ${APT_PACKAGES} \
    && apt-get clean

COPY files/dhcpd.conf.template /etc/dhcpd.conf.template
COPY files/hostapd.conf.template /etc/hostapd.conf.template
COPY files/supervisord.conf.template /etc/supervisord.conf.template
COPY files/start.sh /libexec/start.sh
COPY files/stop.sh /libexec/stop.sh

CMD ["/libexec/start.sh"]
