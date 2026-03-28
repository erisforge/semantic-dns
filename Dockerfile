FROM rust:1.88-bookworm AS build

WORKDIR /app

COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY bins ./bins
COPY crates ./crates

RUN cargo build --release -p semantic-dns

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=build /app/target/release/semantic-dns /usr/local/bin/semantic-dns
COPY deploy/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh

RUN chmod +x /usr/local/bin/docker-entrypoint.sh \
    && mkdir -p /app/config /data

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
