FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this is cached separately 
RUN apt-get update && apt-get install lld clang -y
RUN rustup toolchain install nightly
RUN cargo +nightly chef cook --release --recipe-path recipe.json

# Build crates
COPY . .
RUN cargo +nightly build --release

FROM debian:bullseye-slim as voyager

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    apt-get clean
WORKDIR /app
COPY --from=builder /app/target/release/voyager /usr/local/bin
ENTRYPOINT ["/usr/local/bin/voyager"]

FROM debian:bullseye-slim as europa

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    apt-get clean
WORKDIR /app
COPY --from=builder /app/target/release/europa /usr/local/bin
ENTRYPOINT ["/usr/local/bin/europa"]
