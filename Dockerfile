# Pin the builder to the same Debian release as the distroless runtime
# (bookworm / Debian 12) so the shared libraries copied across stages share
# the runtime's glibc ABI and never skew against it.
FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libpq-dev build-essential clang && rm -rf /var/lib/apt/lists/*

ENV CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0

WORKDIR /build
COPY . .
RUN cargo build --release && strip target/release/open-ontologies

# Collect every shared library the binary transitively needs (direct plus
# recursive NEEDED deps) into /deps, preserving absolute paths. This replaces a
# hand-maintained COPY list that was easy to leave incomplete (libz.so.1, pulled
# in via libpq, was missing and broke `serve` at runtime, issue #59). The glibc
# family, the loader, and libgcc/libstdc++ are excluded because the distroless
# cc-debian12 base already provides them; we only ship the libs it lacks.
RUN mkdir -p /deps && \
    ldd target/release/open-ontologies \
      | awk '/=> \// {print $3} /^\t\// {print $1}' \
      | grep -Ev '/(ld-linux[^/]*|libc|libm|libdl|libpthread|librt|libresolv|libgcc_s|libstdc\+\+)\.so' \
      | sort -u \
      | xargs -I{} cp -v --parents -L {} /deps/

FROM gcr.io/distroless/cc-debian12

LABEL io.modelcontextprotocol.server.name="io.github.fabio-rovai/open-ontologies"

COPY --from=builder /build/target/release/open-ontologies /usr/local/bin/open-ontologies
COPY --from=builder /deps/ /

ENTRYPOINT ["open-ontologies"]
CMD ["serve"]
