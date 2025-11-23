FROM ubuntu:24.04

ARG RUST_VERSION=nightly-2025-11-20

ARG USERNAME=ubuntu

RUN apt update
RUN apt upgrade -y

RUN DEBIAN_FRONTEND=noninteractive apt install -y build-essential qemu-kvm curl lld git dosfstools mtools

ARG GRP_KVM=993

RUN groupadd --gid $GRP_KVM kvm && usermod -aG kvm $USERNAME

USER $USERNAME

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain $RUST_VERSION --no-modify-path

ENV PATH=/home/${USERNAME}/.cargo/bin:${PATH}
RUN rustup component add rust-src --toolchain ${RUST_VERSION}-x86_64-unknown-linux-gnu
