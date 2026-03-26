# ──────────────────────────────────────────────────────────────
#  ETAPA 1 — Builder
# ──────────────────────────────────────────────────────────────
FROM rust:1.88-slim-bookworm AS builder
WORKDIR /build

ENV CARGO_NET_RETRY=10
ENV CARGO_HTTP_TIMEOUT=120
ENV CARGO_HTTP_CHECK_REVOKE=false

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && update-ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./ Cargo.lock* ./

# Truco de caché: compilar dependencias con main vacío primero
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Compilar el código real
COPY src ./src
RUN touch src/main.rs
RUN cargo build --release

# ──────────────────────────────────────────────────────────────
#  ETAPA 2 — Runtime (imagen mínima, sin Rust)
# ──────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    bash \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/simulador /app/simulador

CMD ["/bin/bash", "-c", "/app/simulador; echo '✅ Simulador finalizado. Contenedor en espera.'; tail -f /dev/null"]
