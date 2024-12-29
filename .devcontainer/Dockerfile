FROM mcr.microsoft.com/devcontainers/rust:1-1-bullseye

# Install binstall
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# Install cross
RUN cargo binstall -y cross

VOLUME /usr/local/cargo