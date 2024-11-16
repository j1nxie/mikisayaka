FROM rust:1.81.0-slim-bookworm AS build
RUN apt update && apt install -y build-essential pkg-config libssl-dev

WORKDIR /app
COPY .git .git
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY build.rs build.rs
RUN mkdir src; echo 'fn main() {}' > src/main.rs
RUN cargo install --locked --path .
RUN rm -rf src;
COPY src src
RUN touch src/main.rs
RUN cargo build --release

FROM rust:1.81.0-slim-bookworm AS run
RUN apt update && apt install -y libssl3 ca-certificates
WORKDIR /app
COPY --from=build /app/target/release/mikisayaka .
CMD [ "/app/mikisayaka" ]
