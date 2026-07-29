FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

RUN apt-get update \
    && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends \
        build-essential \
        clang \
        pkg-config \
        python3-venv \
    && rm -rf /var/lib/apt/lists/*
