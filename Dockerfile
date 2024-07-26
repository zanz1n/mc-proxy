FROM rust:1 AS builder

WORKDIR /build

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release

RUN --mount=type=cache,target=/build/target \
    mkdir -p /build/bin \
    && cp /build/target/release/mc-proxy /build/bin/mc-proxy

FROM gcr.io/distroless/cc-debian12

COPY --from=builder /build/bin/mc-proxy /mc-proxy

CMD [ "/mc-proxy" ]
