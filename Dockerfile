FROM rust:1.81.0-alpine3.20 AS build

RUN apk add g++ pkgconfig openssl-dev alpine-sdk
ENV OPENSSL_LIB_DIR=/usr/lib
ENV OPENSSL_INCLUDE_DIR=/usr/include

WORKDIR /app
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY build.rs build.rs
RUN mkdir src; echo 'fn main() {}' > src/main.rs
RUN cargo install --locked --path .
RUN rm -rf src;
COPY src src
RUN touch src/main.rs
RUN cargo build --release

FROM alpine:3.20 AS run
WORKDIR /app
COPY --from=build /app/target/release/mikisayaka .
CMD [ "/app/mikisayaka" ]
