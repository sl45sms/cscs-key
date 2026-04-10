FROM rust:1-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        pkg-config \
        libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY src ./src
COPY templates ./templates

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --shell /bin/bash appuser

WORKDIR /work

COPY --from=builder /app/target/release/cscs-key /usr/local/bin/cscs-key

ENV HOME=/home/appuser \
    CSCS_OIDC_REDIRECT_URL=http://localhost:8765 \
    CSCS_OIDC_BIND_ADDR=0.0.0.0:8765

USER appuser

ENTRYPOINT ["cscs-key"]
CMD ["--help"]
