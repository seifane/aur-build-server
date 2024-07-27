FROM rust AS builder
COPY . /app
WORKDIR /app
RUN cargo build --release

FROM archlinux/archlinux:base-devel AS base

RUN pacman-db-upgrade
RUN pacman -Syy

RUN mkdir /app

WORKDIR /app

RUN useradd -m -s /bin/bash app

FROM base AS server

COPY --from=builder /app/target/release/aur-build-server /app/aur-build-server
COPY docker/start-server.sh /app/start-server.sh

RUN chown 1000:1000 -R /app

USER 1000

RUN chmod +x /app/start-server.sh

ENTRYPOINT ["/app/start-server.sh"]

FROM base AS worker

RUN pacman -Su base-devel git libgit2 openssl-1.1 bubblewrap fakechroot --noconfirm

RUN echo -e "[multilib]\nInclude = /etc/pacman.d/mirrorlist" | sudo tee -a /etc/pacman.conf

COPY --from=builder /app/target/release/aur-build-worker /app/aur-build-worker
COPY docker/start-worker.sh /app/start-worker.sh

RUN chown 1000:1000 -R /app

USER 1000

RUN chmod +x /app/start-worker.sh

ENTRYPOINT ["/app/start-worker.sh"]