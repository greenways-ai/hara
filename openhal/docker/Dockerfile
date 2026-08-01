ARG NGINX_VERSION=1.27.5

FROM rust:1.78-bookworm AS builder
ARG NGINX_VERSION
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential ca-certificates curl libpcre2-dev libssl-dev zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN cargo build --manifest-path openhal/runtime/Cargo.toml --release
RUN curl -fsSLO "https://nginx.org/download/nginx-${NGINX_VERSION}.tar.gz" \
    && tar -xzf "nginx-${NGINX_VERSION}.tar.gz" \
    && cd "nginx-${NGINX_VERSION}" \
    && OPENHAL_RUNTIME_DIR=/src/openhal/runtime/target/release \
       ./configure --with-compat --add-dynamic-module=/src/openhal/nginx \
    && make -j"$(nproc)" modules

FROM nginx:${NGINX_VERSION}-bookworm
ARG NGINX_VERSION
COPY --from=builder \
    /src/nginx-${NGINX_VERSION}/objs/ngx_http_openhal_module.so \
    /usr/lib/nginx/modules/ngx_http_openhal_module.so
COPY --from=builder \
    /src/openhal/runtime/target/release/libopenhal_runtime.so \
    /usr/local/lib/libopenhal_runtime.so
COPY openhal/docker/nginx.conf /etc/nginx/nginx.conf
COPY openhal/examples/app.hal /etc/openhal/app.hal
RUN ldconfig
EXPOSE 8080
