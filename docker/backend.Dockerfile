FROM rust:1.88-slim AS builder

RUN apt-get update && apt-get install -y \
  pkg-config libssl-dev && \
  rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm src/main.rs

COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs
RUN cargo build --release

# Imagen final
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
  ca-certificates libssl3 && \
  rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/pceo-backend .
COPY --from=builder /app/migrations ./migrations

# Usuario sin privilegios
RUN useradd -r -s /bin/false appuser
USER appuser

EXPOSE 3001
CMD ["./horarioshub-api"]
