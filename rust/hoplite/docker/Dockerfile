ARG NGINX_VERSION=1.30.4

FROM rust:1.78-bookworm AS builder
ARG NGINX_VERSION
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential ca-certificates curl libpcre2-dev libssl-dev zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN cargo build --manifest-path rust/hoplite/runtime/Cargo.toml --release
RUN curl -fsSLO "https://nginx.org/download/nginx-${NGINX_VERSION}.tar.gz" \
    && tar -xzf "nginx-${NGINX_VERSION}.tar.gz" \
    && cd "nginx-${NGINX_VERSION}" \
    && HOPLITE_RUNTIME_DIR=/src/rust/target/release \
       ./configure --prefix=/opt/hoplite/nginx \
         --with-http_ssl_module --with-threads \
         --add-module=/src/rust/hoplite/nginx \
    && make -j"$(nproc)" \
    && make install

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libpcre2-8-0 libssl3 zlib1g \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /opt/hoplite/nginx /opt/hoplite/nginx
COPY rust/hoplite/docker/nginx.conf /opt/hoplite/nginx/conf/nginx.conf
COPY rust/hoplite/examples/app.hal /etc/hoplite/app.hal
EXPOSE 8080
ENTRYPOINT ["/opt/hoplite/nginx/sbin/nginx"]
CMD ["-g", "daemon off;"]
