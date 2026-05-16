FROM rust:latest

RUN apt-get update && apt-get install -y \
  pkg-config \
  libssl-dev \
  libssl3 \
  curl \
  && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-watch --locked

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

# Crear ambos ficheros que declara el Cargo.toml
RUN mkdir src \
  && echo "fn main() {}" > src/main.rs \
  && echo "" > src/lib.rs

RUN cargo build

RUN rm src/main.rs src/lib.rs

EXPOSE 3001

CMD ["cargo-watch", "-x", "run", "-w", "src", "-w", "migrations"]
