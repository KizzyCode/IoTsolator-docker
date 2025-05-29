FROM debian:stable-slim AS buildenv

ENV APT_PACKAGES build-essential ca-certificates curl tree
ENV DEBIAN_FRONTEND noninteractive
RUN apt-get update \
    && apt-get upgrade --yes \
    && apt-get install --yes --no-install-recommends ${APT_PACKAGES}

RUN useradd --create-home --uid 10000 buildenv
USER buildenv

RUN curl --tlsv1.3 --output /home/buildenv/rustup.sh https://sh.rustup.rs
RUN sh /home/buildenv/rustup.sh -y --profile=minimal

COPY --chown=buildenv ./supervisor /home/buildenv/supervisor
RUN /home/buildenv/.cargo/bin/cargo install --path=/home/buildenv/supervisor


FROM debian:stable-slim

ENV APT_PACKAGES hostapd isc-dhcp-server net-tools
ENV DEBIAN_FRONTEND noninteractive
RUN apt-get update \
    && apt-get upgrade --yes \
    && apt-get install --yes --no-install-recommends ${APT_PACKAGES} \
    && apt-get clean

COPY --from=buildenv /home/buildenv/.cargo/bin/supervisor /usr/local/sbin/supervisor

CMD [ "/usr/local/sbin/supervisor" ]
