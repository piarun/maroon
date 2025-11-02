FROM rust:1.90-bookworm AS builder
WORKDIR /app

RUN apt-get update \
  && apt-get install -y --no-install-recommends protobuf-compiler \
  && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release -p gateway

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/gateway /usr/local/bin/gateway

ENV RUST_LOG=info
EXPOSE 5000

ENTRYPOINT ["/usr/local/bin/gateway"]

