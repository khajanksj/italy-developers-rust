FROM rust:1.82-bookworm AS builder
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
COPY templates ./templates
COPY migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl libssl3 && rm -rf /var/lib/apt/lists/* && useradd --system --uid 10001 --no-create-home app
WORKDIR /app
COPY --from=builder /app/target/release/italy-developers-rust /usr/local/bin/app
COPY static ./static
USER app
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 CMD ["curl","--fail","--silent","http://127.0.0.1:8080/health/live"]
ENTRYPOINT ["/usr/local/bin/app"]
