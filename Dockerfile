# Limina: the API and the web app in one image, for a host behind a proxy such as Cloudflare.
#
#   docker build -t limina .
#   docker run --rm -p 3000:3000 -e NVIDIA_API_KEY=... -e TRUST_PROXY=true limina
#
# Without NVIDIA_API_KEY the deterministic checks, the laboratory and the library work;
# only reading a text, writing a law and reading an image are off.

# 1. The web app (Angular).
FROM node:22-bookworm-slim AS web
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web/ ./
RUN npm run build

# 2. The API (Rust, release build). The laws' data files are embedded at compile time.
FROM rust:1-bookworm AS api
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY data ./data
RUN cargo build --release --locked -p limina-api

# 3. The runtime image: one binary, the built web app, no build tools.
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home limina
COPY --from=api /src/target/release/limina-api /usr/local/bin/limina-api
COPY --from=web /web/dist/web/browser /app/static
ENV PORT=3000 \
    STATIC_DIR=/app/static \
    LLM_RATE_LIMIT_PER_HOUR=15 \
    TRUST_PROXY=false
USER limina
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["bash", "-c", "exec 3<>/dev/tcp/127.0.0.1/${PORT} && printf 'GET /healthz HTTP/1.0\\r\\n\\r\\n' >&3 && head -n1 <&3 | grep -q ' 200 '"]
CMD ["limina-api"]
