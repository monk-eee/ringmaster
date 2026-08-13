FROM docker.io/library/rust:1-slim AS build
WORKDIR /app
COPY . .
RUN cargo build --release --manifest-path backend/Cargo.toml

FROM docker.io/library/debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/ringmaster-backend /usr/local/bin/ringmaster-backend
ENV RUST_LOG=info
CMD ["/usr/local/bin/ringmaster-backend"]
