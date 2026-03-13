FROM rust:1-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libpq-dev build-essential clang && rm -rf /var/lib/apt/lists/*

ENV CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0

WORKDIR /build
COPY . .
RUN cargo build --release && strip target/release/open-ontologies

FROM debian:bookworm-slim

LABEL io.modelcontextprotocol.server.name="io.github.fabio-rovai/open-ontologies"

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/open-ontologies /usr/local/bin/open-ontologies

RUN open-ontologies init

ENTRYPOINT ["open-ontologies"]
CMD ["serve"]
