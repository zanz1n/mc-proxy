FROM rust:1 AS builder

ARG PROFILE="release"

WORKDIR /build

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --profile ${PROFILE}

RUN --mount=type=cache,target=/build/target \
    case ${PROFILE} in \
    dev) PROFILE_PATH="debug" ;; \
    release) PROFILE_PATH="release" ;; \
    esac \
    && mkdir -p /build/bin \
    && cp /build/target/${PROFILE_PATH}/mc-proxy /build/bin/mc-proxy

FROM gcr.io/distroless/cc-debian12

COPY --from=builder /build/bin/mc-proxy /mc-proxy

CMD [ "/mc-proxy" ]
