FROM node:22-alpine AS frontend-builder

WORKDIR /app/web-ui/admin-ui
COPY web-ui/admin-ui/package.json ./
RUN npm install -g pnpm && pnpm install --ignore-scripts
COPY web-ui/admin-ui ./
RUN pnpm build

WORKDIR /app/web-ui/user-ui
COPY web-ui/user-ui/package.json ./
RUN npm install
COPY web-ui/user-ui ./
RUN npm run build

FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY assets ./assets
COPY --from=frontend-builder /app/web-ui/admin-ui/dist /app/web-ui/admin-ui/dist
COPY --from=frontend-builder /app/web-ui/user-ui/dist /app/web-ui/user-ui/dist

RUN cargo build --release

FROM alpine:3.21

RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=builder /app/target/release/kiro2cc-proxy /app/kiro2cc-proxy

EXPOSE 5678

CMD sh -c 'mkdir -p /app/config && \
  if [ ! -f /app/config/config.json ]; then \
    echo "{\"host\":\"${HOST:-0.0.0.0}\",\"port\":${PORT:-5678},\"adminPsw\":\"${ADMIN_PSW:-$ADMIN_API_KEY}\"}" > /app/config/config.json; \
  fi && \
  if [ ! -f /app/config/credentials.json ]; then \
    echo "[]" > /app/config/credentials.json; \
  fi && \
  ./kiro2cc-proxy --config /app/config/config.json --credentials /app/config/credentials.json'
