FROM ubuntu:20.04

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN apt update && DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC apt install -y gcc gcc-multilib cmake pkg-config libssl-dev curl mysql-client-8.0
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal
RUN cargo install cargo-deb
