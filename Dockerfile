FROM alpine:latest

RUN apk add --no-cache dhcp-server-vanilla envsubst hostapd supervisor

COPY files/dhcpd.conf.template /etc/dhcpd.conf.template
COPY files/hostapd.conf.template /etc/hostapd.conf.template
COPY files/supervisord.conf.template /etc/supervisord.conf.template
COPY files/start.sh /libexec/start.sh
COPY files/stop.sh /libexec/stop.sh

CMD ["/libexec/start.sh"]
