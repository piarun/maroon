FROM rust:1.90-bookworm AS builder
WORKDIR /app

RUN apt-get update \
  && apt-get install -y --no-install-recommends protobuf-compiler \
  && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release -p maroon

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/maroon /usr/local/bin/maroon

ENV RUST_LOG=info

ENTRYPOINT ["/usr/local/bin/maroon"]
