FROM rust:1.83.0-slim-bookworm
RUN useradd -m -d /builder -s /bin/bash builder
USER builder
WORKDIR /builder
COPY --chown=builder:builder Cargo.toml Cargo.lock ./
RUN mkdir src &&\
    echo "fn main() {}" > src/main.rs &&\
    cargo fetch &&\
    cargo build --release
COPY --chown=builder:builder src src/
RUN touch src/main.rs &&\
    cargo test &&\
    cargo build --release
