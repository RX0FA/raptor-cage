ARG VARIANT
FROM mcr.microsoft.com/devcontainers/rust:${VARIANT}
RUN rm /etc/sudoers.d/vscode
RUN su vscode -c 'cargo install cargo-audit'
