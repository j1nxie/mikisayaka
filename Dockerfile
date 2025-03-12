FROM rust:1.85.0-slim-bookworm AS base
RUN apt update && apt install -y build-essential pkg-config libssl-dev git

FROM base AS deps
WORKDIR /app
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY build.rs build.rs
RUN mkdir src; echo 'fn main() {}' > src/main.rs
RUN cargo install --locked --path .
RUN rm -rf src;

FROM deps AS build
COPY .git .git
COPY src src
RUN touch src/main.rs
RUN cargo build --release

FROM rust:1.85.0-slim-bookworm AS run
RUN apt update && apt install -y libssl3 ca-certificates
WORKDIR /app
COPY --from=build /app/target/release/mikisayaka .
CMD [ "/app/mikisayaka" ]
