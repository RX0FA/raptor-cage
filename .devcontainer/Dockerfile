ARG VARIANT
FROM mcr.microsoft.com/devcontainers/rust:${VARIANT}
RUN apt-get update &&\
    export DEBIAN_FRONTEND=noninteractive &&\
    apt-get install -y cloc &&\
    apt-get clean &&\
    rm -rf /var/lib/apt/lists/*
RUN rm /etc/sudoers.d/vscode
RUN su vscode -c 'cargo install cargo-audit cargo-edit'
