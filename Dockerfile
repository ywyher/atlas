FROM lukemathwalker/cargo-chef:latest-rust-slim AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin atlas

# We do not need the Rust toolchain to run the binary!
FROM debian:trixie-slim AS runtime

# for reqwest to work
RUN apt-get update && apt-get install -y --no-install-recommends \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/atlas /usr/local/bin/app
ENTRYPOINT ["/usr/local/bin/app"]