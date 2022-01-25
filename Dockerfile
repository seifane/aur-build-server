FROM rust as builder
COPY . /app
WORKDIR /app
RUN cargo build --release

FROM archlinux:latest

RUN pacman -Syy
RUN pacman -Su base-devel git libgit2 --noconfirm

RUN mkdir /app
RUN mkdir /app/config
RUN mkdir /app/data

WORKDIR /app

RUN useradd -m -s /bin/bash app
RUN echo "app  ALL=(ALL) NOPASSWD:ALL" | sudo tee /etc/sudoers.d/app

COPY --from=builder /app/target/release/aur-build-server /app/aur-build-server
COPY start.sh /app/start.sh

RUN chmod +x /app/start.sh
RUN chown 1000:1000 -R /app

USER 1000
ENTRYPOINT "/app/start.sh"