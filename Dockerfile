# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:1.77-slim AS builder

WORKDIR /build

# Cache dependencies before copying source
COPY Cargo.toml Cargo.lock ./
COPY crates/maverick-core/Cargo.toml  crates/maverick-core/
COPY crates/maverick-domain/Cargo.toml crates/maverick-domain/
COPY crates/maverick-crypto/Cargo.toml crates/maverick-crypto/

# Stub sources to cache the dependency layer
RUN mkdir -p crates/maverick-core/src/bin \
    crates/maverick-domain/src \
    crates/maverick-crypto/src && \
    echo 'fn main(){}' > crates/maverick-core/src/bin/main.rs && \
    echo '' > crates/maverick-core/src/lib.rs && \
    echo '' > crates/maverick-domain/src/lib.rs && \
    echo '' > crates/maverick-crypto/src/lib.rs

RUN cargo build --release -p maverick-core 2>/dev/null || true

# Copy real sources and rebuild only changed code
COPY crates/ crates/
RUN touch crates/maverick-core/src/bin/main.rs \
    crates/maverick-core/src/lib.rs \
    crates/maverick-domain/src/lib.rs \
    crates/maverick-crypto/src/lib.rs && \
    cargo build --release -p maverick-core

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates wget && \
    rm -rf /var/lib/apt/lists/* && \
    groupadd --system maverick && \
    useradd --system --no-create-home --shell /usr/sbin/nologin --gid maverick maverick

COPY --from=builder /build/target/release/maverick /usr/local/bin/maverick

RUN mkdir -p /var/lib/maverick && \
    chown maverick:maverick /var/lib/maverick

USER maverick
WORKDIR /var/lib/maverick

ENV MAVERICK_HTTP_BIND_ADDR=0.0.0.0:8080
ENV MAVERICK_UDP_BIND_ADDR=0.0.0.0:1700
ENV MAVERICK_DB_PATH=/var/lib/maverick/maverick.db
ENV MAVERICK_LOG_FILTER=maverick_core=info
ENV MAVERICK_STORAGE_PROFILE=auto

EXPOSE 8080/tcp
EXPOSE 1700/udp

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://localhost:8080/api/v1/health || exit 1

ENTRYPOINT ["/usr/local/bin/maverick"]
