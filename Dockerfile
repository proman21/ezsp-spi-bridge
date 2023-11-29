FROM --platform=linux/amd64 ghcr.io/cross-rs/armv7-unknown-linux-musleabihf:latest

RUN apt update && apt install -y netcat && rm -rf /var/lib/apt/lists/*