# Build edge runtime binary `maverick-edge`
FROM rust:1.84-bookworm AS builder

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY README.md ROADMAP.md ./

RUN cargo build --release -p maverick-runtime-edge

FROM debian:bookworm-slim

RUN groupadd --system maverick \
    && useradd --system --no-create-home --shell /usr/sbin/nologin --gid maverick maverick

COPY --from=builder /build/target/release/maverick-edge /usr/local/bin/maverick-edge

RUN mkdir -p /var/lib/maverick && chown maverick:maverick /var/lib/maverick

USER maverick
WORKDIR /var/lib/maverick

ENV RUST_LOG=info

ENTRYPOINT ["/usr/local/bin/maverick-edge"]
CMD ["status"]
