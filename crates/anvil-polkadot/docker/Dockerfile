FROM rust:1.89-slim-bookworm AS builder

WORKDIR /usr/src/foundry-polkadot

ARG RELEASE_REF

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential \
        clang \
        libclang-dev \
        protobuf-compiler \
        pkg-config \
        libssl-dev \
        ca-certificates \
        git && \
    rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown && \
    rustup component add rust-src

# Copy the full workspace so we can build the anvil-polkadot crate with its workspace dependencies
COPY . .

# Optionally switch to a specific git ref (tag/branch/commit) before building.
# If RELEASE_REF is not set, the current workspace state is used.
RUN if [ -n "$RELEASE_REF" ]; then \
      git fetch --tags origin && \
      git checkout "$RELEASE_REF"; \
    fi

# Build only the anvil-polkadot binary in release mode
RUN cargo build --release -p anvil-polkadot

FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        openssl && \
    rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/foundry-polkadot/target/release/anvil-polkadot /usr/local/bin/anvil-polkadot

# Default JSON-RPC port used by anvil-polkadot
EXPOSE 8545

# Run as non-root user for better security
RUN useradd -m -u 1000 anvil && chown -R anvil /app
USER anvil

# By default, start the anvil-polkadot node.
# Additional CLI arguments can be passed at `docker run` time.
ENTRYPOINT ["anvil-polkadot"]

# Default command: Bind to 0.0.0.0 to allow external access (e.g., from Hardhat)
CMD ["--host", "0.0.0.0"]
