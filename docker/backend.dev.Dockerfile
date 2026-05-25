FROM rust:latest

RUN apt-get update && apt-get install -y \
  pkg-config \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-watch --locked

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

# Crear ambos ficheros que declara el Cargo.toml
RUN mkdir src \
  && echo "fn main() {}" > src/main.rs \
  && echo "" > src/lib.rs

RUN cargo build

# Remove compiled binaries so the real source is compiled on startup,
# but keep dependency artifacts cached to speed up that first build.
RUN rm -f target/debug/horarioshub-api \
  target/debug/deps/horarioshub_api-* \
  target/debug/deps/horarioshub-api-* \
  && find target/debug/.fingerprint -name "horarioshub*" -exec rm -rf {} + 2>/dev/null || true

RUN rm src/main.rs src/lib.rs

EXPOSE 3001

CMD ["cargo-watch", "-x", "run", "-w", "src", "-w", "migrations"]
