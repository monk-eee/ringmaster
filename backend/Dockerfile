FROM docker.io/library/rust:1-slim AS build
WORKDIR /app
# ADR-0078: build.rs shells out to `git` at compile time for build
# provenance; rust:1-slim does not include it by default.
RUN apt-get update \
    && apt-get install -y --no-install-recommends git \
    && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release --manifest-path backend/Cargo.toml

FROM docker.io/library/debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/ringmaster-backend /usr/local/bin/ringmaster-backend
ENV RUST_LOG=info
CMD ["/usr/local/bin/ringmaster-backend"]
