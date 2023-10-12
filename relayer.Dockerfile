# ------------------ Chef stage -------------------
# Use cargo chef to cache dependencies
FROM rustlang/rust:nightly AS chef

# Install cargo chef
RUN cargo install cargo-chef 

# Work in app
WORKDIR /app

# ------------------ Planner stage -------------------
FROM chef as planner
# Copy files into container
COPY . .

# Create a lockfile for cargo chef
RUN cargo +nightly chef prepare --recipe-path recipe.json

# ------------------ Builder stage -------------------
FROM chef AS builder
ARG RELAYER_BRANCH_NAME=production

# Clone the relayer repository at the latest commit and set it as the working directory
RUN git clone --branch ${RELAYER_BRANCH_NAME} --single-branch https://github.com/zkemail/relayer /relayer
WORKDIR /relayer

# Copy over our lock file
COPY --from=planner  /app/recipe.json /relayer/recipe.json

# Build for any AWS machine. Same as cargo build but caches dependencies with the chef to make builds faster.
RUN cargo chef cook --target x86_64-unknown-linux-gnu --recipe-path recipe.json
RUN cp /relayer/target/x86_64-unknown-linux-gnu/debug/relayer /relayer/target/debug/
RUN cargo chef cook --target x86_64-unknown-linux-gnu --release --recipe-path recipe.json
RUN cp /relayer/target/x86_64-unknown-linux-gnu/release/relayer /relayer/target/release/

### Above this all dependencies should be cached as long as our lock file stays the same

# Build binary
RUN cargo build --target x86_64-unknown-linux-gnu --release

# ------------------ Runtime stage -------------------

# Using super lightweight debian image to reduce overhead
FROM debian:bullseye-slim AS runtime

# Copy prebuild bin from the Builder stage
COPY --from=builder /relayer/target/release/relayer /usr/local/bin/relayer
COPY --from=builder /relayer/target/release/relayer /relayer/target/release/relayer
COPY --from=builder /relayer/abi /relayer/abi
COPY --from=builder /relayer/Cargo.toml /relayer/Cargo.toml
COPY --from=builder /relayer/Cargo.lock /relayer/Cargo.lock
COPY --from=builder /relayer/proofs /relayer/proofs
COPY --from=builder /relayer/received_eml /relayer/received_eml

# This cargo chef logic comes from https://github.com/LukeMathWalker/cargo-chef
# Inspired by Huff: https://github.com/huff-language/huff-rs/blob/main/Dockerfile