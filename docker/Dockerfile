FROM rust:bullseye AS builder_backend

WORKDIR /app

COPY . .

RUN cargo build --release --bin backend


FROM rust:bullseye AS builder_frontend

RUN ap-get update \
    && apt-get install -y libc6 libc6-dev \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall trunk wasm-bindgen-cli

WORKDIR /app

COPY . .

WORKDIR /app/src/frontend

RUN trunk build --release


FROM debian:bullseye-slim

RUN apt-get update \
    && apt-get install -y openssl ca-certificates curl jq \
    && rm -rf /var/lib/apt/lists/*

RUN update-ca-certificates

WORKDIR /app

COPY --from=builder_backend /app/target/release/backend /usr/local/bin
COPY --from=builder_frontend /app/src/frontend/dist /app/static

CMD ["/usr/local/bin/backend"]
