# Static musl binary on scratch (amd64)
FROM rust:1.95-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src src
RUN cargo build --release --locked

FROM scratch
COPY --from=build /src/target/release/pebble-index01-to-webdav /pebble-index01-to-webdav
USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/pebble-index01-to-webdav"]
