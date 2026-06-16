FROM rust:slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY migrations/ migrations/
COPY .cargo/ .cargo/
RUN cargo build

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/debug/bibliotecario /usr/local/bin/bibliotecario
COPY assets/ /app/assets/
ENV ASSETS_DIR=/app/assets/images
EXPOSE 3050
CMD ["bibliotecario"]
