FROM rust as builder
COPY . /app
WORKDIR /app
RUN cargo build --release

FROM archlinux/archlinux:latest as base

RUN pacman-db-upgrade
RUN pacman -Syy
RUN pacman -Su base-devel git libgit2 --noconfirm

RUN mkdir /app
RUN mkdir /app/config
RUN mkdir /app/data

WORKDIR /app

RUN useradd -m -s /bin/bash app
RUN echo -e "app  ALL=(ALL) NOPASSWD:/usr/sbin/pacman\napp  ALL=(ALL) NOPASSWD:/usr/sbin/pacman-key" | sudo tee /etc/sudoers.d/app

FROM base as local

RUN pacman -S --noconfirm rustup
USER 1000
RUN rustup default stable

WORKDIR /app

FROM base

COPY --from=builder /app/target/release/aur-build-server /app/aur-build-server
COPY start.sh /app/start.sh

RUN chmod +x /app/start.sh
RUN chown 1000:1000 -R /app

USER 1000
ENTRYPOINT ["/app/start.sh"]