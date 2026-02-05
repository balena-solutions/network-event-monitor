FROM rust:alpine3.21 AS build

WORKDIR /app

RUN apk add --no-cache \
	musl-dev \
	pkgconfig \
	dbus-dev \
	sqlite-dev

COPY . .

ENV RUSTFLAGS="-C target-feature=-crt-static"

RUN cargo build --release --example monitor_example

FROM alpine:3.21

RUN apk add --no-cache \
	ca-certificates \
	dbus-libs \
	sqlite-libs \
	libgcc \
	libstdc++

COPY --from=build /app/target/release/examples/monitor_example /monitor_example

ENV DBUS_SYSTEM_BUS_ADDRESS=unix:path=/host/run/dbus/system_bus_socket

ENTRYPOINT ["/monitor_example"]
