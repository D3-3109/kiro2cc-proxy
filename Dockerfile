FROM node:22-alpine AS frontend-builder

RUN npm install -g pnpm@9

WORKDIR /app/web-ui/admin-ui
COPY web-ui/admin-ui/package.json ./
RUN pnpm install --ignore-scripts
COPY web-ui/admin-ui ./
RUN pnpm build

WORKDIR /app/web-ui/user-ui
COPY web-ui/user-ui/package.json ./
RUN npm install
COPY web-ui/user-ui ./
RUN npm run build

FROM rust:1-alpine AS builder

# reqwest's default native-tls feature links OpenSSL statically on musl.
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY assets ./assets
COPY --from=frontend-builder /app/web-ui/admin-ui/dist /app/web-ui/admin-ui/dist
COPY --from=frontend-builder /app/web-ui/user-ui/dist /app/web-ui/user-ui/dist

RUN cargo build --release --locked

FROM alpine:3.21

# tini runs as PID 1 so `docker stop` (SIGTERM) reaches the proxy instead of
# being ignored by a signal-less PID 1 until the 10s SIGKILL timeout.
RUN apk add --no-cache ca-certificates tini

WORKDIR /app
COPY --from=builder /app/target/release/kiro2cc-proxy /app/kiro2cc-proxy
COPY --chmod=755 docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh

EXPOSE 5678
VOLUME ["/app/config"]

ENTRYPOINT ["/sbin/tini", "--", "docker-entrypoint.sh"]
