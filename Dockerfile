FROM rust as builder
COPY . /app
WORKDIR /app
RUN cargo build --release

FROM archlinux/archlinux:latest as base

RUN pacman-db-upgrade
RUN pacman -Syy
RUN pacman -Su base-devel git libgit2 openssl-1.1 --noconfirm

RUN mkdir /app
RUN mkdir /app/config
RUN mkdir /app/data

WORKDIR /app

RUN useradd -m -s /bin/bash app
RUN echo -e "app  ALL=(ALL) NOPASSWD:/usr/sbin/pacman\napp  ALL=(ALL) NOPASSWD:/usr/sbin/pacman-key" | sudo tee /etc/sudoers.d/app

FROM base as server

COPY --from=builder /app/target/release/aur-build-server /app/aur-build-server
COPY docker/start-server.sh /app/start-server.sh
RUN chown 1000:1000 -R /app
USER 1000
RUN mkdir /app/serve
RUN mkdir /app/logs
RUN chmod +x /app/start-server.sh

ENTRYPOINT ["/app/start-server.sh"]

FROM base as worker

COPY --from=builder /app/target/release/aur-build-worker /app/aur-build-worker
COPY docker/start-worker.sh /app/start-worker.sh
RUN chown 1000:1000 -R /app
USER 1000
RUN mkdir /app/worker_logs
RUN chmod +x /app/start-worker.sh

ENTRYPOINT ["/app/start-worker.sh"]